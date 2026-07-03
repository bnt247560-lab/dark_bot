use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::{Type, FromRow};

#[derive(Debug, Serialize, Deserialize, Type, Clone, PartialEq, Eq)]
#[sqlx(type_name = "job_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Downloading,
    Processing,
    Uploading,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Job {
    pub id: Uuid,
    pub user_id: i64,
    pub status: JobStatus,
    pub progress: i32,
    pub source_url: Option<String>,
    pub file_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error_message: Option<String>,
}
