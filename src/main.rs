#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod config;
pub mod platform;
pub mod state;
pub mod watchdog;

use config::Config;
use platform::{DefaultProvider, SystemProvider};
use state::WatchdogAction;
use std::thread;
use std::time::Duration;
use watchdog::WatchdogEngine;

#[cfg(windows)]
fn acquire_single_instance_lock() -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;

    unsafe {
        let mutex = CreateMutexW(None, true, w!("Local\\SleepSafe_SingleInstance_Mutex"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return false;
        }
        std::mem::forget(mutex);
        true
    }
}

#[cfg(not(windows))]
fn acquire_single_instance_lock() -> bool {
    true
}

fn main() {
    if !acquire_single_instance_lock() {
        eprintln!("[SleepSafe] Another instance of SleepSafe is already running.");
        return;
    }

    let config = Config::load();
    let provider = DefaultProvider::new();
    let mut engine = WatchdogEngine::new(config.clone());

    loop {
        let power = provider.get_power_source();
        let lid = provider.get_lid_state();
        let ext_display = provider.has_external_display();
        let temp = provider.get_cpu_temp().unwrap_or(40.0);

        let action = engine.tick(power, lid, ext_display, temp);

        match action {
            WatchdogAction::StartMonitoring { .. } => {}
            WatchdogAction::CancelMonitoring { .. } => {}
            WatchdogAction::TriggerSuspend(reason) => {
                eprintln!("[SleepSafe] Suspending system: {}", reason);
                let _ = provider.suspend_system();
            }
            WatchdogAction::None => {}
        }

        thread::sleep(Duration::from_secs(config.check_interval_secs));
    }
}
