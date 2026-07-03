use std::path::Path;
use crate::errors::AppResult;

pub async fn delete_file(path: &Path) -> AppResult<()> {
    if path.exists() {
        tokio::fs::remove_file(path).await?;
    }
    Ok(())
}
