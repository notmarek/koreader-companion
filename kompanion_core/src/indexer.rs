use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct IndexMetadata {
    pub name: Option<String>,
    pub author: Option<String>,
    pub icon: Option<String>,
    pub extra: HashMap<String, String>,
}

impl IndexMetadata {
    pub fn new(
        name: Option<String>,
        author: Option<String>,
        icon: Option<String>,
    ) -> Self {
        Self {
            name,
            author,
            icon,
            extra: HashMap::new(),
        }
    }
}

pub trait FileIndexer: Send + Sync {
    fn can_handle(&self, filename: &str) -> bool;

    fn extract_metadata(&self, full_path: &str) -> Result<IndexMetadata, String>;

    fn handle_sdr(
        &self,
        full_path: &str,
        metadata: &IndexMetadata,
    ) -> Result<Option<String>, String>;

    fn mime_type(&self) -> &str;

    /// CCat content type used for the library entry. Books (EPUB, FB2) should
    /// map to "EBOK"; anything else defaults to "PDOC" (personal document).
    fn cde_type(&self) -> &str {
        "PDOC"
    }

    fn supports_hooks(&self) -> bool {
        false
    }

    fn on_install(&self, _full_path: &str) {}

    fn on_remove(&self, _full_path: &str) {}
}
