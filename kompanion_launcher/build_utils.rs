use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexerSpec {
    pub extension: String,
    pub mime_type: String,
}

pub fn parse_indexer_specs(indexer_dir: &Path) -> Vec<IndexerSpec> {
    let mod_rs = fs::read_to_string(indexer_dir.join("mod.rs"))
        .expect("failed to read indexer mod.rs");
    let mod_rs = before_tests(&mod_rs);

    let mut specs = Vec::new();
    for line in mod_rs.lines() {
        let trimmed = line.trim();
        let Some(name) = trimmed.strip_prefix("pub mod ").and_then(|r| r.strip_suffix(';')) else {
            continue;
        };
        let name = name.trim();
        let src = fs::read_to_string(indexer_dir.join(format!("{name}.rs")))
            .unwrap_or_else(|e| panic!("failed to read indexer module {name}.rs: {e}"));
        let src = before_tests(&src);

        let mime_type = extract_string_after(&src, "fn mime_type")
            .unwrap_or_else(|| panic!("indexer {name}.rs: no mime_type() string literal found"));
        let extensions = extract_ends_with_extensions(&src);
        assert!(
            !extensions.is_empty(),
            "indexer {name}.rs: no ends_with(\".ext\") found in can_handle()"
        );

        for extension in extensions {
            specs.push(IndexerSpec {
                extension,
                mime_type: mime_type.clone(),
            });
        }
    }
    assert!(
        !specs.is_empty(),
        "no indexer modules found in {}",
        indexer_dir.display()
    );
    specs
}

pub fn generate_install_sql(specs: &[IndexerSpec]) -> String {
    let exts = quoted_ext_list(specs);
    let mut out = String::from("BEGIN;\n");
    out.push_str(&format!("DELETE FROM mimetypes WHERE ext IN ({exts});\n"));
    out.push_str(&format!("DELETE FROM extenstions WHERE ext IN ({exts});\n\n"));

    for spec in specs {
        out.push_str(&format!(
            "INSERT INTO mimetypes (ext, mimetype) VALUES ('{}', 'MT:{}');\n",
            spec.extension, spec.mime_type
        ));
        out.push_str(&format!(
            "INSERT INTO extenstions (ext, mimetype) VALUES ('{}', 'MT:{}');\n",
            spec.extension, spec.mime_type
        ));
    }

    out.push_str("\n-- Launcher: clean old rows first (safe reinstall)\n");
    out.push_str("DELETE FROM associations WHERE handlerId = 'com.notmarek.kompanion.launcher';\n");
    out.push_str("DELETE FROM properties WHERE handlerId = 'com.notmarek.kompanion.launcher';\n");
    out.push_str("DELETE FROM handlerIds WHERE handlerId = 'com.notmarek.kompanion.launcher';\n\n");
    out.push_str("INSERT INTO handlerIds (handlerId) VALUES ('com.notmarek.kompanion.launcher');\n");
    for (name, value) in [
        ("extend-start", "Y"),
        ("unloadPolicy", "unloadOnPause"),
        ("maxGoTime", "60"),
        ("maxPauseTime", "60"),
        ("maxUnloadTime", "60"),
        ("maxLoadTime", "60"),
        ("command", "/var/local/kompanion/bin/kompanion_launcher"),
    ] {
        out.push_str(&format!(
            "INSERT INTO properties (handlerId, name, value) VALUES ('com.notmarek.kompanion.launcher', '{name}', '{value}');\n"
        ));
    }
    for spec in specs {
        out.push_str(&format!(
            "INSERT INTO associations (interface, handlerId, contentId, defaultAssoc) VALUES ('application', 'com.notmarek.kompanion.launcher', 'MT:{}', 'true');\n",
            spec.mime_type
        ));
    }

    out.push_str("\n-- Extractor: clean old rows first (safe reinstall), path set by install.sh\n");
    out.push_str("DELETE FROM associations WHERE handlerId = 'com.notmarek.kompanion.extractor';\n");
    out.push_str("DELETE FROM properties WHERE handlerId = 'com.notmarek.kompanion.extractor';\n");
    out.push_str("DELETE FROM handlerIds WHERE handlerId = 'com.notmarek.kompanion.extractor';\n\n");
    out.push_str("INSERT INTO handlerIds (handlerId) VALUES ('com.notmarek.kompanion.extractor');\n");
    out.push_str("INSERT INTO properties (handlerId, name, value) VALUES ('com.notmarek.kompanion.extractor', 'lib', '@EXTRACTOR_LIB@');\n");
    out.push_str("INSERT INTO properties (handlerId, name, value) VALUES ('com.notmarek.kompanion.extractor', 'entry', 'load_extractor');\n");
    for spec in specs {
        out.push_str(&format!(
            "INSERT INTO associations (interface, handlerId, contentId, defaultAssoc) VALUES ('extractor', 'com.notmarek.kompanion.extractor', 'GL:*.{}', 'true');\n",
            spec.extension
        ));
    }
    out.push_str("COMMIT;\n");
    out
}

pub fn generate_uninstall_sql(specs: &[IndexerSpec]) -> String {
    let exts = quoted_ext_list(specs);
    let mut out = String::from("BEGIN;\n");
    out.push_str(&format!("DELETE FROM mimetypes WHERE ext IN ({exts});\n"));
    out.push_str(&format!("DELETE FROM extenstions WHERE ext IN ({exts});\n\n"));

    out.push_str("-- Remove associations and properties for the launcher\n");
    out.push_str("DELETE FROM associations WHERE handlerId = 'com.notmarek.kompanion.launcher';\n");
    out.push_str("DELETE FROM properties WHERE handlerId = 'com.notmarek.kompanion.launcher';\n");
    out.push_str("DELETE FROM handlerIds WHERE handlerId = 'com.notmarek.kompanion.launcher';\n\n");

    out.push_str("-- Remove associations and properties for the extractor\n");
    out.push_str("DELETE FROM associations WHERE handlerId = 'com.notmarek.kompanion.extractor';\n");
    out.push_str("DELETE FROM properties WHERE handlerId = 'com.notmarek.kompanion.extractor';\n");
    out.push_str("DELETE FROM handlerIds WHERE handlerId = 'com.notmarek.kompanion.extractor';\n");
    out.push_str("COMMIT;\n");
    out
}

fn quoted_ext_list(specs: &[IndexerSpec]) -> String {
    specs
        .iter()
        .map(|s| format!("'{}'", s.extension))
        .collect::<Vec<_>>()
        .join(", ")
}

fn before_tests(src: &str) -> &str {
    src.split("#[cfg(test)]").next().unwrap_or(src)
}

fn extract_string_after(src: &str, marker: &str) -> Option<String> {
    let idx = src.find(marker)?;
    let rest = &src[idx + marker.len()..];
    let start = rest.find('"')?;
    let rest = &rest[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_ends_with_extensions(src: &str) -> Vec<String> {
    const MARKER: &str = "ends_with(\"";
    let mut extensions = Vec::new();
    let mut rest = src;
    while let Some(idx) = rest.find(MARKER) {
        rest = &rest[idx + MARKER.len()..];
        let Some(end) = rest.find('"') else { break };
        let literal = &rest[..end];
        if let Some(ext) = literal.strip_prefix('.') {
            if !ext.is_empty() && !extensions.iter().any(|e| e == ext) {
                extensions.push(ext.to_string());
            }
        }
        rest = &rest[end + 1..];
    }
    extensions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct FixtureDir(std::path::PathBuf);

    impl FixtureDir {
        fn new() -> Self {
            static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "kompanion_build_utils_test_{}_{}",
                std::process::id(),
                id
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for FixtureDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_module(dir: &std::path::Path, name: &str, body: &str) {
        fs::write(dir.join(format!("{name}.rs")), body).unwrap();
    }

    const EPUB_BODY: &str = r#"use kompanion_core::indexer::{FileIndexer, IndexMetadata};

pub struct EpubIndexer;

impl FileIndexer for EpubIndexer {
    fn can_handle(&self, filename: &str) -> bool {
        filename.ends_with(".epub")
    }

    fn mime_type(&self) -> &str {
        "application/epub+zip"
    }
}
"#;

    const CBZ_BODY: &str = r#"use kompanion_core::indexer::{FileIndexer, IndexMetadata};

pub struct CbzIndexer;

impl FileIndexer for CbzIndexer {
    fn can_handle(&self, filename: &str) -> bool {
        filename.ends_with(".cbz")
    }

    fn mime_type(&self) -> &str {
        "application/vnd.comicbook+zip"
    }
}
"#;

    #[test]
    fn parses_extractors_from_indexer_dir() {
        let dir = FixtureDir::new();
        fs::write(
            dir.0.join("mod.rs"),
            "pub mod epub;\npub mod cbz;\nuse std::sync::OnceLock;\n",
        )
        .unwrap();
        write_module(&dir.0, "epub", EPUB_BODY);
        write_module(&dir.0, "cbz", CBZ_BODY);

        let specs = parse_indexer_specs(&dir.0);

        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].extension, "epub");
        assert_eq!(specs[0].mime_type, "application/epub+zip");
        assert_eq!(specs[1].extension, "cbz");
        assert_eq!(specs[1].mime_type, "application/vnd.comicbook+zip");
    }

    #[test]
    fn ignores_test_modules_in_indexer_source() {
        let dir = FixtureDir::new();
        fs::write(dir.0.join("mod.rs"), "pub mod epub;\n").unwrap();
        fs::write(
            dir.0.join("epub.rs"),
            format!(
                "{EPUB_BODY}\n#[cfg(test)]\nmod tests {{\n    #[test]\n    fn x() {{\n        let _ = \"book.pdf\".ends_with(\".pdf\");\n    }}\n}}\n"
            ),
        )
        .unwrap();

        let specs = parse_indexer_specs(&dir.0);

        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].extension, "epub");
    }

    #[test]
    fn install_sql_contains_all_generated_rows() {
        let specs = vec![
            IndexerSpec {
                extension: "epub".into(),
                mime_type: "application/epub+zip".into(),
            },
            IndexerSpec {
                extension: "cbz".into(),
                mime_type: "application/vnd.comicbook+zip".into(),
            },
        ];

        let sql = generate_install_sql(&specs);

        assert!(sql.starts_with("BEGIN;\n"));
        assert!(sql.contains("DELETE FROM mimetypes WHERE ext IN ('epub', 'cbz');"));
        assert!(sql.contains("DELETE FROM extenstions WHERE ext IN ('epub', 'cbz');"));
        assert!(sql.contains(
            "INSERT INTO mimetypes (ext, mimetype) VALUES ('cbz', 'MT:application/vnd.comicbook+zip');"
        ));
        assert!(sql.contains(
            "INSERT INTO extenstions (ext, mimetype) VALUES ('epub', 'MT:application/epub+zip');"
        ));
        assert!(sql.contains("@EXTRACTOR_LIB@"));
        assert!(sql.contains(
            "INSERT INTO associations (interface, handlerId, contentId, defaultAssoc) VALUES ('application', 'com.notmarek.kompanion.launcher', 'MT:application/vnd.comicbook+zip', 'true');"
        ));
        assert!(sql.contains(
            "INSERT INTO associations (interface, handlerId, contentId, defaultAssoc) VALUES ('extractor', 'com.notmarek.kompanion.extractor', 'GL:*.cbz', 'true');"
        ));
        assert!(sql.trim_end().ends_with("COMMIT;"));
    }

    #[test]
    fn uninstall_sql_covers_all_generated_extensions() {
        let specs = vec![
            IndexerSpec {
                extension: "epub".into(),
                mime_type: "application/epub+zip".into(),
            },
            IndexerSpec {
                extension: "cbz".into(),
                mime_type: "application/vnd.comicbook+zip".into(),
            },
        ];

        let sql = generate_uninstall_sql(&specs);

        assert!(sql.contains("DELETE FROM mimetypes WHERE ext IN ('epub', 'cbz');"));
        assert!(sql.contains("DELETE FROM extenstions WHERE ext IN ('epub', 'cbz');"));
        assert!(sql.trim_end().ends_with("COMMIT;"));
    }

    #[test]
    fn parses_real_indexer_registry() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../kompanion_extractor/src/indexer");
        let specs = parse_indexer_specs(&dir);

        assert!(specs
            .iter()
            .any(|s| s.extension == "epub" && s.mime_type == "application/epub+zip"));
        assert!(specs
            .iter()
            .any(|s| s.extension == "cbz" && s.mime_type == "application/vnd.comicbook+zip"));
    }
}
