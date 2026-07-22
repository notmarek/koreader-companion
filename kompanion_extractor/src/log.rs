use std::fs::File;
use std::io::Write;
use std::sync::Mutex;

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

pub fn init(path: &str) {
    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        *LOG_FILE.lock().unwrap() = Some(file);
    }
}

pub fn log_message(msg: &str) {
    eprintln!("{}", msg);
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *guard {
            let _ = writeln!(file, "{}", msg);
            let _ = file.flush();
        }
    }
}

#[macro_export]
macro_rules! extractor_log {
    ($($arg:tt)*) => {
        $crate::log::log_message(&format!($($arg)*));
    };
}
