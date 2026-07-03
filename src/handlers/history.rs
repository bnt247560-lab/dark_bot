use teloxide::prelude::*;
use crate::app::AppState;
use std::sync::Arc;

pub async fn handle(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    let jobs = state.db.get_user_jobs(msg.chat.id.0, 5).await?;
    
    if jobs.is_empty() {
        bot.send_message(msg.chat.id, "You have no recent jobs.").await?;
        return Ok(());
    }

    let mut response = "Your last 5 jobs:\n\n".to_string();
    for job in jobs {
        response.push_str(&format!(
            "🆔 `{}`\n📅 {}\n📊 Status: {:?}\n\n",
            job.id,
            job.created_at.format("%Y-%m-%d %H:%M"),
            job.status
        ));
    }

    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}
