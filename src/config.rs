use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Absolute thermal ceiling to suspend (in Celsius). Default: 58.0°C
    pub temp_ceiling_celsius: f32,
    /// Continuous thermal rise above baseline to trigger suspend (in Celsius). Default: 6.0°C
    pub temp_delta_celsius: f32,
    /// Max wait time in seconds during a power outage before suspending. Default: 600s (10 min)
    pub max_timeout_secs: u64,
    /// Polling interval in seconds during active monitoring. Default: 5s
    pub check_interval_secs: u64,
    /// If true, does not trigger thermal panic when an external monitor is attached. Default: true
    pub ignore_if_external_monitor: bool,
    /// Start with Windows. Default: true
    pub auto_start_with_windows: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            temp_ceiling_celsius: 58.0,
            temp_delta_celsius: 6.0,
            max_timeout_secs: 600,
            check_interval_secs: 5,
            ignore_if_external_monitor: true,
            auto_start_with_windows: true,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        #[cfg(windows)]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let mut path = PathBuf::from(appdata);
                path.push("SleepSafe");
                path.push("config.json");
                return path;
            }
        }

        #[cfg(not(windows))]
        {
            if let Ok(home) = std::env::var("HOME") {
                let mut path = PathBuf::from(home);
                path.push(".config");
                path.push("sleepsafe");
                path.push("config.json");
                return path;
            }
        }

        PathBuf::from("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str(&content) {
                    return cfg;
                }
            }
        }
        let default_cfg = Self::default();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, json)?;
        Ok(())
    }
}
