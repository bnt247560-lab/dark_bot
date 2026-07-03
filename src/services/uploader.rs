use crate::errors::{AppError, AppResult};
use std::path::Path;
use teloxide::prelude::*;
use teloxide::types::{InputFile, MessageId};

pub struct Uploader {
    bot: Bot,
}

impl Uploader {
    pub fn new(bot: Bot) -> Self {
        Self { bot }
    }

    pub async fn send_text(&self, chat_id: ChatId, text: &str) -> AppResult<()> {
        self.bot
            .send_message(chat_id, text)
            .await
            .map_err(|e| AppError::Bot(e.to_string()))?;
        Ok(())
    }

    pub async fn send_text_message_id(&self, chat_id: ChatId, text: &str) -> AppResult<MessageId> {
        let message = self
            .bot
            .send_message(chat_id, text)
            .await
            .map_err(|e| AppError::Bot(e.to_string()))?;
        Ok(message.id)
    }

    pub async fn edit_text(&self, chat_id: ChatId, message_id: MessageId, text: &str) -> AppResult<()> {
        self.bot
            .edit_message_text(chat_id, message_id, text)
            .await
            .map_err(|e| AppError::Bot(e.to_string()))?;
        Ok(())
    }

    pub async fn upload_file(&self, chat_id: ChatId, file_path: &Path) -> AppResult<()> {
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        match ext.as_str() {
            "mp4" | "mov" | "mkv" | "webm" => self.upload_video(chat_id, file_path).await,
            _ => self.upload_document(chat_id, file_path).await,
        }
    }

    pub async fn upload_video(&self, chat_id: ChatId, file_path: &Path) -> AppResult<()> {
        self.bot
            .send_video(chat_id, InputFile::file(file_path))
            .await
            .map_err(|e| AppError::Bot(e.to_string()))?;
        Ok(())
    }

    pub async fn upload_document(&self, chat_id: ChatId, file_path: &Path) -> AppResult<()> {
        self.bot
            .send_document(chat_id, InputFile::file(file_path))
            .await
            .map_err(|e| AppError::Bot(e.to_string()))?;
        Ok(())
    }
}
