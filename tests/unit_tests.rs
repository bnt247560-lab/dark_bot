use dark_bot::config::Settings;
use dark_bot::models::job::JobStatus;
use dark_bot::queue::QueueJob;
use tempfile::tempdir;
use uuid::Uuid;

fn valid_settings() -> Settings {
    Settings {
        teloxide_token: "123456:valid_test_token".to_string(),
        database_url: "postgres://user:pass@localhost:5432/dark_bot".to_string(),
        redis_url: "redis://localhost:6379".to_string(),
        log_level: "info".to_string(),
        storage_path: "./storage".to_string(),
        ffmpeg_path: "ffmpeg".to_string(),
        ffprobe_path: "ffprobe".to_string(),
        yt_dlp_path: "yt-dlp".to_string(),
        worker_count: 2,
        max_job_retries: 3,
        max_telegram_file_mb: 512,
        health_bind: "127.0.0.1:8080".to_string(),
        max_url_length: 2048,
        admin_user_ids: "111,222,333".to_string(),
        object_storage_enabled: false,
        object_storage_endpoint: String::new(),
        object_storage_region: "auto".to_string(),
        object_storage_bucket: String::new(),
        object_storage_access_key_id: String::new(),
        object_storage_secret_access_key: String::new(),
        object_storage_public_base_url: String::new(),
        dashboard_enabled: true,
        dashboard_token: "0123456789abcdef0123456789abcdef".to_string(),
    }
}

#[test]
fn settings_accept_valid_configuration() {
    let settings = valid_settings();
    settings.validate().expect("valid settings should pass startup validation");
}

#[test]
fn settings_reject_placeholder_bot_token() {
    let mut settings = valid_settings();
    settings.teloxide_token = "your_bot_token_here".to_string();

    let err = settings.validate().expect_err("placeholder token must be rejected");
    assert!(err.to_string().contains("TELOXIDE_TOKEN"));
}

#[test]
fn settings_reject_invalid_database_url() {
    let mut settings = valid_settings();
    settings.database_url = "mysql://user:pass@localhost/db".to_string();

    let err = settings.validate().expect_err("non-postgres database URLs must be rejected");
    assert!(err.to_string().contains("DATABASE_URL"));
}

#[test]
fn settings_reject_invalid_redis_url() {
    let mut settings = valid_settings();
    settings.redis_url = "http://localhost:6379".to_string();

    let err = settings.validate().expect_err("non-redis URLs must be rejected");
    assert!(err.to_string().contains("REDIS_URL"));
}

#[test]
fn settings_reject_zero_workers_and_retries() {
    let mut settings = valid_settings();
    settings.worker_count = 0;
    assert!(settings.validate().expect_err("zero workers must fail").to_string().contains("WORKER_COUNT"));

    let mut settings = valid_settings();
    settings.max_job_retries = 0;
    assert!(settings.validate().expect_err("zero retries must fail").to_string().contains("MAX_JOB_RETRIES"));
}

#[test]
fn settings_reject_invalid_admin_ids() {
    let mut settings = valid_settings();
    settings.admin_user_ids = "111,not-a-number,333".to_string();

    let err = settings.validate().expect_err("invalid admin ids must be rejected");
    assert!(err.to_string().contains("ADMIN_USER_IDS"));
}

#[test]
fn admin_ids_are_trimmed_and_checked() {
    let mut settings = valid_settings();
    settings.admin_user_ids = " 10, 20 ,30 ".to_string();

    assert_eq!(settings.admin_ids(), vec![10, 20, 30]);
    assert!(settings.is_admin(20));
    assert!(!settings.is_admin(40));
}

#[test]
fn storage_directories_are_created() {
    let temp = tempdir().expect("tempdir should be available");
    let mut settings = valid_settings();
    settings.storage_path = temp.path().join("storage").display().to_string();

    settings.ensure_storage_dirs().expect("storage dirs should be created");
    assert!(temp.path().join("storage/downloads").is_dir());
    assert!(temp.path().join("storage/processed").is_dir());
    assert!(temp.path().join("storage/temp").is_dir());
}

#[test]
fn settings_require_strong_dashboard_token_when_enabled() {
    let mut settings = valid_settings();
    settings.dashboard_enabled = true;
    settings.dashboard_token = "short".to_string();

    let err = settings.validate().expect_err("short dashboard token must be rejected");
    assert!(err.to_string().contains("DASHBOARD_TOKEN"));
}

#[test]
fn settings_allow_empty_dashboard_token_when_disabled() {
    let mut settings = valid_settings();
    settings.dashboard_enabled = false;
    settings.dashboard_token = String::new();

    settings.validate().expect("disabled dashboard should not require a token");
}

#[test]
fn settings_accept_disabled_object_storage() {
    let settings = valid_settings();
    assert!(!settings.object_storage_enabled);
    settings.validate().expect("disabled object storage should not require credentials");
}

#[test]
fn settings_require_object_storage_credentials_when_enabled() {
    let mut settings = valid_settings();
    settings.object_storage_enabled = true;

    let err = settings.validate().expect_err("enabled object storage without credentials must fail");
    assert!(err.to_string().contains("OBJECT_STORAGE_BUCKET"));

    settings.object_storage_bucket = "dark-bot-results".to_string();
    let err = settings.validate().expect_err("missing access key must fail");
    assert!(err.to_string().contains("OBJECT_STORAGE_ACCESS_KEY_ID"));

    settings.object_storage_access_key_id = "access-key".to_string();
    let err = settings.validate().expect_err("missing secret must fail");
    assert!(err.to_string().contains("OBJECT_STORAGE_SECRET_ACCESS_KEY"));

    settings.object_storage_secret_access_key = "secret-key".to_string();
    let err = settings.validate().expect_err("missing public base URL must fail");
    assert!(err.to_string().contains("OBJECT_STORAGE_PUBLIC_BASE_URL"));
}

#[test]
fn settings_reject_invalid_object_storage_endpoint() {
    let mut settings = valid_settings();
    settings.object_storage_enabled = true;
    settings.object_storage_bucket = "dark-bot-results".to_string();
    settings.object_storage_access_key_id = "access-key".to_string();
    settings.object_storage_secret_access_key = "secret-key".to_string();
    settings.object_storage_public_base_url = "https://cdn.example.com".to_string();
    settings.object_storage_endpoint = "not-a-url".to_string();

    let err = settings.validate().expect_err("invalid object storage endpoint must fail");
    assert!(err.to_string().contains("OBJECT_STORAGE_ENDPOINT"));
}

#[test]
fn queue_job_serializes_and_deserializes() {
    let job_id = Uuid::new_v4();
    let payload = QueueJob::new(job_id);

    let raw = payload.to_json().expect("queue payload should serialize");
    let decoded = QueueJob::from_json(&raw).expect("queue payload should deserialize");

    assert_eq!(decoded, payload);
    assert_eq!(decoded.job_id, job_id);
    assert_eq!(decoded.attempt, 0);
}

#[test]
fn queue_job_retry_increments_attempt_without_changing_id() {
    let payload = QueueJob::new(Uuid::new_v4());
    let retry = payload.next_attempt();

    assert_eq!(retry.job_id, payload.job_id);
    assert_eq!(retry.attempt, payload.attempt + 1);
}

#[test]
fn queue_job_rejects_invalid_json() {
    let err = QueueJob::from_json("{not-json}").expect_err("invalid JSON must fail");
    assert!(err.to_string().contains("deserialize"));
}

#[test]
fn job_status_serde_uses_lowercase_values() {
    let status = serde_json::to_string(&JobStatus::Processing).expect("status should serialize");
    assert_eq!(status, "\"processing\"");

    let decoded: JobStatus = serde_json::from_str("\"cancelled\"").expect("status should deserialize");
    assert_eq!(decoded, JobStatus::Cancelled);
}
