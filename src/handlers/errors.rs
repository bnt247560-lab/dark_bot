use teloxide::prelude::*;
use crate::errors::AppError;

pub async fn handle_error(bot: Bot, chat_id: ChatId, error: AppError) {
    let message = match error {
        AppError::Validation(m) => format!("❌ Validation Error: {}", m),
        _ => "❌ An internal error occurred. Please try again later.".to_string(),
    };
    let _ = bot.send_message(chat_id, message).await;
}
