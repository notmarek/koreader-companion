pub mod epub;
pub mod cbz;

use std::sync::OnceLock;

use kompanion_core::indexer::FileIndexer;

static REGISTRY: OnceLock<Vec<Box<dyn FileIndexer>>> = OnceLock::new();

pub fn init_registry() {
    REGISTRY.set(vec![Box::new(epub::EpubIndexer), Box::new(cbz::CbzIndexer)]).ok();
}

pub fn find_indexer(filename: &str) -> Option<&dyn FileIndexer> {
    REGISTRY
        .get()?
        .iter()
        .find(|i| i.can_handle(filename))
        .map(|b| b.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finds_correct_indexer() {
        init_registry();
        assert!(find_indexer("script.sh").is_none());
        assert!(find_indexer("book.epub").is_some());
        assert!(find_indexer("book.pdf").is_none());
        assert!(find_indexer("image.png").is_none());
    }

    #[test]
    fn test_epub_indexer_handles_epub() {
        init_registry();
        let idx = find_indexer("book.epub").unwrap();
        assert!(idx.can_handle("book.epub"));
        assert!(!idx.can_handle("book.sh"));
        assert_eq!(idx.mime_type(), "application/epub+zip");
    }
}
