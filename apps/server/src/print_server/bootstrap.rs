use std::path::Path;

use tracing::{info, warn};

use super::{
    auth::{
        hash_password, normalize_auth_provider_key, normalize_auth_text,
        normalize_optional_auth_text,
    },
};
use crate::storage::seed_initial_admin_user_if_no_users;

pub(super) fn seed_initial_admin_if_configured(db_path: &Path, now: i64) -> Result<(), String> {
    let Ok(raw_password) = std::env::var("DEEPPRINT_INITIAL_ADMIN_PASSWORD") else {
        warn!("no users exist; set DEEPPRINT_INITIAL_ADMIN_PASSWORD to bootstrap the first admin");
        return Ok(());
    };
    let password = raw_password.trim();
    if password.is_empty() {
        warn!("DEEPPRINT_INITIAL_ADMIN_PASSWORD is empty; skip initial admin bootstrap");
        return Ok(());
    }
    if password.len() < 8 {
        return Err("DEEPPRINT_INITIAL_ADMIN_PASSWORD must be at least 8 characters".to_string());
    }

    let username = std::env::var("DEEPPRINT_INITIAL_ADMIN_USERNAME")
        .ok()
        .and_then(|value| normalize_auth_text(&value, 128).ok())
        .unwrap_or_else(|| "admin".to_string());
    let provider_key = normalize_auth_provider_key(&username).map_err(|err| format!("{err}"))?;
    let email = std::env::var("DEEPPRINT_INITIAL_ADMIN_EMAIL")
        .ok()
        .and_then(|value| normalize_optional_auth_text(&value, 320));
    let display_name = std::env::var("DEEPPRINT_INITIAL_ADMIN_DISPLAY_NAME")
        .ok()
        .and_then(|value| normalize_optional_auth_text(&value, 128))
        .unwrap_or_else(|| username.clone());
    let password_hash = hash_password(password)?;
    let seeded = seed_initial_admin_user_if_no_users(
        db_path,
        &username,
        &provider_key,
        email,
        &display_name,
        &password_hash,
        now,
    )
    .map_err(|err| err.to_string())?;

    if seeded {
        info!("bootstrapped initial admin user");
    }
    Ok(())
}
