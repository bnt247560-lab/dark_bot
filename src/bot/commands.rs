use teloxide::macros::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Dark Bot Commands:")]
pub enum Command {
    #[command(description = "Start the bot")]
    Start,
    #[command(description = "Show help")]
    Help,
    #[command(description = "Bot settings")]
    Settings,
    #[command(description = "Check job status")]
    Status,
    #[command(description = "Cancel current job")]
    Cancel,
    #[command(description = "Show job history")]
    History,
    #[command(description = "Show statistics")]
    Stats,
    #[command(description = "Admin dashboard")]
    Admin,
    #[command(description = "Admin queue metrics")]
    Queue,
    #[command(description = "Admin failed jobs")]
    Failed,
    #[command(description = "Admin requeue dead jobs")]
    RetryFailed,
}
