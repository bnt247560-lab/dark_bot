use teloxide::prelude::*;
use teloxide::dispatching::UpdateHandler;
use crate::app::AppState;
use crate::bot::commands::Command;
use crate::handlers::{admin, callback, cancel, help, history, start, stats, status, video};
use std::sync::Arc;

pub fn setup_handler() -> UpdateHandler<anyhow::Error> {
    dptree::entry()
        .branch(
            Update::filter_message()
                .branch(dptree::entry().filter_command::<Command>().endpoint(commands_handler))
                .branch(dptree::entry().filter(|msg: Message| msg.video().is_some()).endpoint(video::handle_video))
                .branch(dptree::entry().filter(|msg: Message| msg.text().is_some()).endpoint(video::handle_link)),
        )
        .branch(Update::filter_callback_query().endpoint(callback::handle_callback))
}

async fn commands_handler(bot: Bot, msg: Message, cmd: Command, state: Arc<AppState>) -> anyhow::Result<()> {
    match cmd {
        Command::Start => start::handle(bot, msg, state).await,
        Command::Help => help::handle(bot, msg).await,
        Command::Settings => {
            bot.send_message(msg.chat.id, "الإعدادات ستُضاف في المرحلة القادمة.").await?;
            Ok(())
        }
        Command::Status => status::handle(bot, msg, state).await,
        Command::Cancel => cancel::handle(bot, msg, state).await,
        Command::History => history::handle(bot, msg, state).await,
        Command::Stats => stats::handle(bot, msg, state).await,
        Command::Admin => admin::dashboard(bot, msg, state).await,
        Command::Queue => admin::queue(bot, msg, state).await,
        Command::Failed => admin::failed(bot, msg, state).await,
        Command::RetryFailed => admin::retry_failed(bot, msg, state).await,
    }
}
