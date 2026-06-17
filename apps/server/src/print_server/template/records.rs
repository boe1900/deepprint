use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    Json,
};

use super::super::{AgentState, ApiError, ApiResult};
use super::validation::{map_template_write_error, normalize_template_payload};
use super::{
    workspace::to_template_response_item, CreateTemplateRequest, TemplateDeleteResponse,
    TemplateResponse, UpdateTemplateRequest,
};
use crate::storage::{
    delete_template_record_at_path, insert_template_record_at_path, update_template_record_at_path,
};

pub(super) async fn create_template(
    State(state): State<Arc<AgentState>>,
    Json(payload): Json<CreateTemplateRequest>,
) -> ApiResult<(StatusCode, Json<TemplateResponse>)> {
    let normalized = normalize_template_payload(
        &payload.group_id,
        &payload.name,
        &payload.description,
        &payload.output_name,
        &payload.typst_code,
        &payload.sample_data,
    )?;
    let template = insert_template_record_at_path(state.db_path.as_ref(), normalized)
        .map_err(map_template_write_error)?;
    Ok((
        StatusCode::CREATED,
        Json(TemplateResponse {
            template: to_template_response_item(template),
        }),
    ))
}

pub(super) async fn update_template(
    State(state): State<Arc<AgentState>>,
    AxumPath(template_id): AxumPath<String>,
    Json(payload): Json<UpdateTemplateRequest>,
) -> ApiResult<Json<TemplateResponse>> {
    let normalized = normalize_template_payload(
        &payload.group_id,
        &payload.name,
        &payload.description,
        &payload.output_name,
        &payload.typst_code,
        &payload.sample_data,
    )?;
    let template = update_template_record_at_path(state.db_path.as_ref(), &template_id, normalized)
        .map_err(map_template_write_error)?
        .ok_or_else(|| ApiError::NotFound(format!("template not found: {template_id}")))?;
    Ok(Json(TemplateResponse {
        template: to_template_response_item(template),
    }))
}

pub(super) async fn delete_template(
    State(state): State<Arc<AgentState>>,
    AxumPath(template_id): AxumPath<String>,
) -> ApiResult<Json<TemplateDeleteResponse>> {
    let deleted = delete_template_record_at_path(state.db_path.as_ref(), &template_id)?;
    if !deleted {
        return Err(ApiError::NotFound(format!(
            "template not found: {template_id}"
        )));
    }

    Ok(Json(TemplateDeleteResponse {
        template_id,
        deleted,
    }))
}
