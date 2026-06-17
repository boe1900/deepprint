use std::path::Path;

#[cfg(test)]
use rusqlite::Connection;

use super::super::super::{
    models::{ApiError, CreateApiKeyRequest},
    sha256_hex,
    utils::now_unix,
    API_KEY_STATUS_ACTIVE,
};
use super::super::core::validate_api_key_name;
use super::{scopes::normalize_api_key_scopes, tokens::generate_api_key_token};
use crate::storage::{insert_api_key_record_at_path, ApiKeyRecord, ApiKeyRecordInput};

pub(crate) fn create_api_key_record_at_path(
    db_path: &Path,
    payload: &CreateApiKeyRequest,
    created_by_user_id: &str,
) -> Result<(ApiKeyRecord, String), ApiError> {
    let input = build_api_key_record_input(payload, created_by_user_id)?;
    let token = input.token.clone();
    let api_key = insert_api_key_record_at_path(db_path, input.record)?;
    Ok((api_key, token))
}

#[cfg(test)]
pub(crate) fn insert_api_key_record(
    conn: &Connection,
    payload: &CreateApiKeyRequest,
    created_by_user_id: &str,
) -> Result<(ApiKeyRecord, String), ApiError> {
    let input = build_api_key_record_input(payload, created_by_user_id)?;
    let token = input.token.clone();
    let api_key = crate::storage::insert_api_key_record(conn, input.record)?;
    Ok((api_key, token))
}

#[cfg(test)]
pub(crate) fn revoke_api_key_record(
    conn: &Connection,
    api_key_id: &str,
    revoked_at: i64,
) -> Result<ApiKeyRecord, ApiError> {
    crate::storage::revoke_api_key_record(conn, api_key_id, revoked_at)?
        .ok_or_else(|| ApiError::NotFound(format!("api key not found: {api_key_id}")))
}

fn build_api_key_record_input(
    payload: &CreateApiKeyRequest,
    created_by_user_id: &str,
) -> Result<GeneratedApiKeyRecordInput, ApiError> {
    let name = validate_api_key_name(&payload.name)?;
    let scopes = normalize_api_key_scopes(&payload.scopes)?;
    validate_api_key_expiration(payload.expires_at)?;

    let now = now_unix();
    let (key_prefix, token) = generate_api_key_token();
    let secret_hash = sha256_hex(token.as_bytes());
    let scopes_json = serde_json::to_string(&scopes)
        .map_err(|err| ApiError::Internal(format!("serialize api key scopes failed: {err}")))?;

    Ok(GeneratedApiKeyRecordInput {
        token,
        record: ApiKeyRecordInput {
            name,
            key_prefix,
            secret_hash,
            scopes_json,
            status: API_KEY_STATUS_ACTIVE.to_string(),
            created_by_user_id: Some(created_by_user_id.to_string()),
            created_at: now,
            expires_at: payload.expires_at,
        },
    })
}

fn validate_api_key_expiration(expires_at: Option<i64>) -> Result<(), ApiError> {
    if let Some(expires_at) = expires_at {
        if expires_at <= now_unix() {
            return Err(ApiError::BadRequest(
                "expires_at must be in the future".to_string(),
            ));
        }
    }
    Ok(())
}

struct GeneratedApiKeyRecordInput {
    token: String,
    record: ApiKeyRecordInput,
}
