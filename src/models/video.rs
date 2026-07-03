use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoMetadata {
    pub width: u32,
    pub height: u32,
    pub duration: f64,
    pub format: String,
    pub size: u64,
    pub bitrate: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoProcessingOptions {
    pub remove_metadata: bool,
    pub compress: bool,
    pub extract_audio: bool,
    pub target_format: Option<String>,
}
