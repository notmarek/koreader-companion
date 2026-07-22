use std::collections::HashMap;

use crate::extractor_log;
use epub_stream::book::EpubBookBuilder;
use kompanion_core::indexer::{FileIndexer, IndexMetadata};

pub struct EpubIndexer;

impl FileIndexer for EpubIndexer {
    fn can_handle(&self, filename: &str) -> bool {
        filename.ends_with(".epub")
    }

    fn extract_metadata(&self, full_path: &str) -> Result<IndexMetadata, String> {
        let summary = EpubBookBuilder::new()
            .parse_file(full_path)
            .map_err(|e| format!("Failed to parse EPUB: {}", e))?;

        let meta = summary.metadata();

        let name = if meta.title.is_empty() {
            None
        } else {
            Some(meta.title.clone())
        };

        let author = if meta.author.is_empty() {
            None
        } else {
            Some(meta.author.clone())
        };

        let mut extra = HashMap::new();

        if let Some(ref desc) = meta.description {
            if !desc.is_empty() {
                extra.insert("description".to_string(), desc.clone());
            }
        }
        if let Some(ref pub_) = meta.publisher {
            if !pub_.is_empty() {
                extra.insert("publisher".to_string(), pub_.clone());
            }
        }
        if let Some(ref ident) = meta.identifier {
            if !ident.is_empty() {
                extra.insert("identifier".to_string(), ident.clone());
            }
        }
        if let Some(ref date) = meta.date {
            if !date.is_empty() {
                extra.insert("publicationDate".to_string(), date.clone());
            }
        }
        if let Some(ref rights) = meta.rights {
            if !rights.is_empty() {
                extra.insert("rights".to_string(), rights.clone());
            }
        }
        if !meta.language.is_empty() {
            extra.insert("language".to_string(), meta.language.clone());
        }
        if !meta.subjects.is_empty() {
            extra.insert("subjects".to_string(), meta.subjects.join(", "));
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

        let cover = extract_cover_zip(full_path, &sdr_path)?;

        Ok(cover)
    }

    fn mime_type(&self) -> &str {
        "application/epub+zip"
    }
}

fn extract_cover_zip(epub_path: &str, sdr_path: &str) -> Result<Option<String>, String> {
    use epub_stream::book::EpubBook;

    let mut book = EpubBook::open(epub_path).map_err(|e| format!("Failed to open EPUB: {}", e))?;

    let mut buf = Vec::new();
    let cover_ref = match book
        .read_cover_image_into(&mut buf)
        .map_err(|e| format!("Failed to read cover: {}", e))?
    {
        Some(r) => r,
        None => {
            extractor_log!("No cover image found in EPUB");
            return Ok(None);
        }
    };

    let extension = cover_ref
        .href
        .rsplit('.')
        .next()
        .unwrap_or("jpg")
        .to_lowercase();

    let cover_file_name = format!("cover.{}", extension);
    let cover_path = std::path::Path::new(sdr_path).join(&cover_file_name);

    std::fs::write(&cover_path, &buf).map_err(|e| format!("Failed to write cover: {}", e))?;

    Ok(Some(cover_path.to_string_lossy().into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_minimal_epub(path: &str) {
        use std::io::Write;

        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip.start_file("mimetype", options).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        let container_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;

        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(container_xml.as_bytes()).unwrap();

        let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>A Test Book</dc:title>
    <dc:creator opf:role="aut">Test Author</dc:creator>
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:uuid:12345678-1234-1234-1234-123456789012</dc:identifier>
    <meta name="cover" content="cover-image"/>
  </metadata>
  <manifest>
    <item id="cover-image" href="images/cover.jpg" media-type="image/jpeg"/>
  </manifest>
  <spine/>
</package>"#;

        zip.start_file("content.opf", options).unwrap();
        zip.write_all(opf.as_bytes()).unwrap();

        let jpeg_data = &[
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0xFF, 0xD9,
        ];

        zip.start_file("images/cover.jpg", options).unwrap();
        zip.write_all(jpeg_data).unwrap();

        zip.finish().unwrap();
    }

    #[test]
    fn test_extract_epub_metadata() {
        let tmp = std::env::temp_dir().join(format!("epub_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).ok();
        let epub_path = tmp.join("test.epub");
        create_minimal_epub(&epub_path.to_string_lossy());

        let indexer = EpubIndexer;
        let metadata = indexer
            .extract_metadata(&epub_path.to_string_lossy())
            .unwrap();

        assert_eq!(metadata.name.as_deref(), Some("A Test Book"));
        assert_eq!(metadata.author.as_deref(), Some("Test Author"));
        assert_eq!(metadata.extra.get("language").unwrap(), "en");
        assert!(metadata.extra.contains_key("identifier"));
    }

    #[test]
    fn test_extract_epub_cover() {
        let tmp = std::env::temp_dir().join(format!("epub_cover_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).ok();
        let epub_path = tmp.join("test_cover.epub");
        create_minimal_epub(&epub_path.to_string_lossy());

        let indexer = EpubIndexer;
        let metadata = IndexMetadata::new(Some("Title".into()), Some("Author".into()), None);
        let cover = indexer
            .handle_sdr(&epub_path.to_string_lossy(), &metadata)
            .unwrap();

        assert!(cover.is_some());
        let cover_path = cover.unwrap();
        assert!(std::path::Path::new(&cover_path).exists());
        assert!(cover_path.ends_with(".jpg"));
        assert!(cover_path.contains(".sdr/cover."));
    }

    #[test]
    fn test_extract_cover_from_oebps_subdirectory() {
        use std::io::Write;

        let tmp = std::env::temp_dir().join(format!("epub_oebps_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).ok();
        let epub_path = tmp.join("oebps_cover.epub");

        let file = std::fs::File::create(&epub_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip.start_file("mimetype", options).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        let container_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(container_xml.as_bytes()).unwrap();

        let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>OEBPS Test</dc:title>
    <dc:creator opf:role="aut">OEBPS Author</dc:creator>
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:uuid:oebps-test-id</dc:identifier>
    <meta name="cover" content="cover-image"/>
  </metadata>
  <manifest>
    <item id="cover-image" href="Images/cover.jpeg" media-type="image/jpeg"/>
  </manifest>
  <spine/>
</package>"#;
        zip.start_file("OEBPS/content.opf", options).unwrap();
        zip.write_all(opf.as_bytes()).unwrap();

        let jpeg_data = &[
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0xFF, 0xD9,
        ];
        zip.start_file("OEBPS/Images/cover.jpeg", options).unwrap();
        zip.write_all(jpeg_data).unwrap();

        zip.finish().unwrap();

        let indexer = EpubIndexer;
        let metadata = IndexMetadata::new(Some("Title".into()), Some("Author".into()), None);
        let cover = indexer
            .handle_sdr(&epub_path.to_string_lossy(), &metadata)
            .unwrap();

        assert!(cover.is_some());
        let cover_path = cover.unwrap();
        assert!(std::path::Path::new(&cover_path).exists());
        assert!(cover_path.ends_with(".jpeg"));
        assert!(cover_path.contains(".sdr/cover."));
    }

    #[test]
    fn test_epub_indexer_can_handle() {
        let indexer = EpubIndexer;
        assert!(indexer.can_handle("book.epub"));
        assert!(!indexer.can_handle("book.sh"));
        assert!(!indexer.can_handle("book.pdf"));
        assert_eq!(indexer.mime_type(), "application/epub+zip");
    }
}
