use redis::aio::ConnectionManager;
use crate::errors::AppResult;

#[derive(Clone)]
pub struct RedisCache {
    manager: ConnectionManager,
}

impl RedisCache {
    pub async fn new(url: &str) -> AppResult<Self> {
        let client = redis::Client::open(url)?;
        let manager = ConnectionManager::new(client).await?;
        
        Ok(Self { manager })
    }

    pub async fn get_connection(&self) -> AppResult<ConnectionManager> {
        Ok(self.manager.clone())
    }

    pub async fn set(&self, key: &str, value: &str, expiry_secs: u64) -> AppResult<()> {
        let mut conn = self.manager.clone();
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .arg("EX")
            .arg(expiry_secs)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> AppResult<Option<String>> {
        let mut conn = self.manager.clone();
        let val: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut conn)
            .await?;
        Ok(val)
    }

    pub async fn increment_rate_limit(&self, key: &str, expiry: u64) -> AppResult<i64> {
        let mut conn = self.manager.clone();
        let count: i64 = redis::cmd("INCR")
            .arg(key)
            .query_async(&mut conn)
            .await?;
        
        if count == 1 {
            redis::cmd("EXPIRE").arg(key).arg(expiry).query_async(&mut conn).await?;
        }
        
        Ok(count)
    }
}
