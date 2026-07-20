use std::path::Path;

use sh_integration_core::base64;
use sh_integration_core::change_request::generate_change_request;
use sh_integration_core::header::parse_header;
use sh_integration_sys::scanner::ScanManager;

pub fn index_file(
    path: &str,
    filename: &str,
    is_new: bool,
    scanner: &(dyn ScanManager + 'static),
) {
    let full_path = format!("{}/{}", path, filename);
    eprintln!("Indexing file: {}", full_path);

    let mut uuid_buf = [0u8; 37];
    scanner.gen_uuid(&mut uuid_buf);
    let uuid = String::from_utf8(uuid_buf.iter().take_while(|&&b| b != 0).copied().collect())
        .unwrap_or_default();
    eprintln!("Generated UUID: {}", uuid);

    let content = match std::fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to open file: {}", e);
            return;
        }
    };

    eprintln!("Reading header...");
    let header = parse_header(&content);

    let mut valid_icon = false;
    let mut final_icon = header.icon.clone();

    if let Some(ref icon) = header.icon {
        if icon.starts_with("data:image") {
            valid_icon = true;
        } else if Path::new(icon).exists() {
            valid_icon = true;
        }
    }

    if valid_icon || header.use_hooks {
        eprintln!("Valid icon OR uses hooks");
        let sdr_path = format!("{}.sdr", full_path);
        let _ = std::fs::create_dir_all(&sdr_path);

        if valid_icon {
            if let Some(ref icon_str) = header.icon {
                if icon_str.starts_with("data:image") {
                    eprintln!("Valid BASE64 icon detected, attempting to extract it");
                    if let Some(ext) = base64::get_icon_extension(icon_str) {
                        let icon_sdr_path = format!("{}/icon.{}", sdr_path, ext);
                        if base64::decode_base64_to_file(icon_str, &icon_sdr_path).is_ok() {
                            final_icon = Some(icon_sdr_path);
                        }
                    }
                }
            }
        }

        if header.use_hooks {
            eprintln!("Script uses hooks!");
            let escaped_path = escape_quotes(&full_path);
            eprintln!("Running install event");
            let command = format!("source \"{}\"; on_install;", escaped_path);
            let _ = execute_su_command(&command);

            let sdr_script_path = format!("{}/script.sh", sdr_path);
            eprintln!("Writing script to {}", sdr_script_path);
            let _ = std::fs::write(&sdr_script_path, &content);
        }
    }

    let json = generate_change_request(
        &full_path,
        &uuid,
        header.name.as_deref(),
        header.author.as_deref(),
        final_icon.as_deref(),
        is_new,
    );

    let result = scanner.post_change(&json);
    eprintln!(
        "Indexing json:\n{}",
        serde_json::to_string_pretty(&json).unwrap_or_default()
    );
    eprintln!("ccat error: {}", result);
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

    fn fixture_path(name: &str) -> String {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures")
            .join(name)
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn test_index_file_with_mock_scanner() {
        let tmp = std::env::temp_dir().join(format!("sh_test_idx_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).ok();
        let test_sh = fixture_path("test.sh");
        std::fs::copy(&test_sh, tmp.join("test.sh")).ok();

        let mock = MockScanManager::default();
        index_file(&tmp.to_string_lossy(), "test.sh", true, &mock);

        let changes = mock.posted_changes.lock().unwrap();
        assert!(!changes.is_empty());
        let json: serde_json::Value = serde_json::from_str(&changes[0]).unwrap();
        assert_eq!(json["type"], "ChangeRequest");
        assert_eq!(json["commands"][0]["insert"]["cdeType"], "PDOC");
    }

    #[test]
    fn test_index_file_sets_display_tags_for_new() {
        let tmp = std::env::temp_dir().join(format!("sh_test_new_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).ok();
        let test_sh = fixture_path("test.sh");
        std::fs::copy(&test_sh, tmp.join("test.sh")).ok();

        let mock = MockScanManager::default();
        index_file(&tmp.to_string_lossy(), "test.sh", true, &mock);

        let changes = mock.posted_changes.lock().unwrap();
        let json: serde_json::Value = serde_json::from_str(&changes[0]).unwrap();
        let insert = &json["commands"][0]["insert"];
        assert_eq!(insert["displayTags"].as_array().unwrap()[0], "NEW");
    }

    #[test]
    fn test_index_file_percent_finished_for_update() {
        let tmp = std::env::temp_dir().join(format!("sh_test_upd_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).ok();
        let test_sh = fixture_path("test.sh");
        std::fs::copy(&test_sh, tmp.join("test.sh")).ok();

        let mock = MockScanManager::default();
        index_file(&tmp.to_string_lossy(), "test.sh", false, &mock);

        let changes = mock.posted_changes.lock().unwrap();
        let json: serde_json::Value = serde_json::from_str(&changes[0]).unwrap();
        let insert = &json["commands"][0]["insert"];
        assert_eq!(insert["percentFinished"], 0);
        assert!(insert.get("displayTags").is_none());
    }

    #[test]
    fn test_index_file_with_hooks_creates_sdr() {
        let tmp = std::env::temp_dir().join(format!("sh_test_hooks_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).ok();
        let test_hooks = fixture_path("test_hooks.sh");
        std::fs::copy(&test_hooks, tmp.join("test_hooks.sh")).ok();

        let mock = MockScanManager::default();
        index_file(&tmp.to_string_lossy(), "test_hooks.sh", true, &mock);

        let sdr_path = tmp.join("test_hooks.sh.sdr");
        let script_in_sdr = sdr_path.join("script.sh");
        assert!(script_in_sdr.exists(), "SDR script.sh should exist for hooks script");
    }

    #[test]
    fn test_index_file_noicon() {
        let tmp = std::env::temp_dir().join(format!("sh_test_noicon_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).ok();
        let test_noicon = fixture_path("test_noicon.sh");
        std::fs::copy(&test_noicon, tmp.join("test_noicon.sh")).ok();

        let mock = MockScanManager::default();
        index_file(&tmp.to_string_lossy(), "test_noicon.sh", true, &mock);

        let changes = mock.posted_changes.lock().unwrap();
        assert!(!changes.is_empty());
        let json: serde_json::Value = serde_json::from_str(&changes[0]).unwrap();
        assert!(json["commands"][0]["insert"].get("thumbnail").is_none());
    }

    #[test]
    fn test_escape_quotes() {
        assert_eq!(escape_quotes("hello\"world"), "hello\\\"world");
        assert_eq!(escape_quotes("no_quotes"), "no_quotes");
    }
}
