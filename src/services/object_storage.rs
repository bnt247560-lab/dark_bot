use crate::config::Settings;
use crate::errors::{AppError, AppResult};
use aws_credential_types::Credentials;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use std::path::Path;

#[derive(Clone)]
pub struct ObjectStorage {
    enabled: bool,
    bucket: String,
    public_base_url: String,
    client: Option<Client>,
}

#[derive(Debug, Clone)]
pub struct StoredObject {
    pub key: String,
    pub public_url: String,
}

impl ObjectStorage {
    pub async fn from_settings(settings: &Settings) -> AppResult<Self> {
        if !settings.object_storage_enabled {
            return Ok(Self {
                enabled: false,
                bucket: String::new(),
                public_base_url: String::new(),
                client: None,
            });
        }

        let credentials = Credentials::new(
            settings.object_storage_access_key_id.clone(),
            settings.object_storage_secret_access_key.clone(),
            None,
            None,
            "dark_bot_env",
        );

        let region = Region::new(settings.object_storage_region.clone());
        let mut builder = aws_sdk_s3::config::Builder::new()
            .region(region)
            .credentials_provider(credentials)
            .force_path_style(true);

        if !settings.object_storage_endpoint.trim().is_empty() {
            builder = builder.endpoint_url(settings.object_storage_endpoint.clone());
        }

        let client = Client::from_conf(builder.build());

        Ok(Self {
            enabled: true,
            bucket: settings.object_storage_bucket.clone(),
            public_base_url: settings.object_storage_public_base_url.trim_end_matches('/').to_string(),
            client: Some(client),
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub async fn upload_processed_file(&self, job_id: uuid::Uuid, file_path: &Path) -> AppResult<Option<StoredObject>> {
        if !self.enabled {
            return Ok(None);
        }

        let file_name = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| AppError::Validation("Processed file has no valid file name".into()))?;
        let key = format!("processed/{job_id}/{file_name}");
        self.upload_file(&key, file_path).await?;
        let public_url = format!("{}/{}", self.public_base_url, key);

        Ok(Some(StoredObject { key, public_url }))
    }

    async fn upload_file(&self, key: &str, file_path: &Path) -> AppResult<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| AppError::Internal("Object storage client is not initialized".into()))?;

        let body = ByteStream::from_path(file_path)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to open file for object storage upload: {e}")))?;

        client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Object storage upload failed: {e}")))?;

        Ok(())
    }
}
