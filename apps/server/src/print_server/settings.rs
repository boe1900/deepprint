use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use super::{
    submission::map_print_backend_error, utils::now_unix, AgentState, ApiError, ApiResult,
};
use crate::{print_backend::discover::probe_cups_connection, storage::save_cups_base_url};

#[derive(Debug, Serialize)]
pub(crate) struct CupsSettingsResponse {
    pub(crate) cups_base_url: String,
    pub(crate) source: &'static str,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateCupsSettingsRequest {
    pub(crate) cups_base_url: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CupsConnectionTestResponse {
    pub(crate) ok: bool,
    pub(crate) cups_base_url: String,
    pub(crate) message: String,
}

pub(crate) async fn get_cups_settings(
    State(state): State<Arc<AgentState>>,
) -> ApiResult<Json<CupsSettingsResponse>> {
    let current = state.current_cups_base_url();
    Ok(Json(CupsSettingsResponse {
        cups_base_url: current,
        source: "runtime",
    }))
}

pub(crate) async fn update_cups_settings(
    State(state): State<Arc<AgentState>>,
    Json(payload): Json<UpdateCupsSettingsRequest>,
) -> ApiResult<Json<CupsSettingsResponse>> {
    let normalized = normalize_cups_base_url(&payload.cups_base_url)?;
    save_cups_base_url(state.db_path.as_ref(), &normalized, now_unix()).map_err(ApiError::from)?;
    state.set_cups_base_url(normalized.clone());
    Ok(Json(CupsSettingsResponse {
        cups_base_url: normalized,
        source: "runtime",
    }))
}

pub(crate) async fn test_cups_connection(
    State(state): State<Arc<AgentState>>,
    payload: Option<Json<UpdateCupsSettingsRequest>>,
) -> ApiResult<Json<CupsConnectionTestResponse>> {
    let requested = payload
        .map(|body| body.0.cups_base_url)
        .unwrap_or_else(|| state.current_cups_base_url());
    let normalized = normalize_cups_base_url(&requested)?;
    probe_cups_connection(&normalized)
        .await
        .map_err(map_print_backend_error)?;
    Ok(Json(CupsConnectionTestResponse {
        ok: true,
        cups_base_url: normalized,
        message: "CUPS service is reachable".to_string(),
    }))
}

fn normalize_cups_base_url(raw: &str) -> ApiResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_CUPS_BASE_URL",
            "CUPS URL is required",
        ));
    }

    let parsed = url::Url::parse(trimmed).map_err(|err| {
        ApiError::bad_request(
            "INVALID_CUPS_BASE_URL",
            format!("CUPS URL is invalid: {err}"),
        )
    })?;

    match parsed.scheme() {
        "http" | "https" | "ipp" | "ipps" => {}
        scheme => {
            return Err(ApiError::bad_request(
                "INVALID_CUPS_BASE_URL",
                format!("unsupported CUPS URL scheme: {scheme}"),
            ));
        }
    }

    if parsed.host_str().unwrap_or_default().trim().is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_CUPS_BASE_URL",
            "CUPS URL must include a host name or IP",
        ));
    }

    Ok(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::normalize_cups_base_url;

    #[test]
    fn normalize_cups_base_url_keeps_cups_root_slash() {
        assert_eq!(
            normalize_cups_base_url("http://cups:631/").unwrap(),
            "http://cups:631/"
        );
        assert_eq!(
            normalize_cups_base_url("http://127.0.0.1:631").unwrap(),
            "http://127.0.0.1:631/"
        );
        assert_eq!(
            normalize_cups_base_url("http://cups:8631/admin").unwrap(),
            "http://cups:8631/admin"
        );
    }
}
