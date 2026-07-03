use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Cache error: {0}")]
    Cache(#[from] redis::RedisError),

    #[error("Bot error: {0}")]
    Bot(String),

    #[error("Processing error: {0}")]
    Processing(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Security error: {0}")]
    Security(String),
}

pub type AppResult<T> = Result<T, AppError>;
