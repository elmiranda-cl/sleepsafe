use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

static LOG_MUTEX: Mutex<()> = Mutex::new(());

pub fn log_path() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            let mut path = PathBuf::from(appdata);
            path.push("SleepSafe");
            path.push("sleepsafe.log");
            return path;
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".config");
            path.push("sleepsafe");
            path.push("sleepsafe.log");
            return path;
        }
    }

    PathBuf::from("sleepsafe.log")
}

pub fn log(level: &str, message: &str) {
    let _guard = LOG_MUTEX.lock().unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let line = format!("[{}] [{}] {}\n", now, level, message);

    // Also print to stderr if interactive
    eprint!("{}", line);

    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Cap log size at 1 MB to prevent unbounded disk usage
    if let Ok(metadata) = fs::metadata(&path) {
        if metadata.len() > 1024 * 1024 {
            let _ = fs::remove_file(&path);
        }
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::log("INFO", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::log("WARN", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_alert {
    ($($arg:tt)*) => {
        $crate::logger::log("ALERT", &format!($($arg)*))
    };
}
