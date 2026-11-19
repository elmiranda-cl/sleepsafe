use super::SystemProvider;
use crate::log_info;
use crate::state::{LidState, PowerSource};
use std::sync::atomic::{AtomicU8, Ordering};

use windows::core::w;
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, RegisterPowerSettingNotification, SetSuspendState,
    POWERBROADCAST_SETTING, SYSTEM_POWER_STATUS,
};
use windows::Win32::System::SystemServices::{
    GUID_ACDC_POWER_SOURCE, GUID_LIDSWITCH_STATE_CHANGE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetSystemMetrics,
    DEVICE_NOTIFY_WINDOW_HANDLE, PBT_POWERSETTINGCHANGE, SM_CMONITORS, WM_POWERBROADCAST,
    WNDCLASSW,
};

// Global atomics updated by native Win32 power broadcast events:
// Lid: 0 = Closed, 1 = Open, 2 = Unknown
static GLOBAL_LID_STATE: AtomicU8 = AtomicU8::new(1);
// Power: 0 = Battery, 1 = Ac, 2 = Unknown
static GLOBAL_POWER_STATE: AtomicU8 = AtomicU8::new(1);

pub struct WindowsProvider;

impl WindowsProvider {
    pub fn new() -> Self {
        // 1. Initial snapshot of power status
        unsafe {
            let mut status = SYSTEM_POWER_STATUS::default();
            if GetSystemPowerStatus(&mut status).is_ok() {
                match status.ACLineStatus {
                    0 => GLOBAL_POWER_STATE.store(0, Ordering::SeqCst),
                    1 => GLOBAL_POWER_STATE.store(1, Ordering::SeqCst),
                    _ => GLOBAL_POWER_STATE.store(2, Ordering::SeqCst),
                }
            }
        }

        // 2. Spawn native Win32 message-loop thread to listen for hardware power events
        std::thread::spawn(|| {
            unsafe {
                let class_name = w!("SleepSafe_PowerWatcher");
                let wnd_class = WNDCLASSW {
                    lpfnWndProc: Some(power_wnd_proc),
                    lpszClassName: class_name,
                    ..Default::default()
                };

                let _ = windows::Win32::UI::WindowsAndMessaging::RegisterClassW(&wnd_class);

                let hwnd = CreateWindowExW(
                    Default::default(),
                    class_name,
                    w!("SleepSafeHidden"),
                    Default::default(),
                    0,
                    0,
                    0,
                    0,
                    None,
                    None,
                    None,
                    None,
                );

                if let Ok(hwnd) = hwnd {
                    // Register for hardware lid switch notifications
                    let _ = RegisterPowerSettingNotification(
                        HANDLE(hwnd.0 as _),
                        &GUID_LIDSWITCH_STATE_CHANGE,
                        DEVICE_NOTIFY_WINDOW_HANDLE,
                    );

                    // Register for AC/DC power source change notifications
                    let _ = RegisterPowerSettingNotification(
                        HANDLE(hwnd.0 as _),
                        &GUID_ACDC_POWER_SOURCE,
                        DEVICE_NOTIFY_WINDOW_HANDLE,
                    );

                    // Run the Win32 message pump
                    let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = DispatchMessageW(&msg);
                    }
                }
            }
        });

        Self
    }
}

unsafe extern "system" fn power_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_POWERBROADCAST && wparam.0 == PBT_POWERSETTINGCHANGE as usize {
        let setting_ptr = lparam.0 as *const POWERBROADCAST_SETTING;
        if !setting_ptr.is_null() {
            let setting = &*setting_ptr;
            if setting.PowerSetting == GUID_LIDSWITCH_STATE_CHANGE {
                let data = setting.Data[0];
                GLOBAL_LID_STATE.store(data, Ordering::SeqCst);
                log_info!(
                    "Win32 Power Event: Lid state changed to {}",
                    if data == 0 { "CLOSED" } else { "OPEN" }
                );
            } else if setting.PowerSetting == GUID_ACDC_POWER_SOURCE {
                let data = setting.Data[0];
                GLOBAL_POWER_STATE.store(data, Ordering::SeqCst);
                log_info!(
                    "Win32 Power Event: Power source changed to {}",
                    if data == 1 { "AC" } else { "BATTERY" }
                );
            }
        }
        return LRESULT(1);
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

impl SystemProvider for WindowsProvider {
    fn get_power_source(&self) -> PowerSource {
        match GLOBAL_POWER_STATE.load(Ordering::SeqCst) {
            0 => PowerSource::Battery,
            1 => PowerSource::Ac,
            _ => PowerSource::Unknown,
        }
    }

    fn get_lid_state(&self) -> LidState {
        match GLOBAL_LID_STATE.load(Ordering::SeqCst) {
            0 => LidState::Closed,
            1 => LidState::Open,
            _ => LidState::Unknown,
        }
    }

    fn has_external_display(&self) -> bool {
        unsafe {
            let monitors = GetSystemMetrics(SM_CMONITORS);
            monitors > 1
        }
    }

    fn get_cpu_temp(&self) -> Option<f32> {
        // Query ACPI Thermal Zone directly via lightweight WMI query with no window flash
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            use std::process::Command;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            let output = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "try { $t = (Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature -ErrorAction Stop).CurrentTemperature; if ($t -gt 0) { [math]::Round(($t - 2732) / 10, 1) } else { 0 } } catch { 0 }",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output();

            if let Ok(out) = output {
                let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if let Ok(temp) = text.parse::<f32>() {
                    if temp > 10.0 && temp < 120.0 {
                        return Some(temp);
                    }
                }
            }
        }

        // Safe fallback baseline if motherboard driver does not expose MSAcpi thermal zone
        Some(40.0)
    }

    fn suspend_system(&self) -> Result<(), String> {
        unsafe {
            // bhibernate: false (sleep), bforce: false, bwakeupevents: false (keep wake events enabled)
            let res = SetSuspendState(false, false, false);
            if res.as_bool() {
                Ok(())
            } else {
                Err("SetSuspendState failed".to_string())
            }
        }
    }
}
