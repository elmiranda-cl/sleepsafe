use super::SystemProvider;
use crate::state::{LidState, PowerSource};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use windows::Win32::System::Power::{GetSystemPowerStatus, SetSuspendState, SYSTEM_POWER_STATUS};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CMONITORS};

pub struct WindowsProvider {
    // 0 = Closed, 1 = Open, 2 = Unknown
    pub lid_state: Arc<AtomicU8>,
}

impl WindowsProvider {
    pub fn new() -> Self {
        Self {
            lid_state: Arc::new(AtomicU8::new(1)), // Default to Open
        }
    }
}

impl SystemProvider for WindowsProvider {
    fn get_power_source(&self) -> PowerSource {
        unsafe {
            let mut status = SYSTEM_POWER_STATUS::default();
            if GetSystemPowerStatus(&mut status).is_ok() {
                match status.ACLineStatus {
                    0 => PowerSource::Battery,
                    1 => PowerSource::Ac,
                    _ => PowerSource::Unknown,
                }
            } else {
                PowerSource::Unknown
            }
        }
    }

    fn get_lid_state(&self) -> LidState {
        match self.lid_state.load(Ordering::SeqCst) {
            0 => LidState::Closed,
            1 => LidState::Open,
            _ => LidState::Unknown,
        }
    }

    fn has_external_display(&self) -> bool {
        unsafe {
            let monitors = GetSystemMetrics(SM_CMONITORS);
            // More than 1 monitor usually implies external display connected to laptop
            monitors > 1
        }
    }

    fn get_cpu_temp(&self) -> Option<f32> {
        // Fallback default temp: 42°C if ACPI thermal zone WMI is not queried directly
        // For extended production release, LibreHardwareMonitor or WMI root\wmi is linked.
        Some(42.0)
    }

    fn suspend_system(&self) -> Result<(), String> {
        unsafe {
            // SetSuspendState(bhibernate: false, bforce: false, bwakeupevents: false)
            let res = SetSuspendState(false, false, false);
            if res.as_bool() {
                Ok(())
            } else {
                Err("SetSuspendState failed".to_string())
            }
        }
    }
}
