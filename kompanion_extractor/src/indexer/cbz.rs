use std::{
    collections::HashMap,
    io::{self, BufReader},
    path::Path,
};

use kompanion_core::indexer::{FileIndexer, IndexMetadata};
use std::fs::File;

pub struct CbzIndexer;

impl FileIndexer for CbzIndexer {
    fn can_handle(&self, filename: &str) -> bool {
        filename.ends_with(".cbz")
    }

    fn extract_metadata(&self, full_path: &str) -> Result<IndexMetadata, String> {
        let path = Path::new(full_path);
        Ok(IndexMetadata {
            name: path.file_prefix().map(|s| s.to_string_lossy().into_owned()),
            author: None,
            icon: None,
            extra: HashMap::new(),
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
        "application/vnd.comicbook+zip"
    }
}

fn extract_cover_zip(epub_path: &str, sdr_path: &str) -> Result<Option<String>, String> {
    let mut archive = zip::ZipArchive::new(BufReader::new(
        File::open(epub_path).map_err(|e| format!("{:?}", e))?,
    ))
    .map_err(|e| format!("Could not open {epub_path:?}: {e}"))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("{:?}", e))?;

        if file.is_file() {
            if let Some(path) = file.enclosed_name() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "jpg" | "jpeg" | "png") {
                        println!(
                            "Entry {} is a file with name {:?} ({} bytes)",
                            i,
                            path,
                            file.size()
                        );
                        let cover_file_name = format!("cover.{}", ext);
                        let cover_path = std::path::Path::new(sdr_path).join(&cover_file_name);
                        let mut cover_file = File::create(&cover_path)
                            .map_err(|e| format!("Couldn't create cover file: {e:?}"))?;
                        io::copy(&mut file, &mut cover_file)
                            .map_err(|e| format!("Couldn't extract cover: {e:?}"))?;
                        return Ok(Some(cover_path.to_string_lossy().into_owned()));
                    }
                }
            }
        }
    }
    Ok(None)
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
                "kompanion_cbz_test_{}_{}",
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

    fn create_cbz(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for (name, contents) in entries {
            archive.start_file(name, options).unwrap();
            archive.write_all(contents).unwrap();
        }

        archive.finish().unwrap();
    }

    #[test]
    fn test_cbz_indexer_can_handle_cbz_files() {
        let indexer = CbzIndexer;

        assert!(indexer.can_handle("comic.cbz"));
        assert!(!indexer.can_handle("comic.epub"));
        assert!(!indexer.can_handle("comic.cbz.bak"));
        assert_eq!(indexer.mime_type(), "application/vnd.comicbook+zip");
    }

    #[test]
    fn test_extract_cbz_metadata_from_filename() {
        let indexer = CbzIndexer;
        let metadata = indexer.extract_metadata("/books/My Comic.cbz").unwrap();

        assert_eq!(metadata.name.as_deref(), Some("My Comic"));
        assert_eq!(metadata.author, None);
        assert_eq!(metadata.icon, None);
        assert!(metadata.extra.is_empty());
    }

    #[test]
    fn test_extracts_first_supported_image_to_sdr() {
        let temp_dir = TempDir::new();
        let cbz_path = temp_dir.path().join("comic.cbz");
        let first_image = b"first image";

        create_cbz(
            &cbz_path,
            &[
                ("README.txt", b"not a cover"),
                ("pages/cover.png", first_image),
                ("pages/other.jpg", b"a later image"),
            ],
        );

        let cover = CbzIndexer
            .handle_sdr(
                &cbz_path.to_string_lossy(),
                &IndexMetadata::new(Some("Comic".into()), None, None),
            )
            .unwrap()
            .unwrap();

        assert_eq!(std::fs::read(&cover).unwrap(), first_image);
        assert!(cover.ends_with(".sdr/cover.png"));
    }

    #[test]
    fn test_returns_no_cover_for_archives_without_supported_images() {
        let temp_dir = TempDir::new();
        let cbz_path = temp_dir.path().join("comic.cbz");

        create_cbz(
            &cbz_path,
            &[("pages/page.gif", b"unsupported"), ("README.txt", b"text")],
        );

        let cover = CbzIndexer
            .handle_sdr(
                &cbz_path.to_string_lossy(),
                &IndexMetadata::new(None, None, None),
            )
            .unwrap();

        assert_eq!(cover, None);
        assert!(cbz_path.with_extension("cbz.sdr").is_dir());
    }

    #[test]
    fn test_returns_error_for_missing_cbz() {
        let temp_dir = TempDir::new();
        let cbz_path = temp_dir.path().join("missing.cbz");

        let result = CbzIndexer.handle_sdr(
            &cbz_path.to_string_lossy(),
            &IndexMetadata::new(None, None, None),
        );

        assert!(result.is_err());
    }
}
