use sh_integration_core::fs_ops::rmdir_r;
use sh_integration_core::header::parse_header;
use sh_integration_sys::scanner::ScanManager;

pub fn remove_file(
    path: &str,
    filename: &str,
    uuid: Option<&str>,
    scanner: &(dyn ScanManager + 'static),
) {
    let full_path = format!("{}/{}", path, filename);
    eprintln!("Removing file: {}", full_path);

    let sdr_path = format!("{}.sdr", full_path);
    let sdr_script_path = format!("{}/script.sh", sdr_path);

    eprintln!("Loading file");
    if let Ok(content) = std::fs::read_to_string(&sdr_script_path) {
        let header = parse_header(&content);

        if let Some(u) = uuid {
            eprintln!("Removing ccat entry.");
            scanner.delete_ccat_entry(u);
        }

        if header.use_hooks {
            eprintln!("Script uses hooks!");
            let escaped_path = escape_quotes(&sdr_script_path);
            let command = format!("source \"{}\"; on_remove;", escaped_path);
            eprintln!("Running remove event");
            let _ = execute_su_command(&command);
        }
    } else if let Some(u) = uuid {
        eprintln!("Removing ccat entry.");
        scanner.delete_ccat_entry(u);
    }

    use std::path::Path;
    let sdr = Path::new(&sdr_path);
    if sdr.exists() {
        eprintln!("SDR exists - deleting");
        let _ = rmdir_r(sdr);
    }
}

fn escape_quotes(s: &str) -> String {
    s.chars()
        .flat_map(|c| if c == '"' { vec!['\\', '"'] } else { vec![c] })
        .collect()
}

fn execute_su_command(command: &str) -> i32 {
    match std::process::Command::new("/var/local/mkk/su")
        .arg("-c")
        .arg(command)
        .spawn()
    {
        Ok(mut child) => match child.wait() {
            Ok(status) => status.code().unwrap_or(-1),
            Err(_) => -1,
        },
        Err(_) => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sh_integration_sys::scanner::MockScanManager;

    #[test]
    fn test_remove_file_deletes_sdr() {
        let tmp = std::env::temp_dir().join(format!("sh_remove_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).ok();

        let sdr_path = tmp.join("test_remove.sh.sdr");
        std::fs::create_dir_all(&sdr_path).unwrap();
        std::fs::write(sdr_path.join("some_file"), b"test").unwrap();

        let mock = MockScanManager::default();

        remove_file(
            &tmp.to_string_lossy(),
            "test_remove.sh",
            Some("test-uuid-1"),
            &mock,
        );

        assert!(!sdr_path.exists(), "SDR should be removed");
        let deleted = mock.deleted_entries.lock().unwrap();
        assert!(deleted.contains(&"test-uuid-1".to_string()));
    }

    #[test]
    fn test_remove_file_no_uuid() {
        let tmp = std::env::temp_dir().join(format!("sh_remove2_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).ok();

        let mock = MockScanManager::default();

        remove_file(&tmp.to_string_lossy(), "nonexistent.sh", None, &mock);

        let deleted = mock.deleted_entries.lock().unwrap();
        assert!(deleted.is_empty());
    }
}
