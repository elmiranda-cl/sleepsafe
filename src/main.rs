#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod config;
pub mod logger;
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
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

    unsafe {
        let mutex = CreateMutexW(None, true, w!("Local\\SleepSafe_SingleInstance_Mutex"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = MessageBoxW(
                None,
                w!("SleepSafe ya se encuentra en ejecución en segundo plano.\n\nPuedes ver su estado o configurar opciones haciendo clic en el icono de escudo junto al reloj (bandeja del sistema)."),
                w!("SleepSafe ya está activo"),
                MB_OK | MB_ICONINFORMATION,
            );
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
        log_warn!("Another instance of SleepSafe is already running. Exiting cleanly.");
        return;
    }

    log_info!("SleepSafe started. Initializing configuration and hardware providers.");

    let config = Config::load();
    let provider = DefaultProvider::new();
    let mut engine = WatchdogEngine::new(config.clone());

    #[cfg(windows)]
    crate::platform::windows::sync_autostart_registry(config.auto_start_with_windows);

    log_info!(
        "Active Configuration: Ceiling: {:.1}°C, Delta: +{:.1}°C, MaxTimeout: {}s, PollingInterval: {}s",
        config.temp_ceiling_celsius,
        config.temp_delta_celsius,
        config.max_timeout_secs,
        config.check_interval_secs
    );

    loop {
        // Sync pause state from user interaction (e.g. System Tray menu)
        engine.set_paused(provider.is_paused());

        let power = provider.get_power_source();
        let lid = provider.get_lid_state();
        let ext_display = provider.has_external_display();
        let temp = provider.get_cpu_temp().unwrap_or(40.0);

        let action = engine.tick(power, lid, ext_display, temp);

        // Update System Tray icon status and tooltip
        provider.update_status(temp, engine.is_monitoring);

        match action {
            WatchdogAction::StartMonitoring { baseline_temp } => {
                log_alert!(
                    "Charger disconnected with lid CLOSED. Starting thermal slope watchdog. Baseline: {:.1}°C",
                    baseline_temp
                );
            }
            WatchdogAction::CancelMonitoring { reason } => {
                log_info!("Watchdog monitoring canceled: {}", reason);
            }
            WatchdogAction::TriggerSuspend(reason) => {
                log_alert!("CRITICAL ACTION TRIGGERED: {}. Suspending system NOW.", reason);
                if let Err(e) = provider.suspend_system() {
                    log_warn!("Failed to suspend system: {}", e);
                }
            }
            WatchdogAction::None => {
                if engine.is_monitoring {
                    log_info!(
                        "Watchdog tick: Temp={:.1}°C (baseline={:.1}°C, delta=+{:.1}°C), Elapsed={}s/{}s",
                        temp,
                        engine.baseline_temp,
                        temp - engine.baseline_temp,
                        engine.elapsed_secs,
                        config.max_timeout_secs
                    );
                }
            }
        }

        thread::sleep(Duration::from_secs(config.check_interval_secs));
    }
}
