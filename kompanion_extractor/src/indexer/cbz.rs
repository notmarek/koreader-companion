use std::{
    cmp::Ordering,
    collections::HashMap,
    io::{self, BufReader},
    path::Path,
};

use kompanion_core::indexer::{FileIndexer, IndexMetadata};
use std::fs::File;

/// Image extensions recognised as comic pages (matched case-insensitively).
const IMAGE_EXTS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "gif"];

pub struct CbzIndexer;

impl FileIndexer for CbzIndexer {
    fn can_handle(&self, filename: &str) -> bool {
        filename.ends_with(".cbz")
    }

    fn extract_metadata(&self, full_path: &str) -> Result<IndexMetadata, String> {
        let path = Path::new(full_path);
        let filename_name = path.file_prefix().map(|s| s.to_string_lossy().into_owned());

        // A plain CBZ carries no metadata; a ComicInfo.xml (ComicRack spec) may.
        // Fall back to the filename on any failure to open/parse the archive.
        let info = open_archive(full_path)
            .ok()
            .and_then(|mut archive| read_comic_info(&mut archive));

        let Some(info) = info else {
            return Ok(IndexMetadata {
                name: filename_name,
                author: None,
                icon: None,
                extra: HashMap::new(),
            });
        };

        // Prefer the story Title, then "Series #Number", then the filename.
        let name = non_empty_opt(&info.title)
            .or_else(|| {
                let series = non_empty_opt(&info.series)?;
                Some(match non_empty_opt(&info.number) {
                    Some(number) => format!("{series} #{number}"),
                    None => series,
                })
            })
            .or(filename_name);

        // Writer may list several creators comma-separated; take the first.
        let author = non_empty_opt(&info.writer).and_then(|writer| {
            writer
                .split(',')
                .map(str::trim)
                .find(|s| !s.is_empty())
                .map(str::to_string)
        });

        let mut extra = HashMap::new();
        if let Some(series) = non_empty_opt(&info.series) {
            extra.insert("series".to_string(), series);
        }
        if let Some(language) = non_empty_opt(&info.language_iso) {
            extra.insert("language".to_string(), language);
        }
        if let Some(publisher) = non_empty_opt(&info.publisher) {
            extra.insert("publisher".to_string(), publisher);
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

    fn cde_type(&self) -> &str {
        "EBOK"
    }

    fn mime_type(&self) -> &str {
        "application/vnd.comicbook+zip"
    }
}

/// A CBZ has no field naming the cover. The convention shared by comic readers
/// is: sort the image entries by filename and take the first — that is page 1.
/// A ComicInfo.xml, when present, may instead mark a page as `FrontCover`, which
/// overrides the positional guess.
fn extract_cover_zip(cbz_path: &str, sdr_path: &str) -> Result<Option<String>, String> {
    let mut archive = open_archive(cbz_path)?;

    // Collect image entries as (zip index, full path, lowercased extension).
    // `by_index` borrows the archive mutably, so gather metadata first and
    // re-open the chosen entry afterwards to copy it out.
    let mut images: Vec<(usize, String, String)> = Vec::new();
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| format!("{:?}", e))?;
        if !file.is_file() {
            continue;
        }
        let Some(path) = file.enclosed_name() else {
            continue;
        };
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if IMAGE_EXTS.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
            images.push((i, path.to_string_lossy().into_owned(), ext.to_ascii_lowercase()));
        }
    }

    if images.is_empty() {
        return Ok(None);
    }

    images.sort_by(|a, b| natural_cmp(&a.1, &b.1));

    // Default cover is the first page in natural order.
    let mut chosen = 0usize;
    if let Some(info) = read_comic_info(&mut archive) {
        if let Some(front) = front_cover_index(&info) {
            if (front as usize) < images.len() {
                chosen = front as usize;
            } else {
                log::debug!(
                    "ComicInfo FrontCover index {front} out of bounds ({} images)",
                    images.len()
                );
            }
        }
    }

    let zip_index = images[chosen].0;
    let ext = images[chosen].2.clone();

    let mut file = archive.by_index(zip_index).map_err(|e| format!("{:?}", e))?;
    let cover_file_name = format!("cover.{}", ext);
    let cover_path = std::path::Path::new(sdr_path).join(&cover_file_name);
    let mut cover_file =
        File::create(&cover_path).map_err(|e| format!("Couldn't create cover file: {e:?}"))?;
    io::copy(&mut file, &mut cover_file).map_err(|e| format!("Couldn't extract cover: {e:?}"))?;
    Ok(Some(cover_path.to_string_lossy().into_owned()))
}

fn open_archive(cbz_path: &str) -> Result<zip::ZipArchive<BufReader<File>>, String> {
    zip::ZipArchive::new(BufReader::new(
        File::open(cbz_path).map_err(|e| format!("{:?}", e))?,
    ))
    .map_err(|e| format!("Could not open {cbz_path:?}: {e}"))
}

/// Locate and parse a `ComicInfo.xml` entry (matched case-insensitively, in any
/// folder). Returns `None` when absent or unparseable — it is optional metadata.
fn read_comic_info(archive: &mut zip::ZipArchive<BufReader<File>>) -> Option<ComicInfo> {
    let mut found = None;
    for i in 0..archive.len() {
        let file = archive.by_index(i).ok()?;
        if !file.is_file() {
            continue;
        }
        let is_match = file
            .enclosed_name()
            .and_then(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
            .map(|n| n.eq_ignore_ascii_case("ComicInfo.xml"))
            .unwrap_or(false);
        if is_match {
            found = Some(i);
            break;
        }
    }

    let entry = archive.by_index(found?).ok()?;
    match quick_xml::de::from_reader(BufReader::new(entry)) {
        Ok(info) => Some(info),
        Err(e) => {
            log::debug!("Failed to parse ComicInfo.xml: {e}");
            None
        }
    }
}

/// The `Image` index (into the sorted page list) of the first page whose `Type`
/// is `FrontCover`, if any.
fn front_cover_index(info: &ComicInfo) -> Option<u32> {
    info.pages.as_ref()?.page.iter().find_map(|page| {
        let is_front = page
            .kind
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case("FrontCover"))
            .unwrap_or(false);
        if is_front {
            page.image
        } else {
            None
        }
    })
}

/// Case-insensitive, numeric-aware comparison so `page2.jpg` sorts before
/// `page10.jpg` (plain lexical ordering would reverse them).
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) if ca.is_ascii_digit() && cb.is_ascii_digit() => {
                let na: String = take_digits(&mut ai);
                let nb: String = take_digits(&mut bi);
                // Compare by numeric value: fewer significant digits = smaller.
                let ta = na.trim_start_matches('0');
                let tb = nb.trim_start_matches('0');
                let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if ord != Ordering::Equal {
                    return ord;
                }
                // Equal value: shorter (fewer leading zeros) sorts first, stably.
                let ord = na.len().cmp(&nb.len());
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (Some(ca), Some(cb)) => {
                let la = ca.to_ascii_lowercase();
                let lb = cb.to_ascii_lowercase();
                if la != lb {
                    return la.cmp(&lb);
                }
                ai.next();
                bi.next();
            }
        }
    }
}

fn take_digits(iter: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut out = String::new();
    while let Some(&c) = iter.peek() {
        if c.is_ascii_digit() {
            out.push(c);
            iter.next();
        } else {
            break;
        }
    }
    out
}

fn non_empty_opt(value: &Option<String>) -> Option<String> {
    value.as_deref().and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// Subset of the ComicInfo.xml (ComicRack) schema we consume. Unknown elements
/// are ignored; all fields are optional.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ComicInfo {
    title: Option<String>,
    series: Option<String>,
    number: Option<String>,
    writer: Option<String>,
    #[serde(rename = "LanguageISO")]
    language_iso: Option<String>,
    publisher: Option<String>,
    pages: Option<ComicPages>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct ComicPages {
    #[serde(rename = "Page", default)]
    page: Vec<ComicPage>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct ComicPage {
    /// Zero-based index into the sorted page list.
    #[serde(rename = "@Image")]
    image: Option<u32>,
    #[serde(rename = "@Type")]
    kind: Option<String>,
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
            &[("pages/page.bmp", b"unsupported"), ("README.txt", b"text")],
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

    #[test]
    fn test_natural_sort_picks_first_page_regardless_of_zip_order() {
        // Pages stored out of order in the archive; page 1 (0001) must win even
        // though 0010 is the first physical entry.
        let temp_dir = TempDir::new();
        let cbz_path = temp_dir.path().join("comic.cbz");
        let page_one = b"page one";

        create_cbz(
            &cbz_path,
            &[
                ("0010.jpg", b"page ten"),
                ("0001.jpg", page_one),
                ("0002.jpg", b"page two"),
            ],
        );

        let cover = CbzIndexer
            .handle_sdr(
                &cbz_path.to_string_lossy(),
                &IndexMetadata::new(None, None, None),
            )
            .unwrap()
            .unwrap();

        assert_eq!(std::fs::read(&cover).unwrap(), page_one);
        assert!(cover.ends_with(".sdr/cover.jpg"));
    }

    #[test]
    fn test_case_insensitive_extension_and_webp() {
        let temp_dir = TempDir::new();
        let cbz_path = temp_dir.path().join("comic.cbz");
        let image = b"webp cover";

        create_cbz(&cbz_path, &[("Cover.WEBP", image)]);

        let cover = CbzIndexer
            .handle_sdr(
                &cbz_path.to_string_lossy(),
                &IndexMetadata::new(None, None, None),
            )
            .unwrap()
            .unwrap();

        assert_eq!(std::fs::read(&cover).unwrap(), image);
        assert!(cover.ends_with(".sdr/cover.webp"));
    }

    #[test]
    fn test_comicinfo_frontcover_overrides_sort_order() {
        let temp_dir = TempDir::new();
        let cbz_path = temp_dir.path().join("comic.cbz");
        let third = b"the real cover";
        let comic_info = br#"<?xml version="1.0"?>
<ComicInfo>
  <Pages>
    <Page Image="0" Type="Story"/>
    <Page Image="2" Type="FrontCover"/>
  </Pages>
</ComicInfo>"#;

        create_cbz(
            &cbz_path,
            &[
                ("01.jpg", b"page one"),
                ("02.jpg", b"page two"),
                ("03.jpg", third),
                ("ComicInfo.xml", comic_info),
            ],
        );

        let cover = CbzIndexer
            .handle_sdr(
                &cbz_path.to_string_lossy(),
                &IndexMetadata::new(None, None, None),
            )
            .unwrap()
            .unwrap();

        assert_eq!(std::fs::read(&cover).unwrap(), third);
    }

    #[test]
    fn test_comicinfo_metadata_is_used() {
        let temp_dir = TempDir::new();
        let cbz_path = temp_dir.path().join("filename-title.cbz");
        let comic_info = br#"<?xml version="1.0"?>
<ComicInfo>
  <Title>Chapter One</Title>
  <Series>Amazing Comics</Series>
  <Number>3</Number>
  <Writer>Alan Moore, Dave Gibbons</Writer>
  <LanguageISO>en</LanguageISO>
  <Publisher>Example Press</Publisher>
</ComicInfo>"#;

        create_cbz(
            &cbz_path,
            &[("01.jpg", b"page one"), ("ComicInfo.xml", comic_info)],
        );

        let metadata = CbzIndexer
            .extract_metadata(&cbz_path.to_string_lossy())
            .unwrap();

        assert_eq!(metadata.name.as_deref(), Some("Chapter One"));
        assert_eq!(metadata.author.as_deref(), Some("Alan Moore"));
        assert_eq!(metadata.extra.get("series").map(String::as_str), Some("Amazing Comics"));
        assert_eq!(metadata.extra.get("language").map(String::as_str), Some("en"));
        assert_eq!(
            metadata.extra.get("publisher").map(String::as_str),
            Some("Example Press")
        );
    }

    #[test]
    fn test_comicinfo_series_number_fallback_for_name() {
        let temp_dir = TempDir::new();
        let cbz_path = temp_dir.path().join("comic.cbz");
        let comic_info = br#"<?xml version="1.0"?>
<ComicInfo>
  <Series>Amazing Comics</Series>
  <Number>3</Number>
</ComicInfo>"#;

        create_cbz(
            &cbz_path,
            &[("01.jpg", b"page one"), ("ComicInfo.xml", comic_info)],
        );

        let metadata = CbzIndexer
            .extract_metadata(&cbz_path.to_string_lossy())
            .unwrap();

        assert_eq!(metadata.name.as_deref(), Some("Amazing Comics #3"));
    }

    #[test]
    fn test_malformed_comicinfo_falls_back() {
        let temp_dir = TempDir::new();
        let cbz_path = temp_dir.path().join("My Comic.cbz");
        let page_one = b"page one";

        create_cbz(
            &cbz_path,
            &[
                ("0010.jpg", b"page ten"),
                ("0001.jpg", page_one),
                ("ComicInfo.xml", b"this is not xml <<<"),
            ],
        );

        // Metadata falls back to the filename.
        let metadata = CbzIndexer
            .extract_metadata(&cbz_path.to_string_lossy())
            .unwrap();
        assert_eq!(metadata.name.as_deref(), Some("My Comic"));
        assert!(metadata.extra.is_empty());

        // Cover falls back to the natural-sort first page.
        let cover = CbzIndexer
            .handle_sdr(
                &cbz_path.to_string_lossy(),
                &IndexMetadata::new(None, None, None),
            )
            .unwrap()
            .unwrap();
        assert_eq!(std::fs::read(&cover).unwrap(), page_one);
    }
}
