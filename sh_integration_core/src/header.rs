use crate::ScriptHeader;

pub fn parse_header(content: &str) -> ScriptHeader {
    let mut header = ScriptHeader {
        name: None,
        author: None,
        icon: None,
        use_hooks: false,
        use_fbink: true,
    };

    for line in content.lines().take(6) {
        if let Some(value) = line.strip_prefix("# Name: ") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                header.name = Some(trimmed.to_string());
            }
        } else if let Some(value) = line.strip_prefix("# Author: ") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                header.author = Some(trimmed.to_string());
            }
        } else if let Some(value) = line.strip_prefix("# Icon: ") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                header.icon = Some(trimmed.to_string());
            }
        } else if line.trim() == "# UseHooks" {
            header.use_hooks = true;
        } else if line.trim() == "# DontUseFBInk" {
            header.use_fbink = false;
        }
    }

    header
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_fixture(name: &str) -> String {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures")
            .join(name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", path.display(), e))
    }

    #[test]
    fn test_parse_basic() {
        let content = read_fixture("test.sh");
        let header = parse_header(&content);
        assert!(header.name.is_some());
        assert!(header.author.is_some());
        assert!(header.icon.is_some());
        assert!(!header.use_hooks);
        assert!(!header.use_fbink);
    }

    #[test]
    fn test_parse_fbink() {
        let content = read_fixture("test_fbink.sh");
        let header = parse_header(&content);
        assert!(header.name.is_some());
        assert!(header.author.is_some());
        assert!(header.icon.is_some());
        assert!(!header.use_hooks);
        assert!(header.use_fbink);
    }

    #[test]
    fn test_parse_hooks() {
        let content = read_fixture("test_hooks.sh");
        let header = parse_header(&content);
        assert!(header.name.is_some());
        assert!(header.author.is_some());
        assert!(header.icon.is_some());
        assert!(header.use_hooks);
        assert!(!header.use_fbink);
    }

    #[test]
    fn test_parse_hooks_fbink() {
        let content = read_fixture("test_hooks_fbink.sh");
        let header = parse_header(&content);
        assert!(header.name.is_some());
        assert!(header.author.is_some());
        assert!(header.icon.is_some());
        assert!(header.use_hooks);
        assert!(header.use_fbink);
    }

    #[test]
    fn test_parse_noicon() {
        let content = read_fixture("test_noicon.sh");
        let header = parse_header(&content);
        assert!(header.name.is_some());
        assert_eq!(header.name.as_deref(), Some("TestName"));
        assert!(header.author.is_some());
        assert_eq!(header.author.as_deref(), Some("TestAuthor"));
        assert!(header.icon.is_none());
    }

    #[test]
    fn test_parse_noheader() {
        let content = read_fixture("test_noheader.sh");
        let header = parse_header(&content);
        assert!(header.name.is_none());
        assert!(header.author.is_none());
        assert!(header.icon.is_none());
        assert!(!header.use_hooks);
        assert!(header.use_fbink);
    }

    #[test]
    fn test_parse_kindlecraft() {
        let content = read_fixture("KindleCraft.sh");
        let header = parse_header(&content);
        assert!(header.name.is_some());
        assert!(!header.use_hooks);
        assert!(!header.use_fbink);
    }

    #[test]
    fn test_parse_ota_status() {
        let content = read_fixture("Check OTA status v1.1.sh");
        let header = parse_header(&content);
        assert!(header.name.is_some());
        assert!(header.author.is_some());
        assert!(header.use_fbink);
    }

    #[test]
    fn test_parse_jb_runner() {
        let content = read_fixture("jb.sh runner.sh");
        let header = parse_header(&content);
        assert!(header.name.is_some());
    }
}
