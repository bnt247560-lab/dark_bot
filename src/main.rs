mod downloader;
mod processor;

use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use uuid::Uuid;
use std::fs;
use std::path::Path;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "أوامر البوت الظلامي:")]
enum Command {
    #[command(description = "بدء استخدام البوت")]
    Start,
    #[command(description = "مساعدة")]
    Help,
}

#[tokio::main]
async fn main() {
    // التأكد من وجود مجلد التحميلات
    if !Path::new("downloads").exists() {
        fs::create_dir("downloads").expect("فشل في إنشاء مجلد downloads");
    }

    let bot = Bot::from_env();

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        // التحقق من وجود نص
        if let Some(text) = msg.text() {
            // معالجة الأوامر
            if text.starts_with('/') {
                match Command::parse(text, "bot") {
                    Ok(Command::Start) => {
                        bot.send_message(msg.chat.id, "أرسل رابط الفيديو وسأقوم بتطهيره لك فوراً.").await?;
                    }
                    _ => {
                        bot.send_message(msg.chat.id, "استخدم الأوامر الصحيحة أو أرسل الرابط مباشرة.").await?;
                    }
                }
                return respond(());
            }

            // معالجة الروابط (تطهير الفيديو)
            let id = Uuid::new_v4().to_string();
            let raw_path = format!("downloads/{}_raw.mp4", id);
            let clean_path = format!("downloads/{}_clean.mp4", id);

            let status_msg = bot.send_message(msg.chat.id, "⏳ جاري السحب والتطهير الظلامي...").await?;

            // 1. الجلب
            if downloader::download_video(text, &raw_path) {
                // 2. التطهير
                match processor::clean_video(&raw_path, &clean_path) {
                    Ok(_) => {
                        bot.send_video(msg.chat.id, InputFile::file(&clean_path)).await?;
                    }
                    Err(e) => {
                        bot.send_message(msg.chat.id, format!("❌ فشل التطهير: {}", e)).await?;
                    }
                }
            } else {
                bot.send_message(msg.chat.id, "⚠️ فشل جلب الفيديو. تأكد من صحة الرابط.").await?;
            }

            // 3. التنظيف النهائي للملفات
            let _ = fs::remove_file(&raw_path);
            let _ = fs::remove_file(&clean_path);
            let _ = bot.delete_message(msg.chat.id, status_msg.id).await;
        }
        respond(())
    }).await;
}