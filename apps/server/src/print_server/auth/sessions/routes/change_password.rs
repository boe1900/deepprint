use std::sync::Arc;

use axum::{extract::State, http::HeaderMap, Json};

use super::super::super::super::{
    models::{ApiError, AuthChangePasswordRequest, AuthChangePasswordResponse},
    sha256_hex,
    utils::now_unix,
    AgentState,
};
use super::super::super::core::{
    auth_user_response, hash_password, session_token_from_headers,
    validate_auth_change_password_payload, verify_password,
};
use crate::storage::{
    load_auth_session_at_path, load_auth_user_at_path, load_local_auth_identity_by_user_id_at_path,
    touch_auth_session_at_path, update_local_auth_password_at_path,
};

pub(crate) async fn auth_change_password(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    Json(payload): Json<AuthChangePasswordRequest>,
) -> Result<Json<AuthChangePasswordResponse>, ApiError> {
    validate_auth_change_password_payload(&payload)?;

    let Some(session_token) =
        session_token_from_headers(&headers, &state.config.auth_session_cookie_name)
    else {
        return Err(ApiError::Unauthorized("not authenticated".to_string()));
    };

    let session_token_hash = sha256_hex(session_token.as_bytes());
    let now = now_unix();
    let session = load_auth_session_at_path(state.db_path.as_ref(), &session_token_hash, now)?
        .ok_or_else(|| ApiError::Unauthorized("not authenticated".to_string()))?;
    let identity =
        load_local_auth_identity_by_user_id_at_path(state.db_path.as_ref(), &session.user.id)?
            .ok_or_else(|| ApiError::Unauthorized("local password is not available".to_string()))?;

    if !verify_password(&payload.current_password, &identity.password_hash)? {
        return Err(ApiError::Unauthorized(
            "current password is invalid".to_string(),
        ));
    }

    let password_hash = hash_password(&payload.new_password)
        .map_err(|err| ApiError::Internal(format!("hash password failed: {err}")))?;
    update_local_auth_password_at_path(
        state.db_path.as_ref(),
        &session.user.id,
        &password_hash,
        now,
        &session_token_hash,
    )?;
    touch_auth_session_at_path(state.db_path.as_ref(), &session_token_hash, now)?;

    let updated_user = load_auth_user_at_path(state.db_path.as_ref(), &session.user.id)?
        .ok_or_else(|| ApiError::Unauthorized("user is disabled".to_string()))?;

    Ok(Json(AuthChangePasswordResponse {
        changed: true,
        user: auth_user_response(updated_user),
        expires_at: session.expires_at,
    }))
}
