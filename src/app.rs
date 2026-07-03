use crate::cache::RedisCache;
use crate::config::Settings;
use crate::database::PostgresRepository;
use crate::queue::worker::Worker;
use crate::metrics::{Metrics, SharedMetrics};
use crate::queue::JobQueue;
use crate::services::{Downloader, FfmpegWrapper, ObjectStorage, Uploader, VideoProcessor};
use std::sync::Arc;
use teloxide::dptree;
use teloxide::prelude::*;
use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub config: Settings,
    pub db: PostgresRepository,
    pub cache: RedisCache,
    pub queue: JobQueue,
    pub downloader: Downloader,
    pub processor: VideoProcessor,
    pub uploader: Uploader,
    pub object_storage: ObjectStorage,
    pub metrics: SharedMetrics,
    pub shutdown: CancellationToken,
}

pub struct App {
    state: Arc<AppState>,
}

impl App {
    pub async fn new(config: Settings) -> anyhow::Result<Self> {
        let db_pool = sqlx::PgPool::connect(&config.database_url).await?;

        tracing::info!("Running database migrations...");
        sqlx::migrate!("./migrations").run(&db_pool).await?;
        tracing::info!("Database migrations completed");

        let db = PostgresRepository::new(db_pool);
        let cache = RedisCache::new(&config.redis_url).await?;
        let queue = JobQueue::new(cache.clone());

        let ffmpeg = Arc::new(FfmpegWrapper::new(
            config.ffmpeg_path.clone(),
            config.ffprobe_path.clone(),
        ));
        let downloader = Downloader::new(config.yt_dlp_path.clone());
        let processor = VideoProcessor::new(ffmpeg);

        let bot = Bot::new(&config.teloxide_token);
        let uploader = Uploader::new(bot.clone());
        let object_storage = ObjectStorage::from_settings(&config).await?;
        let metrics = Arc::new(Metrics::new()?);

        let state = Arc::new(AppState {
            config,
            db,
            cache,
            queue,
            downloader,
            processor,
            uploader,
            object_storage,
            metrics,
            shutdown: CancellationToken::new(),
        });

        Ok(Self { state })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        self.recover_interrupted_jobs().await?;
        self.start_workers();
        self.start_health_server();

        let bot = Bot::new(&self.state.config.teloxide_token);
        let handler = crate::bot::dispatcher::setup_handler();

        tracing::info!(workers = self.state.config.worker_count, "Bot is running");

        Dispatcher::builder(bot, handler)
            .dependencies(dptree::deps![self.state.clone()])
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;

        self.state.shutdown.cancel();
        tracing::info!("Shutdown signal sent to workers");
        Ok(())
    }

    fn start_workers(&self) {
        for id in 1..=self.state.config.worker_count {
            let worker = Worker::new(id as u32, self.state.clone());
            tokio::spawn(async move {
                worker.run().await;
            });
        }
    }
}

impl App {
    async fn recover_interrupted_jobs(&self) -> anyhow::Result<()> {
        let db_recovered = self.state.db.recover_stale_active_jobs().await?;
        let queue_recovered = self.state.queue.recover_processing_jobs().await?;
        if db_recovered > 0 || queue_recovered > 0 {
            tracing::warn!(db_recovered, queue_recovered, "Recovered interrupted jobs from previous run");
        }
        Ok(())
    }

    fn start_health_server(&self) {
        let state = self.state.clone();
        let bind = state.config.health_bind.clone();
        tokio::spawn(async move {
            if let Err(err) = crate::health::serve(state, bind).await {
                tracing::error!(error = %err, "Health server stopped unexpectedly");
            }
        });
    }
}
