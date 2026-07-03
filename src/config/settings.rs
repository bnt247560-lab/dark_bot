use config::{Config, Environment};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub teloxide_token: String,
    pub database_url: String,
    pub redis_url: String,
    pub log_level: String,
    pub storage_path: String,
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub yt_dlp_path: String,
    pub worker_count: usize,
    pub max_job_retries: u32,
    pub max_telegram_file_mb: u64,
    pub health_bind: String,
    pub max_url_length: usize,
    pub admin_user_ids: String,
    pub object_storage_enabled: bool,
    pub object_storage_endpoint: String,
    pub object_storage_region: String,
    pub object_storage_bucket: String,
    pub object_storage_access_key_id: String,
    pub object_storage_secret_access_key: String,
    pub object_storage_public_base_url: String,
}


impl Settings {
    pub fn new() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let settings: Self = Config::builder()
            .set_default("log_level", "info")?
            .set_default("storage_path", "./storage")?
            .set_default("ffmpeg_path", "ffmpeg")?
            .set_default("ffprobe_path", "ffprobe")?
            .set_default("yt_dlp_path", "yt-dlp")?
            .set_default("worker_count", 2)?
            .set_default("max_job_retries", 3)?
            .set_default("max_telegram_file_mb", 512)?
            .set_default("health_bind", "0.0.0.0:8080")?
            .set_default("max_url_length", 2048)?
            .set_default("admin_user_ids", "")?
            .set_default("object_storage_enabled", false)?
            .set_default("object_storage_endpoint", "")?
            .set_default("object_storage_region", "auto")?
            .set_default("object_storage_bucket", "")?
            .set_default("object_storage_access_key_id", "")?
            .set_default("object_storage_secret_access_key", "")?
            .set_default("object_storage_public_base_url", "")?
            .add_source(Environment::default().separator("__"))
            .build()?
            .try_deserialize()?;

        settings.validate()?;
        settings.ensure_storage_dirs()?;
        Ok(settings)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.teloxide_token.trim().is_empty() || self.teloxide_token == "your_bot_token_here" {
            anyhow::bail!("TELOXIDE_TOKEN is missing or still uses the placeholder value");
        }
        if !self.database_url.starts_with("postgres://") && !self.database_url.starts_with("postgresql://") {
            anyhow::bail!("DATABASE_URL must be a valid PostgreSQL connection string");
        }
        if !self.redis_url.starts_with("redis://") && !self.redis_url.starts_with("rediss://") {
            anyhow::bail!("REDIS_URL must be a valid Redis connection string");
        }
        if self.worker_count == 0 {
            anyhow::bail!("WORKER_COUNT must be at least 1");
        }
        if self.max_job_retries == 0 {
            anyhow::bail!("MAX_JOB_RETRIES must be at least 1");
        }
        if self.max_telegram_file_mb == 0 {
            anyhow::bail!("MAX_TELEGRAM_FILE_MB must be at least 1");
        }
        if self.max_url_length < 20 {
            anyhow::bail!("MAX_URL_LENGTH must be at least 20");
        }
        for raw in self.admin_user_ids.split(',').map(str::trim).filter(|v| !v.is_empty()) {
            raw.parse::<i64>()
                .map_err(|_| anyhow::anyhow!("ADMIN_USER_IDS contains an invalid Telegram user id: {raw}"))?;
        }
        self.health_bind.parse::<std::net::SocketAddr>()
            .map_err(|e| anyhow::anyhow!("HEALTH_BIND must be a valid socket address: {e}"))?;

        if self.object_storage_enabled {
            if self.object_storage_bucket.trim().is_empty() {
                anyhow::bail!("OBJECT_STORAGE_BUCKET is required when OBJECT_STORAGE_ENABLED=true");
            }
            if self.object_storage_access_key_id.trim().is_empty() {
                anyhow::bail!("OBJECT_STORAGE_ACCESS_KEY_ID is required when OBJECT_STORAGE_ENABLED=true");
            }
            if self.object_storage_secret_access_key.trim().is_empty() {
                anyhow::bail!("OBJECT_STORAGE_SECRET_ACCESS_KEY is required when OBJECT_STORAGE_ENABLED=true");
            }
            if self.object_storage_public_base_url.trim().is_empty() {
                anyhow::bail!("OBJECT_STORAGE_PUBLIC_BASE_URL is required when OBJECT_STORAGE_ENABLED=true");
            }
            if !self.object_storage_endpoint.trim().is_empty()
                && !(self.object_storage_endpoint.starts_with("https://") || self.object_storage_endpoint.starts_with("http://"))
            {
                anyhow::bail!("OBJECT_STORAGE_ENDPOINT must start with http:// or https://");
            }
        }

        Ok(())
    }

    pub fn admin_ids(&self) -> Vec<i64> {
        self.admin_user_ids
            .split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .filter_map(|v| v.parse::<i64>().ok())
            .collect()
    }

    pub fn is_admin(&self, user_id: i64) -> bool {
        self.admin_ids().contains(&user_id)
    }

    pub fn ensure_storage_dirs(&self) -> anyhow::Result<()> {
        let base = Path::new(&self.storage_path);
        std::fs::create_dir_all(base.join("downloads"))?;
        std::fs::create_dir_all(base.join("processed"))?;
        std::fs::create_dir_all(base.join("temp"))?;
        Ok(())
    }
}
