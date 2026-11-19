use super::SystemProvider;
use crate::log_info;
use crate::state::{LidState, PowerSource};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU8, Ordering};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, RegisterPowerSettingNotification, SetSuspendState,
    POWERBROADCAST_SETTING, SYSTEM_POWER_STATUS,
};
use windows::Win32::System::SystemServices::{
    GUID_ACDC_POWER_SOURCE, GUID_LIDSWITCH_STATE_CHANGE,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, ShellExecuteW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetMessageW, GetSystemMetrics, LoadIconW, MessageBoxW, PostMessageW,
    RegisterWindowMessageW, SetForegroundWindow, TrackPopupMenu, DEVICE_NOTIFY_WINDOW_HANDLE,
    IDI_APPLICATION, IDI_SHIELD, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST,
    MF_GRAYED, MF_SEPARATOR, MF_STRING, PBT_POWERSETTINGCHANGE, SM_CMONITORS, SW_SHOW,
    TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_DESTROY, WM_ENDSESSION,
    WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_POWERBROADCAST, WM_RBUTTONUP, WNDCLASSW,
};

// Global atomics updated by native Win32 power broadcast events:
// Lid: 0 = Closed, 1 = Open, 2 = Unknown
static GLOBAL_LID_STATE: AtomicU8 = AtomicU8::new(1);
// Power: 0 = Battery, 1 = Ac, 2 = Unknown
static GLOBAL_POWER_STATE: AtomicU8 = AtomicU8::new(1);
// Last known CPU temperature (as f32 bits)
static GLOBAL_LAST_TEMP: AtomicU32 = AtomicU32::new(0);
// Watchdog monitoring active state
static GLOBAL_MONITORING: AtomicBool = AtomicBool::new(false);
// User paused state
static GLOBAL_PAUSED: AtomicBool = AtomicBool::new(false);
// Main window handle for tray management
static GLOBAL_HWND: AtomicIsize = AtomicIsize::new(0);
// Taskbar restart message ID
static TASKBAR_CREATED_MSG: AtomicU32 = AtomicU32::new(0);
// Guard against multiple simultaneous status dialogs
static DIALOG_ACTIVE: AtomicBool = AtomicBool::new(false);

const WM_TRAYICON: u32 = WM_APP + 1;

// Menu action command IDs
const IDM_HEADER: usize = 0;
const IDM_STATUS: usize = 1001;
const IDM_VIEW_LOG: usize = 1002;
const IDM_OPEN_FOLDER: usize = 1003;
const IDM_EDIT_CONFIG: usize = 1004;
const IDM_TOGGLE_PAUSE: usize = 1005;
const IDM_EXIT: usize = 1006;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn get_current_tooltip() -> String {
    let is_paused = GLOBAL_PAUSED.load(Ordering::Relaxed);
    let is_monitoring = GLOBAL_MONITORING.load(Ordering::Relaxed);
    let pwr = GLOBAL_POWER_STATE.load(Ordering::Relaxed);
    let lid = GLOBAL_LID_STATE.load(Ordering::Relaxed);
    let temp_bits = GLOBAL_LAST_TEMP.load(Ordering::Relaxed);
    let temp = if temp_bits != 0 {
        f32::from_bits(temp_bits)
    } else {
        40.0
    };

    if is_paused {
        "SleepSafe [PAUSADO]\r\nProtección suspendida temporalmente".to_string()
    } else if is_monitoring {
        format!(
            "SleepSafe - ¡VIGILANDO!\r\nBatería | Tapa cerrada | {:.1}°C",
            temp
        )
    } else {
        let pwr_str = match pwr {
            1 => "Conectado a CA",
            0 => "Batería",
            _ => "Energía: ?",
        };
        let lid_str = match lid {
            1 => "Tapa abierta",
            0 => "Tapa cerrada",
            _ => "Tapa: ?",
        };
        format!(
            "SleepSafe - Protección Activa\r\n{} | {} | {:.1}°C",
            pwr_str, lid_str, temp
        )
    }
}

fn add_tray_icon(hwnd: HWND) {
    let icon = unsafe { LoadIconW(None, IDI_SHIELD).or_else(|_| LoadIconW(None, IDI_APPLICATION)) };
    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    if let Ok(icon) = icon {
        nid.hIcon = icon;
    }
    let tip_text = get_current_tooltip();
    let mut tip = [0u16; 128];
    for (i, c) in tip_text.encode_utf16().take(127).enumerate() {
        tip[i] = c;
    }
    nid.szTip = tip;
    unsafe {
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

fn update_tray_tooltip_dynamic() {
    let hwnd_raw = GLOBAL_HWND.load(Ordering::SeqCst);
    if hwnd_raw != 0 {
        let hwnd = HWND(hwnd_raw as _);
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_TIP;
        let tip_text = get_current_tooltip();
        let mut tip = [0u16; 128];
        for (i, c) in tip_text.encode_utf16().take(127).enumerate() {
            tip[i] = c;
        }
        nid.szTip = tip;
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }
}

fn remove_tray_icon() {
    let hwnd_raw = GLOBAL_HWND.swap(0, Ordering::SeqCst);
    if hwnd_raw != 0 {
        let hwnd = HWND(hwnd_raw as _);
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        }
    }
}

fn show_status_dialog() {
    if DIALOG_ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(move || {
        let is_paused = GLOBAL_PAUSED.load(Ordering::Relaxed);
        let is_monitoring = GLOBAL_MONITORING.load(Ordering::Relaxed);
        let pwr = GLOBAL_POWER_STATE.load(Ordering::Relaxed);
        let lid = GLOBAL_LID_STATE.load(Ordering::Relaxed);
        let temp_bits = GLOBAL_LAST_TEMP.load(Ordering::Relaxed);
        let temp = if temp_bits != 0 {
            f32::from_bits(temp_bits)
        } else {
            40.0
        };
        let ext_display = unsafe { GetSystemMetrics(SM_CMONITORS) > 1 };

        let cfg = crate::config::Config::load();

        let pwr_str = match pwr {
            1 => "Conectado a CA (Red eléctrica)",
            0 => "Batería (Modo móvil)",
            _ => "Desconocido",
        };
        let lid_str = match lid {
            1 => "Abierta (Usuario activo)",
            0 => "Cerrada (Portátil cerrado)",
            _ => "Desconocido",
        };
        let disp_str = if ext_display {
            "Detectada (Escritorio / Dock)"
        } else {
            "No detectada (Pantalla interna únicamente)"
        };
        let mode_str = if is_paused {
            "PAUSADO (Protección desactivada temporalmente por el usuario)"
        } else if is_monitoring {
            "¡VIGILANCIA ACTIVA! (Midiendo pendiente térmica con tapa cerrada)"
        } else {
            "En reposo / Protegido (El equipo se encuentra seguro)"
        };

        let body = format!(
            "=========================================\r\n\
             🛡️ SleepSafe v0.1.0-alpha\r\n\
             Protección contra Backpack Oven Syndrome\r\n\
             =========================================\r\n\r\n\
             ESTADO ACTUAL DE LOS SENSORES:\r\n\
             • Fuente de energía: {}\r\n\
             • Tapa del portátil: {}\r\n\
             • Pantalla externa: {}\r\n\
             • Temperatura CPU: {:.1} °C\r\n\r\n\
             ESTADO DE LA PROTECCIÓN:\r\n\
             • Modo: {}\r\n\
             • Límite crítico de temperatura: {:.1} °C\r\n\
             • Alza máxima permitida (Delta): +{:.1} °C\r\n\
             • Tiempo de gracia máximo: {} segundos\r\n\r\n\
             UBICACIONES DE ARCHIVOS:\r\n\
             • Registro (Log): {}\r\n\
             • Configuración: {}\r\n\r\n\
             Consejo: Haz clic derecho en el icono de escudo junto al reloj para abrir logs o configurar umbrales.",
            pwr_str,
            lid_str,
            disp_str,
            temp,
            mode_str,
            cfg.temp_ceiling_celsius,
            cfg.temp_delta_celsius,
            cfg.max_timeout_secs,
            crate::logger::log_path().display(),
            crate::config::Config::config_path().display()
        );

        let title_w = to_wide("SleepSafe - Estado del Sistema");
        let body_w = to_wide(&body);

        unsafe {
            let _ = MessageBoxW(
                None,
                PCWSTR(body_w.as_ptr()),
                PCWSTR(title_w.as_ptr()),
                MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND | MB_TOPMOST,
            );
        }

        DIALOG_ACTIVE.store(false, Ordering::SeqCst);
    });
}

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

        // 2. Spawn native Win32 message-loop thread for hardware power events and System Tray
        std::thread::spawn(|| unsafe {
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
                GLOBAL_HWND.store(hwnd.0 as isize, Ordering::SeqCst);

                // Register for TaskbarCreated message in case Explorer restarts
                let taskbar_msg = RegisterWindowMessageW(w!("TaskbarCreated"));
                TASKBAR_CREATED_MSG.store(taskbar_msg, Ordering::SeqCst);

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

                // Add the native System Tray icon next to the clock
                add_tray_icon(hwnd);

                // Run the Win32 message pump
                let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = DispatchMessageW(&msg);
                }

                remove_tray_icon();
            }
        });

        Self
    }
}

impl Drop for WindowsProvider {
    fn drop(&mut self) {
        remove_tray_icon();
    }
}

unsafe extern "system" fn power_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // 1. Taskbar created (Explorer restarted)
    if msg != 0 && msg == TASKBAR_CREATED_MSG.load(Ordering::Relaxed) {
        add_tray_icon(hwnd);
        return LRESULT(0);
    }

    // 2. Tray Icon interaction
    if msg == WM_TRAYICON {
        let event = lparam.0 as u32;

        if event == WM_RBUTTONUP {
            let _ = SetForegroundWindow(hwnd);

            if let Ok(hmenu) = CreatePopupMenu() {
                let is_paused = GLOBAL_PAUSED.load(Ordering::Relaxed);
                let is_monitoring = GLOBAL_MONITORING.load(Ordering::Relaxed);
                let pwr = GLOBAL_POWER_STATE.load(Ordering::Relaxed);
                let lid = GLOBAL_LID_STATE.load(Ordering::Relaxed);
                let temp_bits = GLOBAL_LAST_TEMP.load(Ordering::Relaxed);
                let temp = if temp_bits != 0 {
                    f32::from_bits(temp_bits)
                } else {
                    40.0
                };

                // Header
                let title_str = format!(
                    "SleepSafe v0.1.0-alpha ({})",
                    if is_paused {
                        "Pausado"
                    } else if is_monitoring {
                        "Vigilando"
                    } else {
                        "Activo"
                    }
                );
                let title_w = to_wide(&title_str);
                let _ = AppendMenuW(
                    hmenu,
                    MF_GRAYED | MF_STRING,
                    IDM_HEADER,
                    PCWSTR(title_w.as_ptr()),
                );
                let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);

                // Live status telemetry in menu
                let pwr_str = format!(
                    "Alimentación: {}",
                    match pwr {
                        1 => "Conectado a CA",
                        0 => "Batería",
                        _ => "Desconocido",
                    }
                );
                let pwr_w = to_wide(&pwr_str);
                let _ = AppendMenuW(hmenu, MF_GRAYED | MF_STRING, 0, PCWSTR(pwr_w.as_ptr()));

                let lid_str = format!(
                    "Tapa: {}",
                    match lid {
                        1 => "Abierta",
                        0 => "Cerrada",
                        _ => "Desconocido",
                    }
                );
                let lid_w = to_wide(&lid_str);
                let _ = AppendMenuW(hmenu, MF_GRAYED | MF_STRING, 0, PCWSTR(lid_w.as_ptr()));

                let temp_str = format!("Temperatura CPU: {:.1} °C", temp);
                let temp_w = to_wide(&temp_str);
                let _ = AppendMenuW(hmenu, MF_GRAYED | MF_STRING, 0, PCWSTR(temp_w.as_ptr()));

                let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);

                // Actions
                let status_label = to_wide("📊 Ver Estado Detallado...");
                let _ = AppendMenuW(hmenu, MF_STRING, IDM_STATUS, PCWSTR(status_label.as_ptr()));

                let log_label = to_wide("📜 Ver Registro en Vivo (Log)...");
                let _ = AppendMenuW(hmenu, MF_STRING, IDM_VIEW_LOG, PCWSTR(log_label.as_ptr()));

                let folder_label = to_wide("📁 Abrir Carpeta de Configuración...");
                let _ = AppendMenuW(hmenu, MF_STRING, IDM_OPEN_FOLDER, PCWSTR(folder_label.as_ptr()));

                let config_label = to_wide("⚙️ Editar Configuración (config.json)...");
                let _ = AppendMenuW(hmenu, MF_STRING, IDM_EDIT_CONFIG, PCWSTR(config_label.as_ptr()));

                let pause_label = to_wide(if is_paused {
                    "▶️ Reanudar Protección"
                } else {
                    "⏸️ Pausar Protección"
                });
                let _ = AppendMenuW(
                    hmenu,
                    MF_STRING,
                    IDM_TOGGLE_PAUSE,
                    PCWSTR(pause_label.as_ptr()),
                );

                let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);

                let exit_label = to_wide("❌ Salir de SleepSafe");
                let _ = AppendMenuW(hmenu, MF_STRING, IDM_EXIT, PCWSTR(exit_label.as_ptr()));

                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                let cmd = TrackPopupMenu(
                    hmenu,
                    TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
                    pt.x,
                    pt.y,
                    0,
                    hwnd,
                    None,
                );
                let _ = DestroyMenu(hmenu);
                let _ = PostMessageW(hwnd, 0, WPARAM(0), LPARAM(0));

                match cmd.0 as usize {
                    IDM_STATUS => {
                        show_status_dialog();
                    }
                    IDM_VIEW_LOG => {
                        let path = crate::logger::log_path();
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&path);
                        let path_w = to_wide(&path.to_string_lossy());
                        let _ = ShellExecuteW(
                            None,
                            w!("open"),
                            PCWSTR(path_w.as_ptr()),
                            None,
                            None,
                            SW_SHOW,
                        );
                    }
                    IDM_OPEN_FOLDER => {
                        let path = crate::config::Config::config_path();
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                            let dir_w = to_wide(&parent.to_string_lossy());
                            let _ = ShellExecuteW(
                                None,
                                w!("open"),
                                PCWSTR(dir_w.as_ptr()),
                                None,
                                None,
                                SW_SHOW,
                            );
                        }
                    }
                    IDM_EDIT_CONFIG => {
                        let path = crate::config::Config::config_path();
                        if !path.exists() {
                            let _ = crate::config::Config::default().save();
                        }
                        let path_w = to_wide(&path.to_string_lossy());
                        let _ = ShellExecuteW(
                            None,
                            w!("open"),
                            PCWSTR(path_w.as_ptr()),
                            None,
                            None,
                            SW_SHOW,
                        );
                    }
                    IDM_TOGGLE_PAUSE => {
                        let current = GLOBAL_PAUSED.load(Ordering::Relaxed);
                        let next = !current;
                        GLOBAL_PAUSED.store(next, Ordering::Relaxed);
                        log_info!(
                            "SleepSafe protection {}",
                            if next {
                                "PAUSED by user"
                            } else {
                                "RESUMED by user"
                            }
                        );
                        update_tray_tooltip_dynamic();
                    }
                    IDM_EXIT => {
                        remove_tray_icon();
                        log_info!("SleepSafe terminated cleanly by user via System Tray.");
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
            return LRESULT(0);
        } else if event == WM_LBUTTONDBLCLK || event == WM_LBUTTONUP {
            show_status_dialog();
            return LRESULT(0);
        }
    }

    // 3. Hardware Power Events
    if msg == WM_POWERBROADCAST && wparam.0 == PBT_POWERSETTINGCHANGE as usize {
        let setting_ptr = lparam.0 as *const POWERBROADCAST_SETTING;
        if !setting_ptr.is_null() {
            let setting = &*setting_ptr;
            let val = *(setting.Data.as_ptr() as *const u32);

            if setting.PowerSetting == GUID_LIDSWITCH_STATE_CHANGE {
                // 0 = Closed, 1 = Open (DWORD)
                let state = if val == 0 { 0 } else { 1 };
                GLOBAL_LID_STATE.store(state, Ordering::SeqCst);
                log_info!(
                    "Win32 Power Event: Lid state changed to {}",
                    if state == 0 { "CLOSED" } else { "OPEN" }
                );
                update_tray_tooltip_dynamic();
            } else if setting.PowerSetting == GUID_ACDC_POWER_SOURCE {
                // Win32 SYSTEM_POWER_CONDITION enum:
                // PoAc = 0 (AC connected)
                // PoDc = 1 (DC / Battery active)
                // PoHot = 2 (Hot-swap battery)
                let state = match val {
                    0 => 1, // 1 = Ac in our internal state
                    _ => 0, // 0 = Battery in our internal state
                };
                GLOBAL_POWER_STATE.store(state, Ordering::SeqCst);
                log_info!(
                    "Win32 Power Event: Power source changed to {}",
                    if state == 1 { "AC" } else { "BATTERY" }
                );
                update_tray_tooltip_dynamic();
            }
        }
        return LRESULT(1);
    }

    // 4. Clean exit on destroy / session end
    if msg == WM_DESTROY || msg == WM_ENDSESSION {
        remove_tray_icon();
        return LRESULT(0);
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

        Some(40.0)
    }

    fn suspend_system(&self) -> Result<(), String> {
        unsafe {
            let res = SetSuspendState(false, false, false);
            if res.as_bool() {
                Ok(())
            } else {
                Err("SetSuspendState failed".to_string())
            }
        }
    }

    fn is_paused(&self) -> bool {
        GLOBAL_PAUSED.load(Ordering::Relaxed)
    }

    fn update_status(&self, temp: f32, is_monitoring: bool) {
        GLOBAL_LAST_TEMP.store(temp.to_bits(), Ordering::Relaxed);
        GLOBAL_MONITORING.store(is_monitoring, Ordering::Relaxed);
        update_tray_tooltip_dynamic();
    }
}
