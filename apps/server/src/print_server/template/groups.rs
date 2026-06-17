use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    Json,
};

use super::super::{AgentState, ApiError, ApiResult};
use super::validation::{map_template_write_error, validate_template_group_name};
use super::{
    workspace::build_template_group_response_item, CreateTemplateGroupRequest,
    TemplateGroupDeleteResponse, TemplateGroupResponse, UpdateTemplateGroupRequest,
};
use crate::storage::{
    count_templates_in_group_at_path, delete_template_group_by_id_at_path,
    insert_template_group_at_path, update_template_group_record_at_path,
};

pub(super) async fn create_template_group(
    State(state): State<Arc<AgentState>>,
    Json(payload): Json<CreateTemplateGroupRequest>,
) -> ApiResult<(StatusCode, Json<TemplateGroupResponse>)> {
    validate_template_group_name(&payload.name)?;
    let group = insert_template_group_at_path(state.db_path.as_ref(), &payload.name)
        .map_err(map_template_write_error)?;
    Ok((
        StatusCode::CREATED,
        Json(TemplateGroupResponse {
            group: build_template_group_response_item(state.db_path.as_ref(), &group)?,
        }),
    ))
}

pub(super) async fn update_template_group(
    State(state): State<Arc<AgentState>>,
    AxumPath(group_id): AxumPath<String>,
    Json(payload): Json<UpdateTemplateGroupRequest>,
) -> ApiResult<Json<TemplateGroupResponse>> {
    validate_template_group_name(&payload.name)?;
    let group =
        update_template_group_record_at_path(state.db_path.as_ref(), &group_id, &payload.name)
            .map_err(map_template_write_error)?
            .ok_or_else(|| ApiError::NotFound(format!("template group not found: {group_id}")))?;
    Ok(Json(TemplateGroupResponse {
        group: build_template_group_response_item(state.db_path.as_ref(), &group)?,
    }))
}

pub(super) async fn delete_template_group(
    State(state): State<Arc<AgentState>>,
    AxumPath(group_id): AxumPath<String>,
) -> ApiResult<Json<TemplateGroupDeleteResponse>> {
    let deleted = delete_template_group_record(state.db_path.as_ref(), &group_id)?;
    if !deleted {
        return Err(ApiError::NotFound(format!(
            "template group not found: {group_id}"
        )));
    }

    Ok(Json(TemplateGroupDeleteResponse { group_id, deleted }))
}

pub(super) fn delete_template_group_record(
    db_path: &std::path::Path,
    group_id: &str,
) -> Result<bool, ApiError> {
    let template_count = count_templates_in_group_at_path(db_path, group_id)?;
    if template_count > 0 {
        return Err(ApiError::Conflict(
            "该分组下仍有模板，请先删除或移动模板".to_string(),
        ));
    }

    delete_template_group_by_id_at_path(db_path, group_id).map_err(ApiError::Db)
}
