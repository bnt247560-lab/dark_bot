use crate::app::AppState;
use std::sync::Arc;
use teloxide::prelude::*;

pub async fn handle(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    match state.db.get_latest_active_job(msg.chat.id.0).await? {
        Some(job) => {
            bot.send_message(
                msg.chat.id,
                format!(
                    "📊 آخر طلب نشط:\n\n🆔 `{}`\nالحالة: {:?}\nالتقدم: {}%",
                    job.id, job.status, job.progress
                ),
            )
            
            .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "لا يوجد طلب نشط حالياً.").await?;
        }
    }
    Ok(())
}
