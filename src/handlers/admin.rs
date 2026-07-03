use crate::app::AppState;
use std::sync::Arc;
use teloxide::prelude::*;

fn admin_user_id(msg: &Message) -> Option<i64> {
    msg.from().map(|user| user.id.0 as i64)
}

async fn ensure_admin(bot: &Bot, msg: &Message, state: &AppState) -> anyhow::Result<bool> {
    let Some(user_id) = admin_user_id(msg) else {
        bot.send_message(msg.chat.id, "تعذر تحديد المستخدم.").await?;
        return Ok(false);
    };

    if !state.config.is_admin(user_id) {
        bot.send_message(msg.chat.id, "⛔ هذا الأمر مخصص للمدير فقط.").await?;
        return Ok(false);
    }

    Ok(true)
}

pub async fn dashboard(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    if !ensure_admin(&bot, &msg, &state).await? {
        return Ok(());
    }

    let (users, jobs) = state.db.get_stats().await?;
    let queue = state.queue.stats().await?;
    let counts = state.db.get_job_status_counts().await?;

    let mut status_lines = String::new();
    for (status, count) in counts {
        status_lines.push_str(&format!("• {}: {}\n", status, count));
    }

    let response = format!(
        "🛡️ *Admin Dashboard*\n\n👥 Users: {}\n🎬 Jobs: {}\n\n📦 *Queue*\n• Pending: {}\n• Processing: {}\n• Dead: {}\n\n📊 *Job Status*\n{}\nالأوامر:\n/queue\n/failed\n/retryfailed",
        users,
        jobs,
        queue.pending,
        queue.processing,
        queue.dead,
        if status_lines.is_empty() { "لا توجد بيانات.\n".to_string() } else { status_lines }
    );

    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}

pub async fn queue(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    if !ensure_admin(&bot, &msg, &state).await? {
        return Ok(());
    }

    let stats = state.queue.stats().await?;
    let response = format!(
        "📦 *Queue Metrics*\n\n⏳ Pending: {}\n⚙️ Processing: {}\n💀 Dead: {}",
        stats.pending, stats.processing, stats.dead
    );
    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}

pub async fn failed(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    if !ensure_admin(&bot, &msg, &state).await? {
        return Ok(());
    }

    let failed_jobs = state.db.get_recent_failed_jobs(5).await?;
    let dead_jobs = state.queue.dead_jobs(5).await?;

    let mut response = String::from("❌ *Recent Failed Jobs*\n\n");
    if failed_jobs.is_empty() {
        response.push_str("لا توجد مهام فاشلة في قاعدة البيانات.\n");
    } else {
        for job in failed_jobs {
            let short_id = job.id.to_string();
            let short_id = &short_id[..8];
            let error = job.error_message.unwrap_or_else(|| "بدون رسالة خطأ".to_string());
            response.push_str(&format!(
                "• `{}` | user `{}` | {}%\n  {}\n",
                short_id, job.user_id, job.progress, error.chars().take(120).collect::<String>()
            ));
        }
    }

    response.push_str("\n💀 *Dead Queue*\n");
    if dead_jobs.is_empty() {
        response.push_str("لا توجد عناصر في Dead Queue.");
    } else {
        for (payload, _) in dead_jobs {
            let short_id = payload.job_id.to_string();
            let short_id = &short_id[..8];
            response.push_str(&format!("• `{}` attempt `{}`\n", short_id, payload.attempt));
        }
        response.push_str("\nاستخدم /retryfailed لإعادة إدخال آخر العناصر الفاشلة.");
    }

    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}

pub async fn retry_failed(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    if !ensure_admin(&bot, &msg, &state).await? {
        return Ok(());
    }

    let dead_jobs = state.queue.dead_jobs(20).await?;
    for (payload, _) in &dead_jobs {
        state.db.reset_failed_job_to_pending(payload.job_id).await?;
    }
    let moved = state.queue.requeue_dead_jobs(20).await?;

    bot.send_message(
        msg.chat.id,
        format!("🔁 تمت إعادة إدخال {} مهمة من Dead Queue إلى قائمة الانتظار.", moved),
    )
    .await?;
    Ok(())
}
