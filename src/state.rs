#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSource {
    Ac,
    Battery,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LidState {
    Open,
    Closed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SuspendReason {
    ThermalCeilingExceeded { current_temp: f32, ceiling: f32 },
    SteadyThermalRise { baseline_temp: f32, current_temp: f32, delta: f32 },
    TimeoutReached { timeout_secs: u64 },
}

impl std::fmt::Display for SuspendReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SuspendReason::ThermalCeilingExceeded { current_temp, ceiling } => {
                write!(f, "Thermal ceiling reached: {:.1}°C (>= {:.1}°C). Suspending to prevent backpack overheating.", current_temp, ceiling)
            }
            SuspendReason::SteadyThermalRise { baseline_temp, current_temp, delta } => {
                write!(f, "Continuous thermal rise detected: {:.1}°C -> {:.1}°C (+{:.1}°C). Backpack entrapment suspected. Suspending immediately.", baseline_temp, current_temp, delta)
            }
            SuspendReason::TimeoutReached { timeout_secs } => {
                write!(f, "Grace timeout of {}s reached with closed lid on battery. Suspending to save battery.", timeout_secs)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WatchdogAction {
    None,
    StartMonitoring { baseline_temp: f32 },
    CancelMonitoring { reason: &'static str },
    TriggerSuspend(SuspendReason),
}
