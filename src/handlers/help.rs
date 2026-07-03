use teloxide::prelude::*;
use crate::bot::commands::Command;
use teloxide::utils::command::BotCommands;

pub async fn handle(bot: Bot, msg: Message) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}
