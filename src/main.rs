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

fn main() {
    println!("=======================================================");
    println!(" SleepSafe v0.1.0 - Thermal & Closed-Lid Sleep Guard  ");
    println!("=======================================================");

    let config = Config::load();
    println!("Config loaded:");
    println!(" - Thermal ceiling: {:.1}°C", config.temp_ceiling_celsius);
    println!(" - Thermal delta:   +{:.1}°C", config.temp_delta_celsius);
    println!(" - Max timeout:     {}s", config.max_timeout_secs);
    println!(" - Check interval:  {}s", config.check_interval_secs);
    println!(" - Ignore external monitor: {}", config.ignore_if_external_monitor);

    let provider = DefaultProvider::new();
    let mut engine = WatchdogEngine::new(config.clone());

    println!("\nStarting SleepSafe background loop. Press Ctrl+C to exit.\n");

    loop {
        let power = provider.get_power_source();
        let lid = provider.get_lid_state();
        let ext_display = provider.has_external_display();
        let temp = provider.get_cpu_temp().unwrap_or(40.0);

        let action = engine.tick(power, lid, ext_display, temp);

        match action {
            WatchdogAction::StartMonitoring { baseline_temp } => {
                println!(
                    "[SleepSafe] Charger disconnected with lid CLOSED! Baseline temp: {:.1}°C. Monitoring active...",
                    baseline_temp
                );
            }
            WatchdogAction::CancelMonitoring { reason } => {
                println!("[SleepSafe] Monitoring canceled: {}", reason);
            }
            WatchdogAction::TriggerSuspend(reason) => {
                eprintln!("[SleepSafe] CRITICAL ACTION: {}", reason);
                if let Err(e) = provider.suspend_system() {
                    eprintln!("[SleepSafe] Failed to suspend system: {}", e);
                }
            }
            WatchdogAction::None => {
                if engine.is_monitoring {
                    println!(
                        "[SleepSafe] Monitoring... Temp: {:.1}°C (baseline: {:.1}°C, +{:.1}°C), Elapsed: {}s/{}s",
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
