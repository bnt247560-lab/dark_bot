use crate::errors::{AppError, AppResult};
use crate::services::progress::ProgressCallback;
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

pub struct Downloader {
    yt_dlp_path: String,
}

impl Downloader {
    pub fn new(yt_dlp_path: String) -> Self {
        Self { yt_dlp_path }
    }

    pub async fn download(&self, url: &str, output_dir: &Path) -> AppResult<PathBuf> {
        self.download_with_progress(url, output_dir, None).await
    }

    pub async fn download_with_progress(
        &self,
        url: &str,
        output_dir: &Path,
        progress: Option<ProgressCallback>,
    ) -> AppResult<PathBuf> {
        let file_id = Uuid::new_v4().to_string();
        let output_template = output_dir.join(format!("{}.%(ext)s", file_id));

        if let Some(callback) = &progress {
            callback(0, "تحضير التحميل...".to_string()).await;
        }

        let mut child = Command::new(&self.yt_dlp_path)
            .args([
                "--no-playlist",
                "--newline",
                "--no-color",
                "--format", "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best",
                "--merge-output-format", "mp4",
                "-o", output_template.to_str().ok_or_else(|| AppError::Internal("Invalid output path".into()))?,
                url,
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take();
        let progress_re = Regex::new(r"\[download\]\s+([0-9]+(?:\.[0-9]+)?)%").unwrap();

        let mut last_reported = 0u8;
        if let Some(stdout) = stdout {
            let mut lines = BufReader::new(stdout).lines();
            while let Some(line) = lines.next_line().await? {
                if let Some(caps) = progress_re.captures(&line) {
                    let pct = caps
                        .get(1)
                        .and_then(|m| m.as_str().parse::<f32>().ok())
                        .map(|p| p.round().clamp(0.0, 100.0) as u8)
                        .unwrap_or(last_reported);
                    if pct >= last_reported.saturating_add(5) || pct == 100 {
                        last_reported = pct;
                        if let Some(callback) = &progress {
                            callback(pct, format!("تحميل الفيديو... {pct}%")).await;
                        }
                    }
                }
            }
        }

        let output = child.wait_with_output().await?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Download(format!("yt-dlp failed: {}", err_msg)));
        }

        // Drain stderr only after process ends when stdout parsing did not consume it fully.
        if let Some(callback) = &progress {
            callback(100, "اكتمل التحميل".to_string()).await;
        }

        for entry in std::fs::read_dir(output_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.file_stem().and_then(|s| s.to_str()) == Some(&file_id) {
                return Ok(path);
            }
        }

        // Fallback: yt-dlp can create a merged mp4 with a slightly different name in edge cases.
        let mut candidates = Vec::new();
        for entry in std::fs::read_dir(output_dir)? {
            let path = entry?.path();
            if path.is_file() {
                candidates.push(path);
            }
        }
        candidates
            .into_iter()
            .max_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
            .ok_or_else(|| AppError::Download("Downloaded file not found".into()))
    }
}
