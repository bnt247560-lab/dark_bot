use crate::errors::{AppError, AppResult};
use crate::models::VideoMetadata;
use crate::services::progress::ProgressCallback;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub struct FfmpegWrapper {
    ffmpeg_path: String,
    ffprobe_path: String,
}

impl FfmpegWrapper {
    pub fn new(ffmpeg_path: String, ffprobe_path: String) -> Self {
        Self { ffmpeg_path, ffprobe_path }
    }

    pub async fn get_metadata(&self, input: &Path) -> AppResult<VideoMetadata> {
        let output = Command::new(&self.ffprobe_path)
            .args([
                "-v", "error",
                "-show_entries", "stream=width,height,duration,bit_rate",
                "-show_entries", "format=format_name,size,duration",
                "-of", "json",
                input.to_str().ok_or(AppError::Internal("Invalid input path".into()))?,
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(AppError::Processing("FFprobe execution failed".into()));
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| AppError::Processing(format!("Failed to parse ffprobe output: {}", e)))?;

        let stream = json["streams"].as_array().and_then(|v| v.first()).cloned().unwrap_or_default();
        let format = &json["format"];
        let duration = stream["duration"]
            .as_str()
            .or_else(|| format["duration"].as_str())
            .unwrap_or("0")
            .parse()
            .unwrap_or(0.0);

        Ok(VideoMetadata {
            width: stream["width"].as_u64().unwrap_or(0) as u32,
            height: stream["height"].as_u64().unwrap_or(0) as u32,
            duration,
            format: format["format_name"].as_str().unwrap_or("unknown").to_string(),
            size: format["size"].as_str().unwrap_or("0").parse().unwrap_or(0),
            bitrate: stream["bit_rate"].as_str().unwrap_or("0").parse().unwrap_or(0),
        })
    }

    pub async fn process_video(&self, input: &Path, output: &Path, args: Vec<String>) -> AppResult<()> {
        self.process_video_with_progress(input, output, args, None).await
    }

    pub async fn process_video_with_progress(
        &self,
        input: &Path,
        output: &Path,
        args: Vec<String>,
        progress: Option<ProgressCallback>,
    ) -> AppResult<()> {
        let duration = self.get_metadata(input).await.map(|m| m.duration).unwrap_or(0.0);
        let total_ms = (duration * 1000.0).max(1.0) as u64;

        if let Some(callback) = &progress {
            callback(0, "بدء FFmpeg...".to_string()).await;
        }

        let mut cmd = Command::new(&self.ffmpeg_path);
        cmd.arg("-i").arg(input);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.args(["-progress", "pipe:1", "-nostats", "-y"]);
        cmd.arg(output);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take();
        let mut last_reported = 0u8;

        if let Some(stdout) = stdout {
            let mut lines = BufReader::new(stdout).lines();
            while let Some(line) = lines.next_line().await? {
                if let Some(raw) = line.strip_prefix("out_time_ms=") {
                    if let Ok(us) = raw.trim().parse::<u64>() {
                        let ms = us / 1000;
                        let pct = ((ms.saturating_mul(100)) / total_ms).min(100) as u8;
                        if pct >= last_reported.saturating_add(5) || pct == 100 {
                            last_reported = pct;
                            if let Some(callback) = &progress {
                                callback(pct, format!("معالجة الفيديو... {pct}%")).await;
                            }
                        }
                    }
                } else if line.trim() == "progress=end" {
                    if let Some(callback) = &progress {
                        callback(100, "اكتملت معالجة FFmpeg".to_string()).await;
                    }
                }
            }
        }

        let output_status = child.wait_with_output().await?;
        if !output_status.status.success() {
            let stderr = String::from_utf8_lossy(&output_status.stderr);
            return Err(AppError::Processing(format!("FFmpeg execution failed: {stderr}")));
        }

        if let Some(callback) = &progress {
            callback(100, "اكتملت معالجة FFmpeg".to_string()).await;
        }
        Ok(())
    }

    pub async fn remove_metadata(&self, input: &Path, output: &Path) -> AppResult<()> {
        self.remove_metadata_with_progress(input, output, None).await
    }

    pub async fn remove_metadata_with_progress(&self, input: &Path, output: &Path, progress: Option<ProgressCallback>) -> AppResult<()> {
        self.process_video_with_progress(input, output, vec![
            "-map_metadata".to_string(), "-1".to_string(),
            "-c".to_string(), "copy".to_string(),
        ], progress).await
    }

    pub async fn compress_video(&self, input: &Path, output: &Path, crf: u8) -> AppResult<()> {
        self.compress_video_with_progress(input, output, crf, None).await
    }

    pub async fn compress_video_with_progress(&self, input: &Path, output: &Path, crf: u8, progress: Option<ProgressCallback>) -> AppResult<()> {
        self.process_video_with_progress(input, output, vec![
            "-vcodec".to_string(), "libx264".to_string(),
            "-crf".to_string(), crf.to_string(),
            "-preset".to_string(), "medium".to_string(),
            "-acodec".to_string(), "aac".to_string(),
            "-b:a".to_string(), "128k".to_string(),
        ], progress).await
    }

    pub async fn extract_audio(&self, input: &Path, output: &Path) -> AppResult<()> {
        self.extract_audio_with_progress(input, output, None).await
    }

    pub async fn extract_audio_with_progress(&self, input: &Path, output: &Path, progress: Option<ProgressCallback>) -> AppResult<()> {
        self.process_video_with_progress(input, output, vec![
            "-vn".to_string(),
            "-acodec".to_string(), "libmp3lame".to_string(),
            "-q:a".to_string(), "2".to_string(),
        ], progress).await
    }

    pub async fn generate_thumbnail(&self, input: &Path, output: &Path, time_offset: &str) -> AppResult<()> {
        let status = Command::new(&self.ffmpeg_path)
            .args([
                "-ss", time_offset,
                "-i", input.to_str().unwrap(),
                "-vframes", "1",
                "-q:v", "2",
                "-y", output.to_str().unwrap(),
            ])
            .status()
            .await?;

        if !status.success() {
            return Err(AppError::Processing("Failed to generate thumbnail".into()));
        }
        Ok(())
    }
}
