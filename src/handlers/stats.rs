use teloxide::prelude::*;
use crate::app::AppState;
use std::sync::Arc;

pub async fn handle(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    let (users, jobs) = state.db.get_stats().await?;
    
    let response = format!(
        "📊 *Bot Statistics*\n\n👥 Total Users: {}\n🔄 Total Jobs Processed: {}",
        users, jobs
    );

    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}
