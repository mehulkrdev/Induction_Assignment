use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{PathBuf};
use chrono::Local;
use once_cell::sync::OnceCell;

pub static LOG_DIR: OnceCell<PathBuf> = OnceCell::new();
pub static LOG_FILE: OnceCell<std::fs::File> = OnceCell::new();

pub fn init_logger(component_name: &str) -> io::Result<()> {
    LOG_DIR.get_or_try_init(|| {
        let assignment_root = std::env::current_dir()?;
        let logs_root = assignment_root.join("logs");

        fs::create_dir_all(&logs_root)?;

        let current_time = Local::now().format("%d-%m-%Y_%H:%M").to_string();
        let log_run_dir = logs_root.join(current_time);
        fs::create_dir_all(&log_run_dir)?;
        
        let log_file_path = log_run_dir.join(format!("{}.log", component_name));
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open(&log_file_path)?;

        LOG_FILE.set(file).map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to set log file"))?;
        Ok(log_run_dir)
    }).map(|_| ()) // Return an empty Ok once initialized
}

pub fn cleanup_log_dir() {
    if let Some(path) = LOG_DIR.get() {
        let _ = fs::remove_dir_all(path);
    }
}

#[macro_export]
macro_rules! log_entry {
    ($($arg:tt)*) => ({
        use chrono::Local;
        use std::io::Write;
        let msg = format!($($arg)*);
        let file = file!();
        let line = line!();
        let log_line = format!("{}[{}]: \"{}\"\n", file, line, msg);
        
        if let Some(mut file) = $crate::logger::LOG_FILE.get() {
            let _ = file.write_all(log_line.as_bytes());
        } else {
            // Fallback to stderr if logger not initialized
            eprint!("Logger not initialized: {}", log_line);
        }
    });
}

pub use log_entry;

pub fn set_log_file_for_tests(file: std::fs::File) {
    let _ = LOG_FILE.set(file);
}
