use std::process::Command;

pub fn download_video(url: &str, output: &str) -> bool {
    Command::new("yt-dlp")
        .args([url, "-o", output, "--quiet"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}