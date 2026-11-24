use super::SystemProvider;
use crate::state::{LidState, PowerSource};
use std::fs;

pub struct MockProvider;

impl MockProvider {
    pub fn new() -> Self {
        Self
    }
}

impl SystemProvider for MockProvider {
    fn get_power_source(&self) -> PowerSource {
        if let Ok(content) = fs::read_to_string("/sys/class/power_supply/AC/online") {
            if content.trim() == "1" {
                return PowerSource::Ac;
            } else {
                return PowerSource::Battery;
            }
        }
        PowerSource::Ac
    }

    fn get_lid_state(&self) -> LidState {
        if let Ok(entries) = fs::read_dir("/proc/acpi/button/lid") {
            for entry in entries.flatten() {
                let state_file = entry.path().join("state");
                if let Ok(content) = fs::read_to_string(state_file) {
                    if content.contains("closed") {
                        return LidState::Closed;
                    } else if content.contains("open") {
                        return LidState::Open;
                    }
                }
            }
        }
        LidState::Open
    }

    fn has_external_display(&self) -> bool {
        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("eDP") {
                    continue;
                }
                let status_path = entry.path().join("status");
                if let Ok(status) = fs::read_to_string(status_path) {
                    if status.trim() == "connected" {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn get_cpu_temp(&self) -> Option<f32> {
        if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
            for entry in entries.flatten() {
                let path = entry.path();
                let type_path = path.join("type");
                let temp_path = path.join("temp");
                if let Ok(t) = fs::read_to_string(type_path) {
                    if t.contains("x86_pkg_temp") || t.contains("TCPU") || t.contains("acpitz") {
                        if let Ok(raw) = fs::read_to_string(temp_path) {
                            if let Ok(val) = raw.trim().parse::<f32>() {
                                return Some(val / 1000.0);
                            }
                        }
                    }
                }
            }
        }
        Some(40.0)
    }

    fn suspend_system(&self) -> Result<(), String> {
        println!("[MOCK/LINUX] Suspend system requested!");
        Ok(())
    }

    fn update_status(&self, _temp: Option<f32>, _is_monitoring: bool) {}
}
