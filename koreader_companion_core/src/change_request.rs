use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};

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
    mime_type: &str,
    extra: Option<&Map<String, Value>>,
) -> Value {
    let metadata = std::fs::metadata(file_path).ok();

    let display_name = name_string.map(|s| s.to_string()).unwrap_or_else(|| {
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
        "mimeType": mime_type,
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

    if let Some(ref m) = metadata {
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

    if let Some(extra_map) = extra {
        let obj = insert.as_object_mut().unwrap();
        for (k, v) in extra_map {
            obj.insert(k.clone(), v.clone());
        }
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

    #[test]
    fn test_basic_change_request() {
        let json = generate_change_request(
            "/mnt/us/scripts/test.sh",
            "test-uuid-123",
            Some("My Script"),
            Some("Marek"),
            Some("/mnt/us/scripts/test.sh.sdr/icon.png"),
            true,
            "text/x-shellscript",
            None,
        );
        assert_eq!(json["type"], "ChangeRequest");
        let insert = &json["commands"][0]["insert"];
        assert_eq!(insert["uuid"], "test-uuid-123");
        assert_eq!(insert["mimeType"], "text/x-shellscript");
        assert_eq!(insert["cdeType"], "PDOC");
        assert_eq!(insert["displayTags"].as_array().unwrap()[0], "NEW");
        assert_eq!(insert["titles"][0]["display"], "My Script");
        assert_eq!(insert["credits"][0]["name"]["display"], "Marek");
        assert_eq!(insert["thumbnail"], "/mnt/us/scripts/test.sh.sdr/icon.png");
    }

    #[test]
    fn test_custom_mime_type() {
        let json = generate_change_request(
            "/mnt/us/books/book.epub",
            "uuid-epub",
            Some("My Book"),
            Some("Author"),
            None,
            true,
            "application/epub+zip",
            None,
        );
        let insert = &json["commands"][0]["insert"];
        assert_eq!(insert["mimeType"], "application/epub+zip");
    }

    #[test]
    fn test_extra_fields() {
        let mut extra = Map::new();
        extra.insert("description".to_string(), json!("A test book"));
        extra.insert("language".to_string(), json!("en"));

        let json = generate_change_request(
            "/mnt/us/books/book.epub",
            "uuid-extra",
            Some("Book"),
            Some("Author"),
            None,
            false,
            "application/epub+zip",
            Some(&extra),
        );
        let insert = &json["commands"][0]["insert"];
        assert_eq!(insert["description"], "A test book");
        assert_eq!(insert["language"], "en");
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
            "text/x-shellscript",
            None,
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
            "text/x-shellscript",
            None,
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
            "text/x-shellscript",
            None,
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
            "text/x-shellscript",
            None,
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
            "text/x-shellscript",
            None,
        );
        let insert = &json["commands"][0]["insert"];
        assert_eq!(insert["percentFinished"], 0);
        assert!(insert.get("displayTags").is_none());
    }
}
