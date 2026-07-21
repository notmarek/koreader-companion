pub mod base64;
pub mod change_request;
pub mod indexer;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScannerEventType {
    Add = 0,
    Delete = 1,
    Update = 2,
    AddThumb = 3,
    UpdateThumb = 4,
}

#[derive(Debug, Clone)]
pub struct ScannerEvent {
    pub event_type: ScannerEventType,
    pub path: String,
    pub filename: String,
    pub uuid: String,
}
