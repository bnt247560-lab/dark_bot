use crate::errors::AppResult;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub async fn create_job_dir(base_storage_path: &str, job_id: Uuid) -> AppResult<PathBuf> {
    let dir = Path::new(base_storage_path).join("temp").join(job_id.to_string());
    tokio::fs::create_dir_all(&dir).await?;
    Ok(dir)
}

pub async fn cleanup_job_dir(base_storage_path: &str, job_id: Uuid) -> AppResult<()> {
    let dir = Path::new(base_storage_path).join("temp").join(job_id.to_string());
    if tokio::fs::try_exists(&dir).await? {
        tokio::fs::remove_dir_all(dir).await?;
    }
    Ok(())
}
