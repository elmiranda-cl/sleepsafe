use crate::config::Config;
use crate::state::{LidState, PowerSource, SuspendReason, WatchdogAction};

pub struct WatchdogEngine {
    pub config: Config,
    pub is_monitoring: bool,
    pub baseline_temp: f32,
    pub prev_temp: f32,
    pub rise_count: u32,
    pub elapsed_secs: u64,
    pub is_paused: bool,
}

impl WatchdogEngine {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            is_monitoring: false,
            baseline_temp: 40.0,
            prev_temp: 40.0,
            rise_count: 0,
            elapsed_secs: 0,
            is_paused: false,
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.is_paused = paused;
        if paused {
            self.is_monitoring = false;
        }
    }

    pub fn tick(
        &mut self,
        power: PowerSource,
        lid: LidState,
        has_external_monitor: bool,
        current_temp: f32,
    ) -> WatchdogAction {
        if self.is_paused {
            return WatchdogAction::None;
        }

        // If AC is plugged in, we do not need to monitor or suspend
        if power == PowerSource::Ac {
            if self.is_monitoring {
                self.is_monitoring = false;
                return WatchdogAction::CancelMonitoring {
                    reason: "AC power connected",
                };
            }
            return WatchdogAction::None;
        }

        // If the lid is open, user is actively using the laptop
        if lid == LidState::Open {
            if self.is_monitoring {
                self.is_monitoring = false;
                return WatchdogAction::CancelMonitoring {
                    reason: "Lid opened by user",
                };
            }
            return WatchdogAction::None;
        }

        // At this point: Power is Battery AND Lid is Closed
        if !self.is_monitoring {
            self.is_monitoring = true;
            self.baseline_temp = current_temp;
            self.prev_temp = current_temp;
            self.rise_count = 0;
            self.elapsed_secs = 0;
            return WatchdogAction::StartMonitoring {
                baseline_temp: current_temp,
            };
        }

        // Monitoring is active: advance timer
        self.elapsed_secs += self.config.check_interval_secs;

        // Track temperature direction
        if current_temp > self.prev_temp {
            self.rise_count += 1;
        } else if current_temp < self.prev_temp && self.rise_count > 0 {
            self.rise_count -= 1;
        }
        self.prev_temp = current_temp;

        // If external monitor is connected, laptop is on a desk
        if has_external_monitor && self.config.ignore_if_external_monitor {
            if self.elapsed_secs >= self.config.max_timeout_secs {
                self.is_monitoring = false;
                return WatchdogAction::TriggerSuspend(SuspendReason::TimeoutReached {
                    timeout_secs: self.config.max_timeout_secs,
                });
            }
            return WatchdogAction::None;
        }

        // CHECK 1: Absolute thermal ceiling (e.g. >= 58°C)
        if current_temp >= self.config.temp_ceiling_celsius {
            self.is_monitoring = false;
            return WatchdogAction::TriggerSuspend(SuspendReason::ThermalCeilingExceeded {
                current_temp,
                ceiling: self.config.temp_ceiling_celsius,
            });
        }

        // CHECK 2: Steady rise (+delta above baseline with at least 3 continuous rises)
        let delta = current_temp - self.baseline_temp;
        if delta >= self.config.temp_delta_celsius && self.rise_count >= 3 {
            self.is_monitoring = false;
            return WatchdogAction::TriggerSuspend(SuspendReason::SteadyThermalRise {
                baseline_temp: self.baseline_temp,
                current_temp,
                delta,
            });
        }

        // CHECK 3: Max timeout (e.g. 10 minutes) during power outage on desk
        if self.elapsed_secs >= self.config.max_timeout_secs {
            self.is_monitoring = false;
            return WatchdogAction::TriggerSuspend(SuspendReason::TimeoutReached {
                timeout_secs: self.config.max_timeout_secs,
            });
        }

        WatchdogAction::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_ac_operation() {
        let mut engine = WatchdogEngine::new(Config::default());
        let action = engine.tick(PowerSource::Ac, LidState::Closed, false, 40.0);
        assert_eq!(action, WatchdogAction::None);
    }

    #[test]
    fn test_lid_open_on_battery() {
        let mut engine = WatchdogEngine::new(Config::default());
        let action = engine.tick(PowerSource::Battery, LidState::Open, false, 42.0);
        assert_eq!(action, WatchdogAction::None);
    }

    #[test]
    fn test_start_monitoring_on_unplug_with_closed_lid() {
        let mut engine = WatchdogEngine::new(Config::default());
        let action = engine.tick(PowerSource::Battery, LidState::Closed, false, 40.0);
        assert_eq!(
            action,
            WatchdogAction::StartMonitoring {
                baseline_temp: 40.0
            }
        );
        assert!(engine.is_monitoring);
    }

    #[test]
    fn test_backpack_thermal_runaway_triggers_suspend() {
        let mut engine = WatchdogEngine::new(Config::default());
        // 1. Initial unplug
        let _ = engine.tick(PowerSource::Battery, LidState::Closed, false, 40.0);

        // 2. Temp starts climbing: 40 -> 42 -> 44 -> 47
        let _ = engine.tick(PowerSource::Battery, LidState::Closed, false, 42.0);
        let _ = engine.tick(PowerSource::Battery, LidState::Closed, false, 44.0);
        let action = engine.tick(PowerSource::Battery, LidState::Closed, false, 47.0);

        match action {
            WatchdogAction::TriggerSuspend(SuspendReason::SteadyThermalRise { delta, .. }) => {
                assert!(delta >= 6.0);
            }
            other => panic!("Expected SteadyThermalRise, got {:?}", other),
        }
    }

    #[test]
    fn test_thermal_ceiling_immediate_suspend() {
        let mut engine = WatchdogEngine::new(Config::default());
        let _ = engine.tick(PowerSource::Battery, LidState::Closed, false, 50.0);
        let action = engine.tick(PowerSource::Battery, LidState::Closed, false, 60.0);

        match action {
            WatchdogAction::TriggerSuspend(SuspendReason::ThermalCeilingExceeded {
                current_temp,
                ceiling,
            }) => {
                assert_eq!(current_temp, 60.0);
                assert_eq!(ceiling, 58.0);
            }
            other => panic!("Expected ThermalCeilingExceeded, got {:?}", other),
        }
    }

    #[test]
    fn test_ac_restore_cancels_monitoring() {
        let mut engine = WatchdogEngine::new(Config::default());
        let _ = engine.tick(PowerSource::Battery, LidState::Closed, false, 40.0);
        assert!(engine.is_monitoring);

        let action = engine.tick(PowerSource::Ac, LidState::Closed, false, 40.5);
        assert_eq!(
            action,
            WatchdogAction::CancelMonitoring {
                reason: "AC power connected"
            }
        );
        assert!(!engine.is_monitoring);
    }

    #[test]
    fn test_external_monitor_bypasses_thermal_panic() {
        let mut engine = WatchdogEngine::new(Config::default());
        let _ = engine.tick(PowerSource::Battery, LidState::Closed, true, 40.0);
        // Thermal rises, but external monitor is connected
        let action = engine.tick(PowerSource::Battery, LidState::Closed, true, 55.0);
        assert_eq!(action, WatchdogAction::None);
    }
}
