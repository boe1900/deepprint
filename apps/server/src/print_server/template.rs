use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    Json,
};

#[path = "template/groups.rs"]
mod groups;
#[path = "template/models.rs"]
mod models;
#[path = "template/records.rs"]
mod records;
#[path = "template/validation.rs"]
mod validation;
#[path = "template/workspace.rs"]
mod workspace;

use super::{AgentState, ApiResult};
#[cfg(test)]
use super::ApiError;
pub(super) use models::{
    CreateTemplateGroupRequest, CreateTemplateRequest, TemplateDeleteResponse,
    TemplateGroupDeleteResponse, TemplateGroupResponse, TemplateGroupResponseItem,
    TemplateResponse, TemplateResponseItem, TemplateWorkspaceResponse, UpdateTemplateGroupRequest,
    UpdateTemplateRequest,
};

type NormalizedTemplatePayload = crate::storage::TemplateRecordInput;

pub(super) async fn get_template_workspace(
    state: State<Arc<AgentState>>,
) -> ApiResult<Json<TemplateWorkspaceResponse>> {
    workspace::get_template_workspace(state).await
}

pub(super) async fn create_template_group(
    state: State<Arc<AgentState>>,
    payload: Json<CreateTemplateGroupRequest>,
) -> ApiResult<(StatusCode, Json<TemplateGroupResponse>)> {
    groups::create_template_group(state, payload).await
}

pub(super) async fn update_template_group(
    state: State<Arc<AgentState>>,
    group_id: AxumPath<String>,
    payload: Json<UpdateTemplateGroupRequest>,
) -> ApiResult<Json<TemplateGroupResponse>> {
    groups::update_template_group(state, group_id, payload).await
}

pub(super) async fn delete_template_group(
    state: State<Arc<AgentState>>,
    group_id: AxumPath<String>,
) -> ApiResult<Json<TemplateGroupDeleteResponse>> {
    groups::delete_template_group(state, group_id).await
}

pub(super) async fn create_template(
    state: State<Arc<AgentState>>,
    payload: Json<CreateTemplateRequest>,
) -> ApiResult<(StatusCode, Json<TemplateResponse>)> {
    records::create_template(state, payload).await
}

pub(super) async fn update_template(
    state: State<Arc<AgentState>>,
    template_id: AxumPath<String>,
    payload: Json<UpdateTemplateRequest>,
) -> ApiResult<Json<TemplateResponse>> {
    records::update_template(state, template_id, payload).await
}

pub(super) async fn delete_template(
    state: State<Arc<AgentState>>,
    template_id: AxumPath<String>,
) -> ApiResult<Json<TemplateDeleteResponse>> {
    records::delete_template(state, template_id).await
}

pub(super) fn build_template_workspace_response(
    db_path: &std::path::Path,
) -> rusqlite::Result<TemplateWorkspaceResponse> {
    workspace::build_template_workspace_response(db_path)
}

#[cfg(test)]
pub(super) fn delete_template_group_record(
    db_path: &std::path::Path,
    group_id: &str,
) -> Result<bool, ApiError> {
    groups::delete_template_group_record(db_path, group_id)
}

#[cfg(test)]
pub(super) fn normalize_template_payload(
    group_id: &str,
    name: &str,
    description: &str,
    output_name: &str,
    typst_code: &str,
    sample_data: &str,
) -> ApiResult<NormalizedTemplatePayload> {
    validation::normalize_template_payload(
        group_id,
        name,
        description,
        output_name,
        typst_code,
        sample_data,
    )
}
