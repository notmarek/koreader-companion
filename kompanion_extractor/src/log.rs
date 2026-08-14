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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    // LOG_FILE is a process-global Mutex, so all assertions about which messages
    // land in which file are only reliable when run sequentially in one test.
    #[test]
    fn test_log() {
        // 1. Before init: LOG_FILE is None; log_message must not panic.
        log_message("before-init message");

        // 2. A bad path must be silently ignored — not a panic.
        init("/nonexistent/dir/kompanion.log");
        log_message("after-bad-init message");

        // 3. Good init: subsequent messages are written and flushed to the file.
        let path = std::env::temp_dir()
            .join(format!("kompanion_extractor_log_{}.log", std::process::id()));
        let path_str = path.to_str().unwrap();
        init(path_str);

        log_message("first message");
        log_message("second message");

        let mut contents = String::new();
        std::fs::File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();

        assert!(contents.contains("first message"), "first message written to file");
        assert!(contents.contains("second message"), "second message written to file");
        // Messages sent before init must not appear in the file.
        assert!(!contents.contains("before-init"), "pre-init message must NOT be in file");
        assert!(!contents.contains("after-bad-init"), "bad-path message must NOT be in file");
        assert_eq!(contents.lines().count(), 2, "exactly two lines flushed");

        std::fs::remove_file(&path).ok();
    }
}

#[macro_export]
macro_rules! extractor_log {
    ($($arg:tt)*) => {
        $crate::log::log_message(&format!($($arg)*));
    };
}
