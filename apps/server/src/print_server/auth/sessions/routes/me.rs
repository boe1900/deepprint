use std::sync::Arc;

use axum::{extract::State, http::HeaderMap, Json};

use super::super::super::super::{
    models::{ApiError, AuthMeResponse},
    sha256_hex,
    utils::now_unix,
    AgentState,
};
use super::super::super::core::{auth_user_response, session_token_from_headers};
use crate::storage::{
    has_active_local_auth_user_at_path, load_auth_session_at_path, touch_auth_session_at_path,
};

pub(crate) async fn auth_me(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<AuthMeResponse>, ApiError> {
    let login_enabled = has_active_local_auth_user_at_path(state.db_path.as_ref())?;

    let Some(session_token) =
        session_token_from_headers(&headers, &state.config.auth_session_cookie_name)
    else {
        return Ok(Json(unauthenticated_me_response(login_enabled)));
    };

    let session_token_hash = sha256_hex(session_token.as_bytes());
    let session =
        load_auth_session_at_path(state.db_path.as_ref(), &session_token_hash, now_unix())?;
    let Some(session) = session else {
        return Ok(Json(unauthenticated_me_response(login_enabled)));
    };

    touch_auth_session_at_path(state.db_path.as_ref(), &session_token_hash, now_unix())?;

    Ok(Json(AuthMeResponse {
        authenticated: true,
        login_enabled,
        user: Some(auth_user_response(session.user)),
        expires_at: Some(session.expires_at),
    }))
}

fn unauthenticated_me_response(login_enabled: bool) -> AuthMeResponse {
    AuthMeResponse {
        authenticated: false,
        login_enabled,
        user: None,
        expires_at: None,
    }
}
