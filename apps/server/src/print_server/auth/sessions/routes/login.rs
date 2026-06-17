use std::sync::Arc;

use axum::{
    extract::State,
    http::{
        header::{SET_COOKIE, USER_AGENT},
        HeaderMap,
    },
    Json,
};

use super::super::super::super::{
    models::{ApiError, AuthLoginRequest, AuthLoginResponse},
    sha256_hex,
    utils::{clamp_u64_to_i64, now_unix},
    AgentState, USER_STATUS_ACTIVE,
};
use super::super::super::core::{
    auth_user_response, build_session_cookie, generate_session_token, header_string,
    normalize_auth_provider_key, validate_auth_login_payload, verify_password,
};
use crate::storage::{insert_auth_session_at_path, load_local_auth_identity_at_path};

pub(crate) async fn auth_login(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    Json(payload): Json<AuthLoginRequest>,
) -> Result<(HeaderMap, Json<AuthLoginResponse>), ApiError> {
    validate_auth_login_payload(&payload)?;

    let provider_key = normalize_auth_provider_key(&payload.username)?;
    let identity = load_local_auth_identity_at_path(state.db_path.as_ref(), &provider_key)?
        .ok_or_else(|| ApiError::Unauthorized("invalid username or password".to_string()))?;

    if identity.user.status != USER_STATUS_ACTIVE {
        return Err(ApiError::Unauthorized("user is disabled".to_string()));
    }

    if !verify_password(&payload.password, &identity.password_hash)? {
        return Err(ApiError::Unauthorized(
            "invalid username or password".to_string(),
        ));
    }

    let now = now_unix();
    let expires_at = now.saturating_add(clamp_u64_to_i64(state.config.auth_session_ttl_sec));
    let session_token = generate_session_token();
    insert_auth_session_at_path(
        state.db_path.as_ref(),
        &identity.user.id,
        &sha256_hex(session_token.as_bytes()),
        now,
        expires_at,
        header_string(&headers, "x-forwarded-for"),
        header_string(&headers, USER_AGENT.as_str()),
    )?;

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        SET_COOKIE,
        build_session_cookie(
            &state.config,
            &session_token,
            state.config.auth_session_ttl_sec,
        )?,
    );

    Ok((
        response_headers,
        Json(AuthLoginResponse {
            authenticated: true,
            user: auth_user_response(identity.user),
            expires_at,
        }),
    ))
}
