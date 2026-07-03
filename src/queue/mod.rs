pub mod worker;

use crate::cache::RedisCache;
use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const JOB_QUEUE_KEY: &str = "dark_bot:jobs:pending";
const JOB_PROCESSING_KEY: &str = "dark_bot:jobs:processing";
const JOB_DEAD_KEY: &str = "dark_bot:jobs:dead";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueJob {
    pub job_id: Uuid,
    pub attempt: u32,
}

impl QueueJob {
    pub fn new(job_id: Uuid) -> Self {
        Self { job_id, attempt: 0 }
    }

    pub fn next_attempt(&self) -> Self {
        Self { job_id: self.job_id, attempt: self.attempt + 1 }
    }

    pub fn to_json(&self) -> AppResult<String> {
        serde_json::to_string(self)
            .map_err(|e| AppError::Internal(format!("Failed to serialize queue payload: {e}")))
    }

    pub fn from_json(raw: &str) -> AppResult<Self> {
        serde_json::from_str(raw)
            .map_err(|e| AppError::Internal(format!("Failed to deserialize queue payload: {e}")))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct QueueStats {
    pub pending: i64,
    pub processing: i64,
    pub dead: i64,
}

#[derive(Clone)]
pub struct JobQueue {
    cache: RedisCache,
}

impl JobQueue {
    pub fn new(cache: RedisCache) -> Self {
        Self { cache }
    }

    pub async fn enqueue_job_id(&self, job_id: Uuid) -> AppResult<()> {
        self.enqueue_payload(&QueueJob::new(job_id)).await
    }

    pub async fn retry(&self, payload: &QueueJob) -> AppResult<()> {
        let retry_payload = payload.next_attempt();
        self.enqueue_payload(&retry_payload).await
    }

    async fn enqueue_payload(&self, payload: &QueueJob) -> AppResult<()> {
        let payload_json = payload.to_json()?;
        let mut conn = self.cache.get_connection().await?;

        redis::cmd("LPUSH")
            .arg(JOB_QUEUE_KEY)
            .arg(payload_json)
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    pub async fn dequeue(&self) -> AppResult<Option<(QueueJob, String)>> {
        let mut conn = self.cache.get_connection().await?;
        let raw: Option<String> = redis::cmd("BRPOPLPUSH")
            .arg(JOB_QUEUE_KEY)
            .arg(JOB_PROCESSING_KEY)
            .arg(2)
            .query_async(&mut conn)
            .await?;

        let Some(raw_payload) = raw else {
            return Ok(None);
        };

        let payload = QueueJob::from_json(&raw_payload)?;

        Ok(Some((payload, raw_payload)))
    }

    pub async fn ack(&self, raw_payload: &str) -> AppResult<()> {
        let mut conn = self.cache.get_connection().await?;
        redis::cmd("LREM")
            .arg(JOB_PROCESSING_KEY)
            .arg(1)
            .arg(raw_payload)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn stats(&self) -> AppResult<QueueStats> {
        let mut conn = self.cache.get_connection().await?;
        let pending: i64 = redis::cmd("LLEN").arg(JOB_QUEUE_KEY).query_async(&mut conn).await?;
        let processing: i64 = redis::cmd("LLEN").arg(JOB_PROCESSING_KEY).query_async(&mut conn).await?;
        let dead: i64 = redis::cmd("LLEN").arg(JOB_DEAD_KEY).query_async(&mut conn).await?;
        Ok(QueueStats { pending, processing, dead })
    }

    pub async fn recover_processing_jobs(&self) -> AppResult<u64> {
        let mut conn = self.cache.get_connection().await?;
        let raw_jobs: Vec<String> = redis::cmd("LRANGE")
            .arg(JOB_PROCESSING_KEY)
            .arg(0)
            .arg(-1)
            .query_async(&mut conn)
            .await?;

        let mut recovered = 0;
        for raw in raw_jobs {
            redis::cmd("LPUSH")
                .arg(JOB_QUEUE_KEY)
                .arg(&raw)
                .query_async(&mut conn)
                .await?;
            redis::cmd("LREM")
                .arg(JOB_PROCESSING_KEY)
                .arg(1)
                .arg(&raw)
                .query_async(&mut conn)
                .await?;
            recovered += 1;
        }
        Ok(recovered)
    }


    pub async fn dead_jobs(&self, limit: isize) -> AppResult<Vec<(QueueJob, String)>> {
        let mut conn = self.cache.get_connection().await?;
        let end = (limit - 1).max(0);
        let raw_jobs: Vec<String> = redis::cmd("LRANGE")
            .arg(JOB_DEAD_KEY)
            .arg(0)
            .arg(end)
            .query_async(&mut conn)
            .await?;

        let mut jobs = Vec::new();
        for raw in raw_jobs {
            if let Ok(payload) = QueueJob::from_json(&raw) {
                jobs.push((payload, raw));
            }
        }
        Ok(jobs)
    }

    pub async fn requeue_dead_jobs(&self, limit: isize) -> AppResult<u64> {
        let jobs = self.dead_jobs(limit).await?;
        let mut conn = self.cache.get_connection().await?;
        let mut moved = 0;

        for (mut payload, raw) in jobs {
            payload.attempt = 0;
            let new_raw = payload.to_json()?;
            redis::cmd("LPUSH")
                .arg(JOB_QUEUE_KEY)
                .arg(new_raw)
                .query_async(&mut conn)
                .await?;
            redis::cmd("LREM")
                .arg(JOB_DEAD_KEY)
                .arg(1)
                .arg(raw)
                .query_async(&mut conn)
                .await?;
            moved += 1;
        }

        Ok(moved)
    }

    pub async fn dead_letter(&self, raw_payload: &str) -> AppResult<()> {
        let mut conn = self.cache.get_connection().await?;
        redis::cmd("LPUSH")
            .arg(JOB_DEAD_KEY)
            .arg(raw_payload)
            .query_async(&mut conn)
            .await?;
        redis::cmd("LREM")
            .arg(JOB_PROCESSING_KEY)
            .arg(1)
            .arg(raw_payload)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }
}
