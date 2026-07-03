use crate::app::AppState;
use std::sync::Arc;
use teloxide::prelude::*;

pub async fn handle(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    let count = state.db.cancel_user_pending_jobs(msg.chat.id.0).await?;
    if count == 0 {
        bot.send_message(msg.chat.id, "لا يوجد طلب نشط لإلغائه.").await?;
    } else {
        bot.send_message(msg.chat.id, format!("تم إلغاء {} طلب/طلبات نشطة.", count)).await?;
    }
    Ok(())
}
