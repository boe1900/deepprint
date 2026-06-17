use std::path::Path;

use axum::http::{header::AUTHORIZATION, HeaderMap, StatusCode};
#[cfg(test)]
use rusqlite::Connection;
use uuid::Uuid;

use super::super::super::{models::ApiError, sha256_hex, utils::now_unix, API_KEY_STATUS_ACTIVE};
use super::responses::api_key_scopes;
use crate::storage::{touch_api_key_at_path, ApiKeyRecord};

pub(crate) fn generate_api_key_token() -> (String, String) {
    let prefix_source = Uuid::new_v4().simple().to_string();
    let key_prefix = prefix_source[..12].to_string();
    let secret = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    (key_prefix.clone(), format!("dp_{key_prefix}_{secret}"))
}

pub(crate) fn parse_bearer_token(headers: &HeaderMap) -> Result<String, ApiError> {
    let value = headers
        .get(AUTHORIZATION)
        .ok_or_else(|| ApiError::Unauthorized("missing bearer token".to_string()))?
        .to_str()
        .map_err(|_| ApiError::Unauthorized("invalid bearer token".to_string()))?
        .trim();
    let Some(token) = value.strip_prefix("Bearer ") else {
        return Err(ApiError::Unauthorized("missing bearer token".to_string()));
    };
    let token = token.trim();
    if token.is_empty() {
        return Err(ApiError::Unauthorized("missing bearer token".to_string()));
    }
    Ok(token.to_string())
}

pub(crate) fn api_key_prefix_from_token(token: &str) -> Option<String> {
    let rest = token.strip_prefix("dp_")?;
    let (prefix, _) = rest.split_once('_')?;
    if prefix.is_empty() {
        return None;
    }
    Some(prefix.to_string())
}

pub(crate) fn require_api_key_scope_for_path(
    db_path: &Path,
    headers: &HeaderMap,
    required_scope: &str,
) -> Result<ApiKeyRecord, ApiError> {
    let api_key = require_api_key_for_path(db_path, headers)?;
    ensure_api_key_scope(&api_key, required_scope)?;
    Ok(api_key)
}

pub(crate) fn require_api_key_for_path(
    db_path: &Path,
    headers: &HeaderMap,
) -> Result<ApiKeyRecord, ApiError> {
    let token = parse_bearer_token(headers)?;
    let key_prefix = api_key_prefix_from_token(&token)
        .ok_or_else(|| ApiError::Unauthorized("invalid api key".to_string()))?;
    let token_hash = sha256_hex(token.as_bytes());
    let Some(api_key) =
        crate::storage::load_api_key_by_prefix_and_hash_at_path(db_path, &key_prefix, &token_hash)?
    else {
        return Err(ApiError::Unauthorized("invalid api key".to_string()));
    };

    ensure_api_key_active(&api_key)?;
    touch_api_key_at_path(db_path, &api_key.id, now_unix())?;
    Ok(api_key)
}

#[cfg(test)]
pub(crate) fn require_api_key_scope(
    conn: &Connection,
    headers: &HeaderMap,
    required_scope: &str,
) -> Result<ApiKeyRecord, ApiError> {
    let token = parse_bearer_token(headers)?;
    let key_prefix = api_key_prefix_from_token(&token)
        .ok_or_else(|| ApiError::Unauthorized("invalid api key".to_string()))?;
    let token_hash = sha256_hex(token.as_bytes());
    let Some(api_key) =
        crate::storage::load_api_key_by_prefix_and_hash(conn, &key_prefix, &token_hash)?
    else {
        return Err(ApiError::Unauthorized("invalid api key".to_string()));
    };

    ensure_api_key_scope(&api_key, required_scope)?;
    crate::storage::touch_api_key(conn, &api_key.id, now_unix())?;
    Ok(api_key)
}

fn ensure_api_key_scope(api_key: &ApiKeyRecord, required_scope: &str) -> Result<(), ApiError> {
    ensure_api_key_active(api_key)?;

    let scopes = api_key_scopes(api_key)?;
    if !scopes.iter().any(|scope| scope == required_scope) {
        return Err(ApiError::structured(
            StatusCode::FORBIDDEN,
            "API_KEY_SCOPE_REQUIRED",
            format!("api key scope required: {required_scope}"),
        ));
    }

    Ok(())
}

fn ensure_api_key_active(api_key: &ApiKeyRecord) -> Result<(), ApiError> {
    if api_key.status != API_KEY_STATUS_ACTIVE {
        return Err(ApiError::Unauthorized("api key is revoked".to_string()));
    }
    if let Some(expires_at) = api_key.expires_at {
        if expires_at <= now_unix() {
            return Err(ApiError::Unauthorized("api key is expired".to_string()));
        }
    }

    Ok(())
}
