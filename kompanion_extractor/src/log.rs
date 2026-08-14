use std::io::Write;

// Choose the log destination: append to `path`, or fall back to stderr if it
// can't be opened (e.g. read-only mount). Factored out of `init` so the
// fallback behaviour can be tested without touching the global logger.
fn open_log_writer(path: &str) -> Box<dyn Write + Send> {
    match std::fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(file) => Box::new(file),
        Err(err) => {
            eprintln!(
                "Failed to open log file {}: {}; falling back to stderr",
                path, err
            );
            Box::new(std::io::stderr())
        }
    }
}

pub fn init(path: &str) {
    if let Err(err) = simplelog::WriteLogger::init(
        log::LevelFilter::Debug,
        simplelog::Config::default(),
        open_log_writer(path),
    ) {
        eprintln!("Failed to initialize logger: {}", err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_open_log_writer_writes_to_file() {
        let path = std::env::temp_dir()
            .join(format!("kompanion_extractor_log_{}.log", std::process::id()));
        let path_str = path.to_str().unwrap();
        let _ = std::fs::remove_file(&path);

        {
            let mut writer = open_log_writer(path_str);
            writeln!(writer, "hello log").unwrap();
            writer.flush().unwrap();
        }

        let mut contents = String::new();
        std::fs::File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert!(contents.contains("hello log"), "message written to file");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_open_log_writer_appends() {
        let path = std::env::temp_dir()
            .join(format!("kompanion_extractor_log_append_{}.log", std::process::id()));
        let path_str = path.to_str().unwrap();
        let _ = std::fs::remove_file(&path);

        for msg in ["first", "second"] {
            let mut writer = open_log_writer(path_str);
            writeln!(writer, "{}", msg).unwrap();
            writer.flush().unwrap();
        }

        let mut contents = String::new();
        std::fs::File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert!(contents.contains("first"), "first message retained");
        assert!(contents.contains("second"), "second message appended, not truncated");
        assert_eq!(contents.lines().count(), 2, "both lines present");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_open_log_writer_bad_path_falls_back() {
        // An unwritable path must fall back to stderr rather than panic. The
        // returned writer must still accept writes without erroring.
        let mut writer = open_log_writer("/nonexistent/dir/kompanion.log");
        writeln!(writer, "falls back to stderr").unwrap();
        writer.flush().unwrap();
    }
}
