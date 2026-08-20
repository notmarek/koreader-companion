use std::{
    collections::HashMap,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use base64::Engine;
use fb2::{Author, FictionBook, TitleInfo};
use kompanion_core::indexer::{FileIndexer, IndexMetadata};
use std::fs::File;

pub struct Fb2Indexer;

impl FileIndexer for Fb2Indexer {
    fn can_handle(&self, filename: &str) -> bool {
        filename.ends_with(".fb2") || filename.ends_with(".fbz") || filename.ends_with(".fb2.zip")
    }

    fn extract_metadata(&self, full_path: &str) -> Result<IndexMetadata, String> {
        let book = parse_book(full_path)?;
        let info = &book.description.title_info;

        let name = non_empty(&info.book_title.value);

        let author = info.authors.iter().find_map(author_name);

        let mut extra = HashMap::new();
        if !info.lang.trim().is_empty() {
            extra.insert("language".to_string(), info.lang.trim().to_string());
        }
        if let Some(series) = info.sequences.iter().find_map(|s| non_empty_opt(&s.name)) {
            extra.insert("series".to_string(), series);
        }

        Ok(IndexMetadata {
            name,
            author,
            icon: None,
            extra,
        })
    }

    fn handle_sdr(
        &self,
        full_path: &str,
        _metadata: &IndexMetadata,
    ) -> Result<Option<String>, String> {
        let sdr_path = format!("{}.sdr", full_path);
        std::fs::create_dir_all(&sdr_path).map_err(|e| format!("Failed to create SDR: {}", e))?;

        let book = parse_book(full_path)?;
        extract_cover(&book, &sdr_path)
    }

    fn mime_type(&self) -> &str {
        "application/fictionbook2+zip"
    }

    fn cde_type(&self) -> &str {
        "EBOK"
    }
}

/// Parse an FB2 book from either a raw `.fb2` file or a zipped `.fbz` / `.fb2.zip`.
fn parse_book(full_path: &str) -> Result<FictionBook, String> {
    let mut file =
        File::open(full_path).map_err(|e| format!("Failed to open {full_path:?}: {e}"))?;

    // FB2 files may be zip containers regardless of extension (a bare `.fb2` can
    // itself be a zip), so detect by content rather than by filename.
    if is_zip(&mut file)? {
        parse_zipped(file)
    } else {
        quick_xml::de::from_reader(BufReader::new(file))
            .map_err(|e| format!("Failed to parse FB2: {e}"))
    }
}

/// Sniff the leading bytes for the zip local-file-header magic (`PK\x03\x04`),
/// rewinding the file to the start afterwards so the caller can read from the
/// beginning.
fn is_zip(file: &mut File) -> Result<bool, String> {
    let mut magic = [0u8; 4];
    let read = read_up_to(file, &mut magic).map_err(|e| format!("Failed to read header: {e}"))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|e| format!("Failed to rewind: {e}"))?;
    Ok(read >= 4 && &magic == b"PK\x03\x04")
}

/// Read into `buf` until it is full or EOF, returning the number of bytes read.
/// A single `Read::read` may return fewer bytes than requested.
fn read_up_to(reader: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

fn parse_zipped(file: File) -> Result<FictionBook, String> {
    let mut archive =
        zip::ZipArchive::new(BufReader::new(file)).map_err(|e| format!("Could not open zip: {e}"))?;

    // Prefer an entry with an `.fb2` extension; fall back to the first file entry
    // for archives that store the book under a different name.
    let mut target = None;
    let mut first_file = None;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| format!("{e:?}"))?;
        if !entry.is_file() {
            continue;
        }
        if first_file.is_none() {
            first_file = Some(i);
        }
        if let Some(path) = entry.enclosed_name() {
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("fb2"))
                .unwrap_or(false)
            {
                target = Some(i);
                break;
            }
        }
    }

    let index = target
        .or(first_file)
        .ok_or_else(|| "No file entry found in archive".to_string())?;
    let entry = archive.by_index(index).map_err(|e| format!("{e:?}"))?;
    quick_xml::de::from_reader(BufReader::new(entry))
        .map_err(|e| format!("Failed to parse FB2: {e}"))
}

fn extract_cover(book: &FictionBook, sdr_path: &str) -> Result<Option<String>, String> {
    let href = match cover_href(&book.description.title_info) {
        Some(h) => h,
        None => {
            log::debug!("No cover page found in FB2");
            return Ok(None);
        }
    };

    // xlink hrefs reference an inline binary by id, e.g. "#cover.jpg".
    let id = href.trim_start_matches('#');
    let binary = match book.binaries.iter().find(|b| b.id == id) {
        Some(b) => b,
        None => {
            log::debug!("Cover reference {href:?} has no matching binary");
            return Ok(None);
        }
    };

    // FB2 wraps the base64 payload across many lines; the standard engine rejects
    // embedded whitespace, so strip all of it before decoding.
    let cleaned: String = binary.content.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|e| format!("Failed to decode cover image: {e}"))?;

    let extension = image_extension(&binary.content_type, id);
    let cover_file_name = format!("cover.{}", extension);
    let cover_path = Path::new(sdr_path).join(&cover_file_name);

    std::fs::write(&cover_path, &bytes).map_err(|e| format!("Failed to write cover: {e}"))?;

    Ok(Some(cover_path.to_string_lossy().into_owned()))
}

fn cover_href(info: &TitleInfo) -> Option<String> {
    info.cover_page
        .as_ref()?
        .images
        .iter()
        .find_map(|img| img.href.clone())
}

fn image_extension(content_type: &str, href_id: &str) -> String {
    match content_type.to_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "jpg".to_string(),
        "image/png" => "png".to_string(),
        "image/gif" => "gif".to_string(),
        _ => href_id
            .rsplit('.')
            .next()
            .filter(|e| !e.is_empty() && *e != href_id)
            .map(|e| e.to_lowercase())
            .unwrap_or_else(|| "jpg".to_string()),
    }
}

fn author_name(author: &Author) -> Option<String> {
    match author {
        Author::Verbose(v) => {
            let mut parts = Vec::new();
            if let Some(first) = non_empty(&v.first_name.value) {
                parts.push(first);
            }
            if let Some(middle) = v.middle_name.as_ref().and_then(|m| non_empty(&m.value)) {
                parts.push(middle);
            }
            if let Some(last) = non_empty(&v.last_name.value) {
                parts.push(last);
            }
            if parts.is_empty() {
                v.nickname.as_ref().and_then(|n| non_empty(&n.value))
            } else {
                Some(parts.join(" "))
            }
        }
        Author::Anonymous(a) => a.nickname.as_ref().and_then(|n| non_empty(&n.value)),
    }
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn non_empty_opt(value: &Option<String>) -> Option<String> {
    value.as_deref().and_then(non_empty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "kompanion_fb2_test_{}_{}",
                std::process::id(),
                id
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // A 14-byte minimal JPEG, base64-encoded, used as the cover binary payload.
    const COVER_B64: &str = "/9j/4AAQSkZJRgAB/9k=";

    fn cover_bytes() -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(COVER_B64)
            .unwrap()
    }

    fn sample_fb2() -> String {
        format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0" xmlns:l="http://www.w3.org/1999/xlink">
  <description>
    <title-info>
      <genre>sf</genre>
      <author>
        <first-name>Isaac</first-name>
        <last-name>Asimov</last-name>
      </author>
      <book-title>Foundation</book-title>
      <lang>en</lang>
      <coverpage>
        <image l:href="#cover.jpg"/>
      </coverpage>
    </title-info>
    <document-info>
      <author><nickname>uploader</nickname></author>
      <date>2020-01-01</date>
      <id>doc-id</id>
      <version>1.0</version>
    </document-info>
  </description>
  <body>
    <section><p>Hello.</p></section>
  </body>
  <binary id="cover.jpg" content-type="image/jpeg">{COVER_B64}</binary>
</FictionBook>"##
        )
    }

    fn sample_fb2_no_cover() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0" xmlns:l="http://www.w3.org/1999/xlink">
  <description>
    <title-info>
      <genre>sf</genre>
      <author>
        <first-name>Isaac</first-name>
        <last-name>Asimov</last-name>
      </author>
      <book-title>Foundation</book-title>
      <lang>en</lang>
    </title-info>
    <document-info>
      <author><nickname>uploader</nickname></author>
      <date>2020-01-01</date>
      <id>doc-id</id>
      <version>1.0</version>
    </document-info>
  </description>
  <body>
    <section><p>Hello.</p></section>
  </body>
</FictionBook>"#
            .to_string()
    }

    fn write_file(path: &Path, contents: &[u8]) {
        let mut f = File::create(path).unwrap();
        f.write_all(contents).unwrap();
    }

    fn write_fbz(path: &Path, fb2_name: &str, xml: &str) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file(fb2_name, options).unwrap();
        zip.write_all(xml.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn test_fb2_indexer_can_handle() {
        let indexer = Fb2Indexer;
        assert!(indexer.can_handle("book.fb2"));
        assert!(indexer.can_handle("book.fbz"));
        assert!(indexer.can_handle("book.fb2.zip"));
        assert!(!indexer.can_handle("book.epub"));
        assert!(!indexer.can_handle("book.sh"));
        assert_eq!(indexer.mime_type(), "application/fictionbook2+zip");
    }

    #[test]
    fn test_extract_metadata_from_raw_fb2() {
        let dir = TempDir::new();
        let path = dir.path().join("book.fb2");
        write_file(&path, sample_fb2().as_bytes());

        let metadata = Fb2Indexer
            .extract_metadata(&path.to_string_lossy())
            .unwrap();

        assert_eq!(metadata.name.as_deref(), Some("Foundation"));
        assert_eq!(metadata.author.as_deref(), Some("Isaac Asimov"));
        assert_eq!(metadata.extra.get("language").map(String::as_str), Some("en"));
        assert_eq!(metadata.icon, None);
    }

    #[test]
    fn test_extract_cover_from_raw_fb2() {
        let dir = TempDir::new();
        let path = dir.path().join("book.fb2");
        write_file(&path, sample_fb2().as_bytes());

        let cover = Fb2Indexer
            .handle_sdr(
                &path.to_string_lossy(),
                &IndexMetadata::new(None, None, None),
            )
            .unwrap()
            .unwrap();

        assert!(cover.ends_with(".sdr/cover.jpg"));
        assert_eq!(std::fs::read(&cover).unwrap(), cover_bytes());
    }

    #[test]
    fn test_extract_metadata_and_cover_from_fbz() {
        let dir = TempDir::new();
        let path = dir.path().join("book.fbz");
        write_fbz(&path, "book.fb2", &sample_fb2());

        let metadata = Fb2Indexer
            .extract_metadata(&path.to_string_lossy())
            .unwrap();
        assert_eq!(metadata.name.as_deref(), Some("Foundation"));
        assert_eq!(metadata.author.as_deref(), Some("Isaac Asimov"));

        let cover = Fb2Indexer
            .handle_sdr(
                &path.to_string_lossy(),
                &IndexMetadata::new(None, None, None),
            )
            .unwrap()
            .unwrap();
        assert!(cover.ends_with(".sdr/cover.jpg"));
        assert_eq!(std::fs::read(&cover).unwrap(), cover_bytes());
    }

    #[test]
    fn test_bare_fb2_extension_that_is_actually_a_zip() {
        // A `.fb2` file may itself be a zip container; detection is by content.
        let dir = TempDir::new();
        let path = dir.path().join("book.fb2");
        write_fbz(&path, "book.fb2", &sample_fb2());

        let metadata = Fb2Indexer
            .extract_metadata(&path.to_string_lossy())
            .unwrap();
        assert_eq!(metadata.name.as_deref(), Some("Foundation"));
        assert_eq!(metadata.author.as_deref(), Some("Isaac Asimov"));

        let cover = Fb2Indexer
            .handle_sdr(
                &path.to_string_lossy(),
                &IndexMetadata::new(None, None, None),
            )
            .unwrap()
            .unwrap();
        assert!(cover.ends_with(".sdr/cover.jpg"));
        assert_eq!(std::fs::read(&cover).unwrap(), cover_bytes());
    }

    #[test]
    fn test_cover_with_line_wrapped_base64() {
        // FB2 typically wraps the binary payload across multiple lines.
        let wrapped = "/9j/4AAQSkZJRgAB\n/9k=\n";
        let xml = format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0" xmlns:l="http://www.w3.org/1999/xlink">
  <description>
    <title-info>
      <genre>sf</genre>
      <author><first-name>Isaac</first-name><last-name>Asimov</last-name></author>
      <book-title>Foundation</book-title>
      <lang>en</lang>
      <coverpage><image l:href="#cover.jpg"/></coverpage>
    </title-info>
    <document-info><author><nickname>u</nickname></author><date>2020</date><id>d</id><version>1.0</version></document-info>
  </description>
  <body><section><p>Hi.</p></section></body>
  <binary id="cover.jpg" content-type="image/jpeg">{wrapped}</binary>
</FictionBook>"##
        );

        let dir = TempDir::new();
        let path = dir.path().join("wrapped.fb2");
        write_file(&path, xml.as_bytes());

        let cover = Fb2Indexer
            .handle_sdr(
                &path.to_string_lossy(),
                &IndexMetadata::new(None, None, None),
            )
            .unwrap()
            .unwrap();

        assert_eq!(std::fs::read(&cover).unwrap(), cover_bytes());
    }

    #[test]
    fn test_no_cover_returns_none_but_creates_sdr() {
        let dir = TempDir::new();
        let path = dir.path().join("book.fb2");
        write_file(&path, sample_fb2_no_cover().as_bytes());

        let cover = Fb2Indexer
            .handle_sdr(
                &path.to_string_lossy(),
                &IndexMetadata::new(None, None, None),
            )
            .unwrap();

        assert_eq!(cover, None);
        assert!(path.with_extension("fb2.sdr").is_dir());
    }

    #[test]
    fn test_missing_file_is_error() {
        let dir = TempDir::new();
        let path = dir.path().join("missing.fb2");
        let result = Fb2Indexer.extract_metadata(&path.to_string_lossy());
        assert!(result.is_err());
    }
}
