use crate::services::ffmpeg::FfmpegWrapper;
use crate::errors::AppResult;
use std::path::Path;
use std::sync::Arc;

pub struct MetadataService {
    ffmpeg: Arc<FfmpegWrapper>,
}

impl MetadataService {
    pub fn new(ffmpeg: Arc<FfmpegWrapper>) -> Self {
        Self { ffmpeg }
    }

    pub async fn clean(&self, path: &Path) -> AppResult<()> {
        let temp_path = path.with_extension("tmp.mp4");
        self.ffmpeg.remove_metadata(path, &temp_path).await?;
        std::fs::rename(temp_path, path)?;
        Ok(())
    }
}
