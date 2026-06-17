use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use super::super::super::{
    models::{
        ApiError, CreateApiKeyRequest, CreateApiKeyResponse, ListApiKeysResponse,
        RevokeApiKeyResponse,
    },
    utils::now_unix,
    AgentState,
};
use super::super::sessions::require_admin_session_for_path;
use super::{records::create_api_key_record_at_path, responses::api_key_response_item};
use crate::storage::{list_api_key_records_at_path, revoke_api_key_record_at_path};

pub(crate) async fn list_api_keys(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<ListApiKeysResponse>, ApiError> {
    require_admin_session_for_path(state.db_path.as_ref(), &state.config, &headers)?;
    let api_keys = list_api_key_records_at_path(state.db_path.as_ref())?
        .into_iter()
        .map(api_key_response_item)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(ListApiKeysResponse { api_keys }))
}

pub(crate) async fn create_api_key(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    Json(payload): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), ApiError> {
    let admin_session =
        require_admin_session_for_path(state.db_path.as_ref(), &state.config, &headers)?;
    let (api_key, token) =
        create_api_key_record_at_path(state.db_path.as_ref(), &payload, &admin_session.user.id)?;
    Ok((
        StatusCode::CREATED,
        Json(CreateApiKeyResponse {
            api_key: api_key_response_item(api_key)?,
            token,
        }),
    ))
}

pub(crate) async fn revoke_api_key(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    AxumPath(api_key_id): AxumPath<String>,
) -> Result<Json<RevokeApiKeyResponse>, ApiError> {
    require_admin_session_for_path(state.db_path.as_ref(), &state.config, &headers)?;
    let api_key = revoke_api_key_record_at_path(state.db_path.as_ref(), &api_key_id, now_unix())?
        .ok_or_else(|| ApiError::NotFound(format!("api key not found: {api_key_id}")))?;
    Ok(Json(RevokeApiKeyResponse {
        api_key: api_key_response_item(api_key)?,
    }))
}
