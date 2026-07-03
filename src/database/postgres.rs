use sqlx::{Pool, Postgres, Row};
use crate::errors::AppResult;
use crate::models::{User, Job, JobStatus};
use uuid::Uuid;

pub struct PostgresRepository {
    pool: Pool<Postgres>,
}

impl PostgresRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn health_check(&self) -> AppResult<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn recover_stale_active_jobs(&self) -> AppResult<u64> {
        let result = sqlx::query(
            "UPDATE jobs
             SET status = 'pending', progress = 0, updated_at = NOW()
             WHERE status IN ('downloading', 'processing', 'uploading')"
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn get_or_create_user(&self, user_id: i64, first_name: &str, username: Option<&str>) -> AppResult<User> {
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (id, first_name, username, updated_at)
             VALUES ($1, $2, $3, NOW())
             ON CONFLICT (id) DO UPDATE SET
                first_name = EXCLUDED.first_name,
                username = EXCLUDED.username,
                updated_at = NOW()
             RETURNING *"
        )
        .bind(user_id)
        .bind(first_name)
        .bind(username)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn create_job(&self, user_id: i64, source_url: Option<&str>) -> AppResult<Job> {
        let id = Uuid::new_v4();
        let job = sqlx::query_as::<_, Job>(
            "INSERT INTO jobs (id, user_id, status, progress, source_url, created_at, updated_at)
             VALUES ($1, $2, 'pending', 0, $3, NOW(), NOW())
             RETURNING *"
        )
        .bind(id)
        .bind(user_id)
        .bind(source_url)
        .fetch_one(&self.pool)
        .await?;
        Ok(job)
    }


    pub async fn create_uploaded_file_job(&self, user_id: i64, file_path: &str) -> AppResult<Job> {
        let id = Uuid::new_v4();
        let job = sqlx::query_as::<_, Job>(
            "INSERT INTO jobs (id, user_id, status, progress, file_path, created_at, updated_at)
             VALUES ($1, $2, 'pending', 0, $3, NOW(), NOW())
             RETURNING *"
        )
        .bind(id)
        .bind(user_id)
        .bind(file_path)
        .fetch_one(&self.pool)
        .await?;
        Ok(job)
    }

    pub async fn get_job(&self, job_id: Uuid) -> AppResult<Job> {
        let job = sqlx::query_as::<_, Job>("SELECT * FROM jobs WHERE id = $1")
            .bind(job_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(job)
    }

    pub async fn update_job_status(&self, job_id: Uuid, status: JobStatus, progress: u8) -> AppResult<()> {
        sqlx::query("UPDATE jobs SET status = $1, progress = $2, updated_at = NOW() WHERE id = $3")
            .bind(status)
            .bind(progress.clamp(0, 100) as i32)
            .bind(job_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }


    pub async fn update_job_progress(&self, job_id: Uuid, progress: u8) -> AppResult<()> {
        sqlx::query("UPDATE jobs SET progress = $1, updated_at = NOW() WHERE id = $2")
            .bind(progress.clamp(0, 100) as i32)
            .bind(job_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_job_file_path(&self, job_id: Uuid, file_path: &str) -> AppResult<()> {
        sqlx::query("UPDATE jobs SET file_path = $1, updated_at = NOW() WHERE id = $2")
            .bind(file_path)
            .bind(job_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn fail_job(&self, job_id: Uuid, error_message: &str) -> AppResult<()> {
        sqlx::query("UPDATE jobs SET status = 'failed', progress = 100, error_message = $1, updated_at = NOW() WHERE id = $2")
            .bind(error_message)
            .bind(job_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn cancel_user_pending_jobs(&self, user_id: i64) -> AppResult<u64> {
        let result = sqlx::query(
            "UPDATE jobs SET status = 'cancelled', updated_at = NOW()
             WHERE user_id = $1 AND status IN ('pending', 'downloading', 'processing', 'uploading')"
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn get_latest_active_job(&self, user_id: i64) -> AppResult<Option<Job>> {
        let job = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs
             WHERE user_id = $1 AND status IN ('pending', 'downloading', 'processing', 'uploading')
             ORDER BY created_at DESC
             LIMIT 1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(job)
    }

    pub async fn get_user_jobs(&self, user_id: i64, limit: i64) -> AppResult<Vec<Job>> {
        let jobs = sqlx::query_as::<_, Job>("SELECT * FROM jobs WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2")
            .bind(user_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        Ok(jobs)
    }


    pub async fn get_job_status_counts(&self) -> AppResult<Vec<(String, i64)>> {
        let rows = sqlx::query(
            "SELECT status::text AS status, COUNT(*) AS count FROM jobs GROUP BY status ORDER BY status"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut counts = Vec::new();
        for row in rows {
            counts.push((row.get::<String, _>("status"), row.get::<i64, _>("count")));
        }
        Ok(counts)
    }

    pub async fn get_recent_failed_jobs(&self, limit: i64) -> AppResult<Vec<Job>> {
        let jobs = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE status = 'failed' ORDER BY updated_at DESC LIMIT $1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(jobs)
    }

    pub async fn reset_failed_job_to_pending(&self, job_id: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE jobs SET status = 'pending', progress = 0, error_message = NULL, updated_at = NOW() WHERE id = $1"
        )
        .bind(job_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_stats(&self) -> AppResult<(i64, i64)> {
        let user_count: i64 = sqlx::query("SELECT COUNT(*) FROM users").fetch_one(&self.pool).await?.get(0);
        let job_count: i64 = sqlx::query("SELECT COUNT(*) FROM jobs").fetch_one(&self.pool).await?.get(0);
        Ok((user_count, job_count))
    }
}
