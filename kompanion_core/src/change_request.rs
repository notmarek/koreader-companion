use std::collections::HashMap;

use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};

fn compute_sha1(file_path: &str) -> String {
    if let Ok(data) = std::fs::read(file_path) {
        let mut hasher = Sha1::new();
        hasher.update(&data);
        hex::encode(hasher.finalize())
    } else {
        String::new()
    }
}

fn basename(file_path: &str) -> &str {
    std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
}

/// Extra metadata keys mapped to CCat insert fields (all validated by the ccat
/// `change.lua` column_specs).
const EXTRA_LANGUAGE: &str = "language";
const EXTRA_PUBLISHER: &str = "publisher";
const EXTRA_PUBLICATION_DATE: &str = "publicationDate";

/// Build a CCat `/change` ChangeRequest with a single `insert` command for the
/// entry at `file_path`.
///
/// `cde_type` drives how the library classifies the item (EBOK = book, PDOC =
/// personal document, ...); sideloaded books should be EBOK.
#[allow(clippy::too_many_arguments)]
pub fn generate_change_request(
    file_path: &str,
    uuid: &str,
    name: Option<&str>,
    author: Option<&str>,
    icon: Option<&str>,
    is_new: bool,
    mime_type: &str,
    cde_type: &str,
    extra: Option<&HashMap<String, String>>,
) -> Value {
    let metadata = std::fs::metadata(file_path).ok();

    let mut insert = Map::new();

    insert.insert("uuid".to_string(), Value::String(uuid.to_string()));
    insert.insert("location".to_string(), Value::String(file_path.to_string()));
    insert.insert("type".to_string(), Value::String("Entry:Item".to_string()));

    if let Some(ref m) = metadata {
        if let Ok(time) = m.modified() {
            if let Ok(dur) = time.duration_since(std::time::UNIX_EPOCH) {
                insert.insert(
                    "modificationTime".to_string(),
                    Value::Number(dur.as_secs().into()),
                );
            }
        }
        insert.insert("diskUsage".to_string(), Value::Number(m.len().into()));
        insert.insert("contentSize".to_string(), Value::Number(m.len().into()));
    }

    insert.insert("mimeType".to_string(), Value::String(mime_type.to_string()));
    insert.insert("cdeKey".to_string(), Value::String(compute_sha1(file_path)));
    insert.insert("cdeType".to_string(), Value::String(cde_type.to_string()));

    if is_new {
        // Display "NEW" badge on the library card.
        insert.insert("displayTags".to_string(), json!(["NEW"]));
    } else {
        // Keep the existing reading progress position after a re-insert.
        insert.insert("percentFinished".to_string(), Value::Number(serde_json::Number::from_f64(0.0).unwrap()));
    }

    insert.insert("isVisibleInHome".to_string(), Value::Bool(true));
    insert.insert("isArchived".to_string(), Value::Bool(false));

    insert.insert(
        "displayObjects".to_string(),
        json!([{ "ref": "titles" }, { "ref": "credits" }]),
    );
    insert.insert(
        "credits".to_string(),
        json!([{ "kind": "Author", "name": { "display": author.unwrap_or("Unknown") } }]),
    );
    insert.insert(
        "titles".to_string(),
        json!([{ "display": name.unwrap_or_else(|| basename(file_path)) }]),
    );

    if let Some(icon) = icon {
        insert.insert("thumbnail".to_string(), Value::String(icon.to_string()));
    }

    if let Some(extra_map) = extra {
        if let Some(lang) = extra_map.get(EXTRA_LANGUAGE).filter(|l| !l.is_empty()) {
            // ccat derives titles[1].language from languages[1].
            insert.insert("languages".to_string(), json!([lang]));
        }
        if let Some(publisher) = extra_map.get(EXTRA_PUBLISHER).filter(|p| !p.is_empty()) {
            insert.insert("publisher".to_string(), Value::String(publisher.clone()));
        }
        if let Some(date) = extra_map
            .get(EXTRA_PUBLICATION_DATE)
            .filter(|d| !d.is_empty())
        {
            insert.insert(
                "publicationDate".to_string(),
                Value::String(date.clone()),
            );
        }
    }

    json!({
        "type": "ChangeRequest",
        "commands": [{ "insert": insert }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn navigate_insert(json: &Value) -> &Map<String, Value> {
        json["commands"][0]["insert"].as_object().unwrap()
    }

    fn assert_extra(insert: &Map<String, Value>, key: &str, expected: &str) {
        assert_eq!(insert[key].as_str().unwrap(), expected);
    }

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
            "PDOC",
            None,
        );

        assert_eq!(json["type"].as_str().unwrap(), "ChangeRequest");

        let insert = navigate_insert(&json);
        assert_eq!(insert["uuid"].as_str().unwrap(), "test-uuid-123");
        assert_eq!(insert["mimeType"].as_str().unwrap(), "text/x-shellscript");
        assert_eq!(insert["cdeType"].as_str().unwrap(), "PDOC");
        assert_eq!(insert["displayTags"][0].as_str().unwrap(), "NEW");
        assert_eq!(insert["titles"][0]["display"].as_str().unwrap(), "My Script");
        assert_eq!(
            insert["credits"][0]["name"]["display"].as_str().unwrap(),
            "Marek"
        );
        assert_eq!(
            insert["thumbnail"].as_str().unwrap(),
            "/mnt/us/scripts/test.sh.sdr/icon.png"
        );

        // Regression: bool fields must be present with the right values.
        assert_eq!(insert["isVisibleInHome"].as_bool().unwrap(), true);
        assert_eq!(insert["isArchived"].as_bool().unwrap(), false);
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
            "EBOK",
            None,
        );

        let insert = navigate_insert(&json);
        assert_eq!(
            insert["mimeType"].as_str().unwrap(),
            "application/epub+zip"
        );
        assert_eq!(insert["cdeType"].as_str().unwrap(), "EBOK");
    }

    #[test]
    fn test_extra_fields() {
        let mut extra = HashMap::new();
        extra.insert("language".to_string(), "en".to_string());
        extra.insert("publisher".to_string(), "Example Press".to_string());
        extra.insert("publicationDate".to_string(), "2015-07-01".to_string());

        let json = generate_change_request(
            "/mnt/us/books/book.epub",
            "uuid-extra",
            Some("Book"),
            Some("Author"),
            None,
            false,
            "application/epub+zip",
            "EBOK",
            Some(&extra),
        );

        let insert = navigate_insert(&json);
        assert_eq!(insert["languages"][0].as_str().unwrap(), "en");
        assert_extra(insert, "publisher", "Example Press");
        assert_extra(insert, "publicationDate", "2015-07-01");
    }

    #[test]
    fn test_extra_fields_skipped_when_empty() {
        let mut extra = HashMap::new();
        extra.insert("language".to_string(), "".to_string());
        extra.insert("description".to_string(), "not a ccat field".to_string());

        let json = generate_change_request(
            "/mnt/us/books/book.epub",
            "uuid-extra",
            None,
            None,
            None,
            false,
            "application/epub+zip",
            "EBOK",
            Some(&extra),
        );

        let insert = navigate_insert(&json);
        assert!(insert.get("languages").is_none());
        assert!(insert.get("publisher").is_none());
        assert!(insert.get("description").is_none());
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
            "PDOC",
            None,
        );

        let insert = navigate_insert(&json);
        assert_eq!(insert["titles"][0]["display"].as_str().unwrap(), "test.sh");
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
            "PDOC",
            None,
        );

        let insert = navigate_insert(&json);
        assert_eq!(
            insert["credits"][0]["name"]["display"].as_str().unwrap(),
            "Unknown"
        );
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
            "PDOC",
            None,
        );

        let insert = navigate_insert(&json);
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
            "PDOC",
            None,
        );

        let insert = navigate_insert(&json);
        assert_eq!(insert["displayTags"][0].as_str().unwrap(), "NEW");
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
            "PDOC",
            None,
        );

        let insert = navigate_insert(&json);
        assert_eq!(insert["percentFinished"].as_f64().unwrap(), 0.0);
        assert!(insert.get("displayTags").is_none());
    }

    #[test]
    fn test_metadata_fields_from_file() {
        let path = std::env::temp_dir().join("kompanion_change_request_test.txt");
        std::fs::write(&path, b"x").unwrap();

        let json = generate_change_request(
            path.to_str().unwrap(),
            "uuid-6",
            Some("Name"),
            None,
            None,
            false,
            "text/plain",
            "PDOC",
            None,
        );

        std::fs::remove_file(&path).unwrap();

        let insert = navigate_insert(&json);
        assert!(insert["contentSize"].as_u64().unwrap() >= 1);
        assert!(insert["diskUsage"].as_u64().unwrap() >= 1);
        assert!(insert["modificationTime"].as_u64().is_some());
    }
}