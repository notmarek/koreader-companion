use std::collections::HashMap;

use cjson_binding::{CJson, CJsonResult};
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

fn basename(file_path: &str) -> &str {
    std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
}

pub fn generate_change_request(
    json: &mut CJson,
    file_path: &str,
    uuid: &str,
    name_string: Option<&str>,
    author_string: Option<&str>,
    icon_string: Option<&str>,
    is_new: bool,
    mime_type: &str,
    extra: Option<&HashMap<String, String>>,
) -> CJsonResult<()> {
    let metadata = std::fs::metadata(file_path).ok();

    json.add_string_to_object("type", "ChangeRequest")?;

    let mut commands = CJson::create_array()?;
    let mut command = CJson::create_object()?;
    let mut insert = CJson::create_object()?;

    insert.add_string_to_object("uuid", uuid)?;
    insert.add_string_to_object("location", file_path)?;
    insert.add_string_to_object("type", "Entry:Item")?;

    if let Some(ref m) = metadata {
        if let Ok(time) = m.modified() {
            if let Ok(dur) = time.duration_since(std::time::UNIX_EPOCH) {
                insert.add_number_to_object("modificationTime", dur.as_secs() as f64)?;
            }
        }
        insert.add_number_to_object("diskUsage", m.len() as f64)?;
        insert.add_number_to_object("contentSize", m.len() as f64)?;
    }

    insert.add_string_to_object("mimeType", mime_type)?;
    insert.add_string_to_object("cdeKey", &compute_sha1(file_path))?;
    insert.add_string_to_object("cdeType", "PDOC")?;

    if is_new {
        let tags = CJson::create_string_array(&["NEW"])?;
        insert.add_item_to_object("displayTags", tags)?;
    } else {
        insert.add_number_to_object("percentFinished", 0.0)?;
    }

    insert.add_bool_to_object("isVisibleInHome", true)?;
    insert.add_bool_to_object("isArchived", false)?;

    {
        let mut display_objects = CJson::create_array()?;

        let mut title_display = CJson::create_object()?;
        title_display.add_string_to_object("ref", "titles")?;
        display_objects.add_item_to_array(title_display)?;

        let mut credits_display = CJson::create_object()?;
        credits_display.add_string_to_object("ref", "credits")?;
        display_objects.add_item_to_array(credits_display)?;

        insert.add_item_to_object("displayObjects", display_objects)?;
    }

    {
        let mut credits = CJson::create_array()?;
        let mut credit = CJson::create_object()?;
        credit.add_string_to_object("kind", "Author")?;

        let mut name = CJson::create_object()?;
        name.add_string_to_object("display", author_string.unwrap_or("Unknown"))?;
        credit.add_item_to_object("name", name)?;

        credits.add_item_to_array(credit)?;
        insert.add_item_to_object("credits", credits)?;
    }

    {
        let mut titles = CJson::create_array()?;
        let mut title_display = CJson::create_object()?;
        title_display.add_string_to_object(
            "display",
            name_string.unwrap_or_else(|| basename(file_path)),
        )?;
        titles.add_item_to_array(title_display)?;
        insert.add_item_to_object("titles", titles)?;
    }

    if let Some(icon) = icon_string {
        insert.add_string_to_object("thumbnail", icon)?;
    }

    // if let Some(extra_map) = extra {
    //     for (k, v) in extra_map {
    //         insert.add_string_to_object(k, v)?;
    //     }
    // }

    command.add_item_to_object("insert", insert)?;
    commands.add_item_to_array(command)?;
    json.add_item_to_object("commands", commands)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cjson_binding::CJsonRef;

    fn navigate_insert(json: &CJson) -> (CJsonRef, CJsonRef, CJsonRef) {
        let commands = json.get_object_item("commands").unwrap();
        let cmd = commands.get_array_item(0).unwrap();
        let insert = cmd.get_object_item("insert").unwrap();
        (commands, cmd, insert)
    }

    #[test]
    fn test_basic_change_request() {
        let mut json = CJson::create_object().unwrap();
        generate_change_request(
            &mut json,
            "/mnt/us/scripts/test.sh",
            "test-uuid-123",
            Some("My Script"),
            Some("Marek"),
            Some("/mnt/us/scripts/test.sh.sdr/icon.png"),
            true,
            "text/x-shellscript",
            None,
        )
        .unwrap();

        assert_eq!(
            json.get_object_item("type")
                .unwrap()
                .get_string_value()
                .unwrap(),
            "ChangeRequest"
        );

        let (_, _, insert) = navigate_insert(&json);
        assert_eq!(
            insert
                .get_object_item("uuid")
                .unwrap()
                .get_string_value()
                .unwrap(),
            "test-uuid-123"
        );
        assert_eq!(
            insert
                .get_object_item("mimeType")
                .unwrap()
                .get_string_value()
                .unwrap(),
            "text/x-shellscript"
        );
        assert_eq!(
            insert
                .get_object_item("cdeType")
                .unwrap()
                .get_string_value()
                .unwrap(),
            "PDOC"
        );

        let tags = insert.get_object_item("displayTags").unwrap();
        assert_eq!(
            tags.get_array_item(0).unwrap().get_string_value().unwrap(),
            "NEW"
        );

        let titles = insert.get_object_item("titles").unwrap();
        let title_entry = titles.get_array_item(0).unwrap();
        assert_eq!(
            title_entry
                .get_object_item("display")
                .unwrap()
                .get_string_value()
                .unwrap(),
            "My Script"
        );

        let credits = insert.get_object_item("credits").unwrap();
        let credit = credits.get_array_item(0).unwrap();
        let name = credit.get_object_item("name").unwrap();
        assert_eq!(
            name.get_object_item("display")
                .unwrap()
                .get_string_value()
                .unwrap(),
            "Marek"
        );

        assert_eq!(
            insert
                .get_object_item("thumbnail")
                .unwrap()
                .get_string_value()
                .unwrap(),
            "/mnt/us/scripts/test.sh.sdr/icon.png"
        );

        // Regression: cjson stub on kindlehf caused TypeError on bool fields.
        // Verify these fields are present and the add succeeded.
        assert!(insert.get_object_item("isVisibleInHome").is_ok());
        assert!(insert.get_object_item("isArchived").is_ok());
    }

    #[test]
    fn test_custom_mime_type() {
        let mut json = CJson::create_object().unwrap();
        generate_change_request(
            &mut json,
            "/mnt/us/books/book.epub",
            "uuid-epub",
            Some("My Book"),
            Some("Author"),
            None,
            true,
            "application/epub+zip",
            None,
        )
        .unwrap();

        let (_, _, insert) = navigate_insert(&json);
        assert_eq!(
            insert
                .get_object_item("mimeType")
                .unwrap()
                .get_string_value()
                .unwrap(),
            "application/epub+zip"
        );
    }

    #[test]
    fn test_extra_fields() {
        let mut extra = HashMap::new();
        extra.insert("description".to_string(), "A test book".to_string());
        extra.insert("language".to_string(), "en".to_string());

        let mut json = CJson::create_object().unwrap();
        generate_change_request(
            &mut json,
            "/mnt/us/books/book.epub",
            "uuid-extra",
            Some("Book"),
            Some("Author"),
            None,
            false,
            "application/epub+zip",
            Some(&extra),
        )
        .unwrap();

        let (_, _, insert) = navigate_insert(&json);
        assert_eq!(
            insert
                .get_object_item("description")
                .unwrap()
                .get_string_value()
                .unwrap(),
            "A test book"
        );
        assert_eq!(
            insert
                .get_object_item("language")
                .unwrap()
                .get_string_value()
                .unwrap(),
            "en"
        );
    }

    #[test]
    fn test_null_name_uses_basename() {
        let mut json = CJson::create_object().unwrap();
        generate_change_request(
            &mut json,
            "/mnt/us/scripts/test.sh",
            "uuid-1",
            None,
            Some("Author"),
            None,
            false,
            "text/x-shellscript",
            None,
        )
        .unwrap();

        let (_, _, insert) = navigate_insert(&json);
        let titles = insert.get_object_item("titles").unwrap();
        let title_entry = titles.get_array_item(0).unwrap();
        assert_eq!(
            title_entry
                .get_object_item("display")
                .unwrap()
                .get_string_value()
                .unwrap(),
            "test.sh"
        );
    }

    #[test]
    fn test_null_author_defaults_to_unknown() {
        let mut json = CJson::create_object().unwrap();
        generate_change_request(
            &mut json,
            "/mnt/us/scripts/test.sh",
            "uuid-2",
            Some("Name"),
            None,
            None,
            false,
            "text/x-shellscript",
            None,
        )
        .unwrap();

        let (_, _, insert) = navigate_insert(&json);
        let credits = insert.get_object_item("credits").unwrap();
        let credit = credits.get_array_item(0).unwrap();
        let name = credit.get_object_item("name").unwrap();
        assert_eq!(
            name.get_object_item("display")
                .unwrap()
                .get_string_value()
                .unwrap(),
            "Unknown"
        );
    }

    #[test]
    fn test_null_icon_omits_thumbnail() {
        let mut json = CJson::create_object().unwrap();
        generate_change_request(
            &mut json,
            "/mnt/us/scripts/test.sh",
            "uuid-3",
            Some("Name"),
            Some("Author"),
            None,
            false,
            "text/x-shellscript",
            None,
        )
        .unwrap();

        let (_, _, insert) = navigate_insert(&json);
        assert!(insert.get_object_item("thumbnail").is_err());
    }

    #[test]
    fn test_new_flag_adds_display_tags() {
        let mut json = CJson::create_object().unwrap();
        generate_change_request(
            &mut json,
            "/mnt/us/scripts/test.sh",
            "uuid-4",
            Some("Name"),
            Some("Author"),
            None,
            true,
            "text/x-shellscript",
            None,
        )
        .unwrap();

        let (_, _, insert) = navigate_insert(&json);
        let tags = insert.get_object_item("displayTags").unwrap();
        assert_eq!(
            tags.get_array_item(0).unwrap().get_string_value().unwrap(),
            "NEW"
        );
        assert!(insert.get_object_item("percentFinished").is_err());
    }

    #[test]
    fn test_update_adds_percent_finished() {
        let mut json = CJson::create_object().unwrap();
        generate_change_request(
            &mut json,
            "/mnt/us/scripts/test.sh",
            "uuid-5",
            Some("Name"),
            Some("Author"),
            None,
            false,
            "text/x-shellscript",
            None,
        )
        .unwrap();

        let (_, _, insert) = navigate_insert(&json);
        let percent = insert.get_object_item("percentFinished").unwrap();
        assert_eq!(percent.get_number_value().unwrap(), 0.0);
        assert!(insert.get_object_item("displayTags").is_err());
    }
}
