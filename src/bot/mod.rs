pub mod dispatcher;
pub mod commands;
pub mod callbacks;

use teloxide::prelude::*;
use crate::config::Settings;
use crate::database::PostgresPool;
use crate::cache::RedisCache;
use crate::queue::JobQueue;
use std::sync::Arc;

pub struct BotService {
    config: Settings,
    db: Arc<PostgresPool>,
    cache: Arc<RedisCache>,
    queue: Arc<JobQueue>,
}

impl BotService {
    pub fn new(
        config: Settings,
        db: Arc<PostgresPool>,
        cache: Arc<RedisCache>,
        queue: Arc<JobQueue>,
    ) -> Self {
        Self { config, db, cache, queue }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let bot = Bot::new(&self.config.teloxide_token);
        
        tracing::info!("Bot started and listening for updates...");
        
        let handler = dispatcher::setup_handler();
        
        Dispatcher::builder(bot, handler)
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
            
        Ok(())
    }
}
