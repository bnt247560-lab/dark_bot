use crate::app::AppState;
use crate::errors::{AppError, AppResult};
use crate::models::{Job, JobStatus, VideoProcessingOptions};
use crate::queue::QueueJob;
use crate::services::progress::{scale_progress, ProgressCallback};
use crate::utils::temp::{cleanup_job_dir, create_job_dir};
use std::sync::Arc;
use teloxide::prelude::*;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

pub struct Worker {
    id: u32,
    state: Arc<AppState>,
}

impl Worker {
    pub fn new(id: u32, state: Arc<AppState>) -> Self {
        Self { id, state }
    }

    pub async fn run(self) {
        tracing::info!(worker_id = self.id, "Worker started");
        self.state.metrics.record_worker_event("started");

        loop {
            if self.state.shutdown.is_cancelled() {
                tracing::info!(worker_id = self.id, "Worker received shutdown signal");
                self.state.metrics.record_worker_event("shutdown");
                break;
            }

            match self.state.queue.dequeue().await {
                Ok(Some((payload, raw_payload))) => {
                    self.state.metrics.record_worker_event("job_claimed");
                    let result = self.process_payload(payload.clone()).await;

                    match result {
                        Ok(()) => {
                            self.state.metrics.record_job_completed();
                            if let Err(err) = self.state.queue.ack(&raw_payload).await {
                                tracing::error!(worker_id = self.id, error = %err, "Failed to ack job");
                            }
                        }
                        Err(err) => {
                            tracing::error!(worker_id = self.id, job_id = %payload.job_id, error = %err, "Job failed");
                            let max_retries = self.state.config.max_job_retries;
                            if payload.attempt + 1 >= max_retries {
                                self.state.metrics.record_job_failed("final");
                                let _ = self.state.db.fail_job(payload.job_id, &err.to_string()).await;
                                let _ = self.state.queue.dead_letter(&raw_payload).await;
                                let _ = self.notify_user_failure(payload.job_id, &err.to_string()).await;
                            } else {
                                let _ = self.state.queue.ack(&raw_payload).await;
                                sleep(Duration::from_secs(2_u64.pow(payload.attempt.min(5)))).await;
                                self.state.metrics.record_job_retried();
                                let _ = self.state.queue.retry(&payload).await;
                            }
                        }
                    }
                }
                Ok(None) => {
                    sleep(Duration::from_millis(250)).await;
                }
                Err(err) => {
                    self.state.metrics.record_worker_event("queue_error");
                    tracing::error!(worker_id = self.id, error = %err, "Queue dequeue error");
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    async fn process_payload(&self, payload: QueueJob) -> AppResult<()> {
        let job = self.state.db.get_job(payload.job_id).await?;

        if job.status == JobStatus::Cancelled {
            return Ok(());
        }

        self.process_job(job).await
    }

    async fn process_job(&self, job: Job) -> AppResult<()> {
        let chat_id = ChatId(job.user_id);
        let progress_message_id = self
            .state
            .uploader
            .send_text_message_id(chat_id, "📊 تم بدء المهمة... 0%")
            .await?;

        self.state.db.update_job_status(job.id, JobStatus::Downloading, 5).await?;

        let download_progress = self.make_progress_callback(job.id, chat_id, progress_message_id, 5, 40, "⏬");
        let downloaded_path = if let Some(existing_path) = job.file_path.as_deref() {
            self.state.db.update_job_progress(job.id, 40).await?;
            let _ = self
                .state
                .uploader
                .edit_text(chat_id, progress_message_id, "⏬ الفيديو مرفوع مسبقاً. الانتقال إلى المعالجة... 40%")
                .await;
            std::path::PathBuf::from(existing_path)
        } else {
            let source_url = job
                .source_url
                .as_deref()
                .ok_or_else(|| AppError::Validation("Job has neither source URL nor uploaded file path".into()))?;

            let job_dir = create_job_dir(&self.state.config.storage_path, job.id).await?;
            let path = match self
                .state
                .downloader
                .download_with_progress(source_url, &job_dir, Some(download_progress))
                .await
            {
                Ok(path) => path,
                Err(err) => {
                    let _ = cleanup_job_dir(&self.state.config.storage_path, job.id).await;
                    return Err(err);
                }
            };

            self.state
                .db
                .update_job_file_path(job.id, &path.to_string_lossy())
                .await?;
            path
        };

        if !downloaded_path.exists() {
            return Err(AppError::Download(format!(
                "Input file not found: {}",
                downloaded_path.display()
            )));
        }

        self.ensure_not_cancelled(job.id).await?;
        self.state.db.update_job_status(job.id, JobStatus::Processing, 45).await?;
        let _ = self
            .state
            .uploader
            .edit_text(chat_id, progress_message_id, "⚙️ بدأت معالجة الفيديو... 45%")
            .await;

        let job_for_processing = self.state.db.get_job(job.id).await?;
        let options = VideoProcessingOptions {
            remove_metadata: true,
            compress: false,
            extract_audio: false,
            target_format: None,
        };
        let processing_progress = self.make_progress_callback(job.id, chat_id, progress_message_id, 45, 82, "⚙️");
        let output_path = self
            .state
            .processor
            .process_with_progress(&job_for_processing, options, Some(processing_progress))
            .await?;

        self.ensure_not_cancelled(job.id).await?;
        self.state.db.update_job_status(job.id, JobStatus::Uploading, 85).await?;
        let _ = self
            .state
            .uploader
            .edit_text(chat_id, progress_message_id, "📤 بدأ رفع النتيجة... 85%")
            .await;

        let stored_object = if self.state.object_storage.is_enabled() {
            let _ = self
                .state
                .uploader
                .edit_text(chat_id, progress_message_id, "☁️ جارٍ حفظ نسخة احتياطية في التخزين السحابي... 88%")
                .await;
            self.state
                .object_storage
                .upload_processed_file(job.id, &output_path)
                .await?
        } else {
            None
        };

        self.state.uploader.upload_file(chat_id, &output_path).await?;

        if let Some(object) = stored_object {
            let text = format!(
                "☁️ تم حفظ نسخة سحابية للنتيجة.\n\nالرابط: {}\nالمسار: {}",
                object.public_url, object.key
            );
            let _ = self.state.uploader.send_text(chat_id, &text).await;
        }

        self.state.db.update_job_status(job.id, JobStatus::Completed, 100).await?;
        let _ = self
            .state
            .uploader
            .edit_text(chat_id, progress_message_id, "✅ اكتملت المعالجة بنجاح. 100%")
            .await;
        cleanup_job_dir(&self.state.config.storage_path, job.id).await?;

        Ok(())
    }

    fn make_progress_callback(
        &self,
        job_id: uuid::Uuid,
        chat_id: ChatId,
        message_id: teloxide::types::MessageId,
        from: u8,
        to: u8,
        emoji: &'static str,
    ) -> ProgressCallback {
        let state = self.state.clone();
        let last_progress = Arc::new(Mutex::new(from.saturating_sub(5)));
        Arc::new(move |raw_progress, label| {
            let state = state.clone();
            let last_progress = last_progress.clone();
            Box::pin(async move {
                let scaled = scale_progress(raw_progress, from, to);
                let mut last = last_progress.lock().await;
                if scaled < last.saturating_add(4) && scaled != to {
                    return;
                }
                *last = scaled;
                drop(last);

                let _ = state.db.update_job_progress(job_id, scaled).await;
                let text = format!("{emoji} {label}\n\n📊 التقدم: {scaled}%");
                let _ = state.uploader.edit_text(chat_id, message_id, &text).await;
            })
        })
    }

    async fn ensure_not_cancelled(&self, job_id: uuid::Uuid) -> AppResult<()> {
        let job = self.state.db.get_job(job_id).await?;
        if job.status == JobStatus::Cancelled {
            return Err(AppError::Validation("Job was cancelled".into()));
        }
        Ok(())
    }

    async fn notify_user_failure(&self, job_id: uuid::Uuid, error: &str) -> AppResult<()> {
        if let Ok(job) = self.state.db.get_job(job_id).await {
            let message = format!("❌ فشلت معالجة الطلب بعد عدة محاولات.\n\nالسبب: {error}");
            self.state.uploader.send_text(ChatId(job.user_id), &message).await?;
        }
        Ok(())
    }
}
