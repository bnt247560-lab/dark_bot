use teloxide::prelude::*;
use crate::app::AppState;
use std::sync::Arc;

pub async fn handle(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    let user = msg.from();
    let first_name = user.map(|u| u.first_name.as_str()).unwrap_or("Telegram User");
    let username = user.and_then(|u| u.username.as_deref());

    state.db.get_or_create_user(msg.chat.id.0, first_name, username).await?;

    bot.send_message(
        msg.chat.id,
        "مرحباً بك في Dark Bot 🚀\n\nأرسل رابط فيديو أو ملف فيديو، وسأجهّز لك خيارات المعالجة.",
    )
    .await?;

    Ok(())
}
