use crate::app::AppState;
use crate::errors::{AppError, AppResult};
use std::sync::Arc;

pub async fn check_rate_limit(user_id: i64, state: Arc<AppState>) -> AppResult<()> {
    let key = format!("rate_limit:{}", user_id);
    let count = state.cache.increment_rate_limit(&key, 60).await?;
    
    if count > 10 { // 10 requests per minute
        return Err(AppError::Validation("Rate limit exceeded. Please wait a minute.".into()));
    }
    
    Ok(())
}
