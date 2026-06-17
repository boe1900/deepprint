use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct CreateTemplateGroupRequest {
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateTemplateGroupRequest {
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateTemplateRequest {
    pub(crate) group_id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) output_name: String,
    #[serde(default)]
    pub(crate) typst_code: String,
    #[serde(default)]
    pub(crate) sample_data: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateTemplateRequest {
    pub(crate) group_id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) output_name: String,
    #[serde(default)]
    pub(crate) typst_code: String,
    #[serde(default)]
    pub(crate) sample_data: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct TemplateWorkspaceResponse {
    pub(crate) groups: Vec<TemplateGroupResponseItem>,
}

#[derive(Debug, Serialize)]
pub(crate) struct TemplateGroupResponse {
    pub(crate) group: TemplateGroupResponseItem,
}

#[derive(Debug, Serialize)]
pub(crate) struct TemplateResponse {
    pub(crate) template: TemplateResponseItem,
}

#[derive(Debug, Serialize)]
pub(crate) struct TemplateDeleteResponse {
    pub(crate) template_id: String,
    pub(crate) deleted: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct TemplateGroupDeleteResponse {
    pub(crate) group_id: String,
    pub(crate) deleted: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct TemplateGroupResponseItem {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) sort_order: i64,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
    pub(crate) templates: Vec<TemplateResponseItem>,
}

#[derive(Debug, Serialize)]
pub(crate) struct TemplateResponseItem {
    pub(crate) id: String,
    pub(crate) group_id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) output_name: String,
    pub(crate) typst_code: String,
    pub(crate) sample_data: String,
    pub(crate) sort_order: i64,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
}
