use std::process::Command;

pub fn clean_video(input: &str, output: &str) -> Result<(), String> {
    let status = Command::new("ffmpeg")
        .args([
            "-i", input,
            "-map_metadata", "-1",         // محو الميتا داتا
            "-c:v", "libx264",             // إعادة ترميز
            "-preset", "veryfast",         // السرعة
            "-crf", "24",                  // توازن الجودة
            "-vf", "crop=iw-2:ih-2",       // كسر البصمات المكانية
            "-c:a", "copy",                // الحفاظ على الصوت كما هو
            "-y", output,
        ])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() { Ok(()) } else { Err("فشل التطهير".into()) }
}