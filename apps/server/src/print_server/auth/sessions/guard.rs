use std::{path::Path, sync::Arc};

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Request, StatusCode},
    middleware::Next,
    response::Response,
};

use super::super::super::{
    config::AgentConfig, models::ApiError, sha256_hex, utils::now_unix, AgentState,
    AuthSessionRecord, USER_ROLE_ADMIN,
};
use super::super::core::session_token_from_headers;
use crate::storage::{
    has_active_local_auth_user_at_path, load_auth_session_at_path, touch_auth_session_at_path,
};

pub(crate) fn require_admin_session_for_path(
    db_path: &Path,
    config: &AgentConfig,
    headers: &HeaderMap,
) -> Result<AuthSessionRecord, ApiError> {
    let session = load_session_from_headers(db_path, headers, &config.auth_session_cookie_name)?;
    if session.user.must_change_password {
        return Err(ApiError::structured(
            StatusCode::FORBIDDEN,
            "PASSWORD_CHANGE_REQUIRED",
            "password change required",
        ));
    }
    if session.user.role != USER_ROLE_ADMIN {
        return Err(ApiError::structured(
            StatusCode::FORBIDDEN,
            "ADMIN_REQUIRED",
            "admin permission required",
        ));
    }
    Ok(session)
}

pub(crate) async fn enforce_console_session(
    State(state): State<Arc<AgentState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    if is_public_console_api_request(req.method(), req.uri().path()) {
        return Ok(next.run(req).await);
    }

    if !has_active_local_auth_user_at_path(state.db_path.as_ref())? {
        return Ok(next.run(req).await);
    }

    let Some(session_token) =
        session_token_from_headers(req.headers(), &state.config.auth_session_cookie_name)
    else {
        return Err(ApiError::Unauthorized("not authenticated".to_string()));
    };

    let session_token_hash = sha256_hex(session_token.as_bytes());
    let now = now_unix();
    let session = load_auth_session_at_path(state.db_path.as_ref(), &session_token_hash, now)?
        .ok_or_else(|| ApiError::Unauthorized("not authenticated".to_string()))?;

    if session.user.must_change_password {
        return Err(ApiError::structured(
            StatusCode::FORBIDDEN,
            "PASSWORD_CHANGE_REQUIRED",
            "password change required",
        ));
    }

    touch_auth_session_at_path(state.db_path.as_ref(), &session_token_hash, now)?;
    Ok(next.run(req).await)
}

fn load_session_from_headers(
    db_path: &Path,
    headers: &HeaderMap,
    cookie_name: &str,
) -> Result<AuthSessionRecord, ApiError> {
    let Some(session_token) = session_token_from_headers(headers, cookie_name) else {
        return Err(ApiError::Unauthorized("not authenticated".to_string()));
    };

    load_auth_session_at_path(db_path, &sha256_hex(session_token.as_bytes()), now_unix())?
        .ok_or_else(|| ApiError::Unauthorized("not authenticated".to_string()))
}

fn is_public_console_api_request(method: &Method, path: &str) -> bool {
    if *method == Method::OPTIONS {
        return true;
    }
    path == "/v1/health" || path.starts_with("/v1/auth/") || path.starts_with("/v1/open/")
}
