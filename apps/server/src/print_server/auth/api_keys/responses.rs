use super::super::super::models::{ApiError, ApiKeyResponseItem};
use crate::storage::ApiKeyRecord;

pub(crate) fn api_key_scopes(api_key: &ApiKeyRecord) -> Result<Vec<String>, ApiError> {
    serde_json::from_str::<Vec<String>>(&api_key.scopes_json)
        .map_err(|err| ApiError::Internal(format!("stored api key scopes are invalid: {err}")))
}

pub(crate) fn api_key_response_item(api_key: ApiKeyRecord) -> Result<ApiKeyResponseItem, ApiError> {
    let scopes = api_key_scopes(&api_key)?;
    Ok(ApiKeyResponseItem {
        id: api_key.id,
        name: api_key.name,
        key_prefix: api_key.key_prefix,
        scopes,
        status: api_key.status,
        created_by_user_id: api_key.created_by_user_id,
        created_at: api_key.created_at,
        updated_at: api_key.updated_at,
        last_used_at: api_key.last_used_at,
        revoked_at: api_key.revoked_at,
        expires_at: api_key.expires_at,
    })
}
