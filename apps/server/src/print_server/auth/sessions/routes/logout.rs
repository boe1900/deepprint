use std::sync::Arc;

use axum::{
    extract::State,
    http::{header::SET_COOKIE, HeaderMap},
    Json,
};

use super::super::super::super::{
    models::{ApiError, AuthLogoutResponse},
    sha256_hex,
    utils::now_unix,
    AgentState,
};
use super::super::super::core::{build_clear_session_cookie, session_token_from_headers};
use crate::storage::revoke_auth_session_at_path;

pub(crate) async fn auth_logout(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<AuthLogoutResponse>), ApiError> {
    if let Some(session_token) =
        session_token_from_headers(&headers, &state.config.auth_session_cookie_name)
    {
        revoke_auth_session_at_path(
            state.db_path.as_ref(),
            &sha256_hex(session_token.as_bytes()),
            now_unix(),
        )?;
    }

    let mut response_headers = HeaderMap::new();
    response_headers.insert(SET_COOKIE, build_clear_session_cookie(&state.config)?);

    Ok((
        response_headers,
        Json(AuthLogoutResponse { logged_out: true }),
    ))
}
