use sha1::{Digest, Sha1};
use serde_json::{json, Value};

fn compute_sha1(file_path: &str) -> String {
    if let Ok(data) = std::fs::read(file_path) {
        let mut hasher = Sha1::new();
        hasher.update(&data);
        format!("{:x}", hasher.finalize())
    } else {
        String::new()
    }
}

pub fn generate_change_request(
    file_path: &str,
    uuid: &str,
    name_string: Option<&str>,
    author_string: Option<&str>,
    icon_string: Option<&str>,
    is_new: bool,
) -> Value {
    let metadata = std::fs::metadata(file_path).ok();

    let display_name = name_string
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            std::path::Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });

    let mut insert = json!({
        "uuid": uuid,
        "location": file_path,
        "type": "Entry:Item",
        "mimeType": "text/x-shellscript",
        "cdeKey": compute_sha1(file_path),
        "cdeType": "PDOC",
        "isVisibleInHome": true,
        "isArchived": false,
        "displayObjects": [
            {"ref": "titles"},
            {"ref": "credits"}
        ],
        "credits": [{
            "kind": "Author",
            "name": {
                "display": author_string.unwrap_or("Unknown")
            }
        }],
        "titles": [{
            "display": &display_name
        }]
    });

    if let Some(m) = metadata {
        if let Ok(time) = m.modified() {
            if let Ok(dur) = time.duration_since(std::time::UNIX_EPOCH) {
                insert["modificationTime"] = json!(dur.as_secs());
            }
        }
        insert["diskUsage"] = json!(m.len());
        insert["contentSize"] = json!(m.len());
    }

    if is_new {
        insert["displayTags"] = json!(["NEW"]);
    } else {
        insert["percentFinished"] = json!(0);
    }

    if let Some(icon) = icon_string {
        insert["thumbnail"] = json!(icon);
    }

    json!({
        "type": "ChangeRequest",
        "commands": [{
            "insert": insert
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_common_json_fields(json: &Value) {
        assert_eq!(json["type"], "ChangeRequest");
        let commands = &json["commands"];
        assert!(commands.is_array());
        let insert = &commands[0]["insert"];
        assert_eq!(insert["type"], "Entry:Item");
        assert_eq!(insert["mimeType"], "text/x-shellscript");
        assert_eq!(insert["cdeType"], "PDOC");
        assert_eq!(insert["isVisibleInHome"], true);
        assert_eq!(insert["isArchived"], false);
        assert!(insert["displayObjects"].is_array());
        assert!(insert["credits"].is_array());
        assert!(insert["titles"].is_array());
    }

    #[test]
    fn test_basic_change_request() {
        let json = generate_change_request(
            "/mnt/us/scripts/test.sh",
            "test-uuid-123",
            Some("My Script"),
            Some("HackerDude"),
            Some("/mnt/us/scripts/test.sh.sdr/icon.png"),
            true,
        );
        check_common_json_fields(&json);
        let insert = &json["commands"][0]["insert"];
        assert_eq!(insert["uuid"], "test-uuid-123");
        assert_eq!(insert["displayTags"].as_array().unwrap()[0], "NEW");
        assert_eq!(insert["titles"][0]["display"], "My Script");
        assert_eq!(insert["credits"][0]["name"]["display"], "HackerDude");
        assert_eq!(insert["thumbnail"], "/mnt/us/scripts/test.sh.sdr/icon.png");
    }

    #[test]
    fn test_null_name_uses_basename() {
        let json = generate_change_request(
            "/mnt/us/scripts/test.sh",
            "uuid-1",
            None,
            Some("Author"),
            None,
            false,
        );
        let insert = &json["commands"][0]["insert"];
        assert_eq!(insert["titles"][0]["display"], "test.sh");
    }

    #[test]
    fn test_null_author_defaults_to_unknown() {
        let json = generate_change_request(
            "/mnt/us/scripts/test.sh",
            "uuid-2",
            Some("Name"),
            None,
            None,
            false,
        );
        let insert = &json["commands"][0]["insert"];
        assert_eq!(insert["credits"][0]["name"]["display"], "Unknown");
    }

    #[test]
    fn test_null_icon_omits_thumbnail() {
        let json = generate_change_request(
            "/mnt/us/scripts/test.sh",
            "uuid-3",
            Some("Name"),
            Some("Author"),
            None,
            false,
        );
        let insert = &json["commands"][0]["insert"];
        assert!(insert.get("thumbnail").is_none());
    }

    #[test]
    fn test_new_flag_adds_display_tags() {
        let json = generate_change_request(
            "/mnt/us/scripts/test.sh",
            "uuid-4",
            Some("Name"),
            Some("Author"),
            None,
            true,
        );
        let insert = &json["commands"][0]["insert"];
        assert_eq!(insert["displayTags"].as_array().unwrap()[0], "NEW");
        assert!(insert.get("percentFinished").is_none());
    }

    #[test]
    fn test_update_adds_percent_finished() {
        let json = generate_change_request(
            "/mnt/us/scripts/test.sh",
            "uuid-5",
            Some("Name"),
            Some("Author"),
            None,
            false,
        );
        let insert = &json["commands"][0]["insert"];
        assert_eq!(insert["percentFinished"], 0);
        assert!(insert.get("displayTags").is_none());
    }
}
