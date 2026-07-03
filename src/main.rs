use dark_bot::{app::App, config::Settings};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting dark_bot...");

    let config = Settings::new()?;
    let app = App::new(config).await?;
    app.run().await?;

    Ok(())
}
