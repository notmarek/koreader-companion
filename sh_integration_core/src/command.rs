use crate::ScriptHeader;

pub fn generate_command(script_path: &str, header: &ScriptHeader) -> String {
    let escaped_path = escape_quotes(script_path);

    let mut command = if header.use_hooks {
        format!("sh -l -c \"source \\\"{}\\\"; on_run;\"", escaped_path)
    } else {
        format!("sh -l \"{}\"", escaped_path)
    };

    if header.use_fbink {
        command = format!(
            "/mnt/us/libkh/bin/fbink -k; {} 2>&1 | /mnt/us/libkh/bin/fbink -y 5 -r",
            command
        );
    }

    command
}

pub fn escape_quotes(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c == '"' {
                vec!['\\', '"']
            } else {
                vec![c]
            }
        })
        .collect()
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
    fn test_generate_command_fbink() {
        let content = read_fixture("test_fbink.sh");
        let header = crate::header::parse_header(&content);
        let cmd = generate_command("/mnt/us/scripts/test_fbink.sh", &header);
        assert!(cmd.contains("fbink -k"));
        assert!(cmd.contains("fbink -y 5 -r"));
        assert!(cmd.contains("sh -l \"/mnt/us/scripts/test_fbink.sh\""));
    }

    #[test]
    fn test_generate_command_hooks() {
        let content = read_fixture("test_hooks.sh");
        let header = crate::header::parse_header(&content);
        let cmd = generate_command("/mnt/us/scripts/test_hooks.sh", &header);
        assert!(cmd.contains("source"));
        assert!(cmd.contains("on_run"));
        assert!(!cmd.contains("fbink"));
    }

    #[test]
    fn test_generate_command_hooks_fbink() {
        let content = read_fixture("test_hooks_fbink.sh");
        let header = crate::header::parse_header(&content);
        let cmd = generate_command("/mnt/us/scripts/test_hooks_fbink.sh", &header);
        assert!(cmd.contains("source"));
        assert!(cmd.contains("on_run"));
        assert!(cmd.contains("fbink"));
    }

    #[test]
    fn test_generate_command_basic() {
        let content = read_fixture("test.sh");
        let header = crate::header::parse_header(&content);
        let cmd = generate_command("/mnt/us/scripts/test.sh", &header);
        assert!(cmd.contains("sh -l \"/mnt/us/scripts/test.sh\""));
        assert!(!cmd.contains("source"));
        assert!(!cmd.contains("fbink"));
    }

    #[test]
    fn test_escape_quotes() {
        assert_eq!(escape_quotes("hello\"world"), "hello\\\"world");
        assert_eq!(escape_quotes("no_quotes"), "no_quotes");
    }
}
