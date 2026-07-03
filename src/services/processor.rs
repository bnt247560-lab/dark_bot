use crate::errors::{AppError, AppResult};
use crate::models::{Job, VideoMetadata, VideoProcessingOptions};
use crate::services::ffmpeg::FfmpegWrapper;
use crate::services::progress::ProgressCallback;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct VideoProcessor {
    ffmpeg: Arc<FfmpegWrapper>,
}

impl VideoProcessor {
    pub fn new(ffmpeg: Arc<FfmpegWrapper>) -> Self {
        Self { ffmpeg }
    }

    pub async fn process(&self, job: &Job, options: VideoProcessingOptions) -> AppResult<PathBuf> {
        self.process_with_progress(job, options, None).await
    }

    pub async fn process_with_progress(
        &self,
        job: &Job,
        options: VideoProcessingOptions,
        progress: Option<ProgressCallback>,
    ) -> AppResult<PathBuf> {
        let input_path = PathBuf::from(job.file_path.as_ref().ok_or(AppError::Internal("Job file path missing".into()))?);
        let mut current_path = input_path.clone();

        if options.remove_metadata {
            let output = current_path.with_extension("clean.mp4");
            self.ffmpeg
                .remove_metadata_with_progress(&current_path, &output, progress.clone())
                .await?;
            current_path = output;
        }

        if options.extract_audio {
            let output = current_path.with_extension("mp3");
            self.ffmpeg
                .extract_audio_with_progress(&current_path, &output, progress.clone())
                .await?;
            return Ok(output);
        }

        if options.compress {
            let output = current_path.with_extension("compressed.mp4");
            self.ffmpeg
                .compress_video_with_progress(&current_path, &output, 28, progress.clone())
                .await?;
            current_path = output;
        }

        if let Some(fmt) = options.target_format {
            let output = current_path.with_extension(fmt);
            self.ffmpeg
                .process_video_with_progress(&current_path, &output, vec![], progress)
                .await?;
            current_path = output;
        }

        Ok(current_path)
    }

    pub async fn analyze(&self, path: &Path) -> AppResult<VideoMetadata> {
        self.ffmpeg.get_metadata(path).await
    }
}
