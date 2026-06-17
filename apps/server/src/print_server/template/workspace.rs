use std::sync::Arc;

use axum::{extract::State, Json};

use super::super::{AgentState, ApiResult, TemplateGroupRecord, TemplateRecordRow};
use super::{TemplateGroupResponseItem, TemplateResponseItem, TemplateWorkspaceResponse};
use crate::storage::{
    list_template_groups_at_path, list_templates_at_path, list_templates_by_group_at_path,
};

pub(super) async fn get_template_workspace(
    State(state): State<Arc<AgentState>>,
) -> ApiResult<Json<TemplateWorkspaceResponse>> {
    Ok(Json(build_template_workspace_response(
        state.db_path.as_ref(),
    )?))
}

pub(super) fn build_template_workspace_response(
    db_path: &std::path::Path,
) -> rusqlite::Result<TemplateWorkspaceResponse> {
    let groups = list_template_groups_at_path(db_path)?;
    let templates = list_templates_at_path(db_path)?;
    let group_items = groups
        .into_iter()
        .map(|group| {
            let group_templates = templates
                .iter()
                .filter(|template| template.group_id == group.id)
                .cloned()
                .collect::<Vec<_>>();
            Ok(TemplateGroupResponseItem {
                id: group.id,
                name: group.name,
                sort_order: group.sort_order,
                created_at: group.created_at,
                updated_at: group.updated_at,
                templates: group_templates
                    .into_iter()
                    .map(to_template_response_item)
                    .collect(),
            })
        })
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(TemplateWorkspaceResponse {
        groups: group_items,
    })
}

pub(super) fn build_template_group_response_item(
    db_path: &std::path::Path,
    group: &TemplateGroupRecord,
) -> rusqlite::Result<TemplateGroupResponseItem> {
    let templates = list_templates_by_group_at_path(db_path, &group.id)?
        .into_iter()
        .map(to_template_response_item)
        .collect::<Vec<_>>();

    Ok(TemplateGroupResponseItem {
        id: group.id.clone(),
        name: group.name.clone(),
        sort_order: group.sort_order,
        created_at: group.created_at,
        updated_at: group.updated_at,
        templates,
    })
}

pub(super) fn to_template_response_item(template: TemplateRecordRow) -> TemplateResponseItem {
    TemplateResponseItem {
        id: template.id,
        group_id: template.group_id,
        name: template.name,
        description: template.description,
        output_name: template.output_name,
        typst_code: template.typst_code,
        sample_data: template.sample_data,
        sort_order: template.sort_order,
        created_at: template.created_at,
        updated_at: template.updated_at,
    }
}
