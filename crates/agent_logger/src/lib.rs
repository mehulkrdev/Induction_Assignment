use chrono::Local;
use once_cell::sync::OnceCell;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;

use std::sync::Mutex;

pub static LOG_DIR: OnceCell<PathBuf> = OnceCell::new();
pub static LOG_FILE: OnceCell<Mutex<std::fs::File>> = OnceCell::new();

fn get_log_root_dir() -> PathBuf {
    std::env::var("AGENT_LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                PathBuf::from("C:/Assignment/Logs")
            } else {
                PathBuf::from("/var/log/enrollment-agent")
            }
        })
}

pub fn init_logger(component_name: &str) -> std::io::Result<()> {
    LOG_DIR
        .get_or_try_init(|| {
            let logs_root = get_log_root_dir();

            fs::create_dir_all(&logs_root)?;

            let current_time = Local::now().format("%d-%m-%Y_%H:%M").to_string();
            let log_run_dir = logs_root.join(current_time);
            fs::create_dir_all(&log_run_dir)?;

            let log_file_path = log_run_dir.join(format!("{}.log", component_name));
            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&log_file_path)?;

            LOG_FILE
                .set(Mutex::new(file))
                .map_err(|_| std::io::Error::other("Failed to set log file"))?;
            Ok(log_run_dir)
        })
        .map(|_| ())
}

pub fn cleanup_log_dir() {
    if let Some(path) = LOG_DIR.get() {
        let _ = fs::remove_dir_all(path);
    }
}

#[macro_export]
macro_rules! log_entry {
    ($($arg:tt)*) => ({
        use std::io::Write;
        let msg = format!($($arg)*);
        let file = file!();
        let line = line!();
        let log_line = format!("{}[{}]: \"{}\"\n", file, line, msg);

        if let Some(mutex) = $crate::LOG_FILE.get() {
            if let Ok(mut file) = mutex.lock() {
                let _ = file.write_all(log_line.as_bytes());
                let _ = file.sync_all(); // Ensure persistence across potential crashes
            }
        } else {
            // Fallback to stderr if logger not initialized
            eprint!("Logger not initialized: {}", log_line);
        }
    });
}

pub fn set_log_file_for_tests(file: std::fs::File) {
    let _ = LOG_FILE.set(Mutex::new(file));
}
