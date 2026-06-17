use axum::http::{header::COOKIE, HeaderMap, HeaderValue};
use uuid::Uuid;

use super::super::super::{config::AgentConfig, models::ApiError};

pub(crate) fn generate_session_token() -> String {
    format!("dps_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub(crate) fn build_session_cookie(
    config: &AgentConfig,
    session_token: &str,
    ttl_sec: u64,
) -> Result<HeaderValue, ApiError> {
    let secure = if config.auth_cookie_secure {
        "; Secure"
    } else {
        ""
    };
    HeaderValue::from_str(&format!(
        "{}={}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax{}",
        config.auth_session_cookie_name, session_token, ttl_sec, secure
    ))
    .map_err(|err| ApiError::Internal(format!("build session cookie failed: {err}")))
}

pub(crate) fn build_clear_session_cookie(config: &AgentConfig) -> Result<HeaderValue, ApiError> {
    let secure = if config.auth_cookie_secure {
        "; Secure"
    } else {
        ""
    };
    HeaderValue::from_str(&format!(
        "{}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax{}",
        config.auth_session_cookie_name, secure
    ))
    .map_err(|err| ApiError::Internal(format!("build clear session cookie failed: {err}")))
}

pub(crate) fn session_token_from_headers(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    let cookie_header = headers.get(COOKIE)?.to_str().ok()?;
    for cookie in cookie_header.split(';') {
        let Some((name, value)) = cookie.trim().split_once('=') else {
            continue;
        };
        if name.trim() == cookie_name {
            let token = value.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }
    None
}

pub(crate) fn header_string(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}
