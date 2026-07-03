use crate::config::Settings;

pub fn is_admin_user(settings: &Settings, user_id: i64) -> bool {
    settings.is_admin(user_id)
}
