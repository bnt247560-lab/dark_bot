use crate::app::AppState;
use crate::errors::{AppError, AppResult};
use crate::middleware::rate_limit::check_rate_limit;
use crate::utils::temp::create_job_dir;
use std::path::PathBuf;
use std::sync::Arc;
use teloxide::net::Download;
use teloxide::prelude::*;
use tokio::io::AsyncWriteExt;

pub async fn handle_video(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    check_rate_limit(msg.chat.id.0, state.clone()).await?;
    register_user_from_message(&msg, state.clone()).await?;

    let Some(video) = msg.video() else {
        return Ok(());
    };

    let max_bytes = state.config.max_telegram_file_mb * 1024 * 1024;
    if (video.file.size as u64) > max_bytes {
        bot.send_message(
            msg.chat.id,
            format!(
                "⚠️ حجم الفيديو أكبر من الحد المسموح حالياً: {}MB.",
                state.config.max_telegram_file_mb
            ),
        )
        .await?;
        return Ok(());
    }

    let placeholder = bot
        .send_message(msg.chat.id, "📥 تم استلام الفيديو. جارٍ تنزيله من تيليجرام وإدخاله في قائمة المعالجة...")
        .await?;

    let job = state.db.create_job(msg.chat.id.0, None).await?;
    let local_path = match download_telegram_video(&bot, &msg, state.clone(), job.id).await {
        Ok(path) => path,
        Err(err) => {
            let _ = state.db.fail_job(job.id, &err.to_string()).await;
            bot.edit_message_text(msg.chat.id, placeholder.id, format!("❌ تعذر تنزيل الفيديو: {err}"))
                .await?;
            return Ok(());
        }
    };

    state
        .db
        .update_job_file_path(job.id, &local_path.to_string_lossy())
        .await?;
    state.queue.enqueue_job_id(job.id).await?;

    bot.edit_message_text(
        msg.chat.id,
        placeholder.id,
        format!(
            "✅ تم إدخال الفيديو في قائمة المعالجة.\n\n🆔 Job: `{}`\n📊 الحالة: pending\n\nاستخدم /status لمعرفة آخر حالة.",
            job.id
        ),
    )
    .await?;

    Ok(())
}

pub async fn handle_link(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    let text = msg.text().unwrap_or_default().trim();
    if !is_supported_url(text, state.config.max_url_length) {
        return Ok(());
    }

    check_rate_limit(msg.chat.id.0, state.clone()).await?;
    register_user_from_message(&msg, state.clone()).await?;

    let job = state.db.create_job(msg.chat.id.0, Some(text)).await?;
    state.queue.enqueue_job_id(job.id).await?;

    bot.send_message(
        msg.chat.id,
        format!(
            "✅ تم إنشاء طلب جديد.\n\n🆔 Job: `{}`\n📊 الحالة: pending\n\nاستخدم /status لمعرفة آخر حالة.",
            job.id
        ),
    )
    .await?;

    Ok(())
}

async fn register_user_from_message(msg: &Message, state: Arc<AppState>) -> AppResult<()> {
    let user = msg.from();
    let first_name = user.map(|u| u.first_name.as_str()).unwrap_or("Telegram User");
    let username = user.and_then(|u| u.username.as_deref());
    state
        .db
        .get_or_create_user(msg.chat.id.0, first_name, username)
        .await?;
    Ok(())
}

async fn download_telegram_video(bot: &Bot, msg: &Message, state: Arc<AppState>, job_id: uuid::Uuid) -> AppResult<PathBuf> {
    let video = msg
        .video()
        .ok_or_else(|| AppError::Validation("Message does not contain a video".into()))?;

    let file = bot
        .get_file(video.file.id.clone())
        .await
        .map_err(|e| AppError::Bot(e.to_string()))?;

    let job_dir = create_job_dir(&state.config.storage_path, job_id).await?;
    let extension = detect_video_extension(video.file_name.as_deref());
    let output_path = job_dir.join(format!("telegram_upload.{extension}"));

    let mut destination = tokio::fs::File::create(&output_path).await?;
    bot.download_file(&file.path, &mut destination)
        .await
        .map_err(|e| AppError::Download(format!("Telegram file download failed: {e}")))?;
    destination.flush().await?;

    Ok(output_path)
}

fn detect_video_extension(file_name: Option<&str>) -> &'static str {
    if let Some(name) = file_name {
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".mp4") {
            return "mp4";
        }
        if lower.ends_with(".mov") {
            return "mov";
        }
        if lower.ends_with(".mkv") {
            return "mkv";
        }
        if lower.ends_with(".webm") {
            return "webm";
        }
    }

    "mp4"
}

fn is_supported_url(text: &str, max_len: usize) -> bool {
    if text.len() > max_len {
        return false;
    }
    if !(text.starts_with("http://") || text.starts_with("https://")) {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    !lower.contains("localhost")
        && !lower.contains("127.0.0.1")
        && !lower.contains("0.0.0.0")
        && !lower.contains("::1")
}
