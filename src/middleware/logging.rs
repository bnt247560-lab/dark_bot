use teloxide::prelude::*;

pub async fn log_update(upd: Update) -> Update {
    tracing::debug!("Received update: {:?}", upd);
    upd
}
