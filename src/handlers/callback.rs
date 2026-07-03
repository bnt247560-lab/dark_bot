use teloxide::prelude::*;

pub async fn handle_callback(bot: Bot, q: CallbackQuery) -> anyhow::Result<()> {
    if let Some(data) = q.data {
        bot.answer_callback_query(q.id).text(format!("Selected: {}", data)).await?;
    }
    Ok(())
}
