use std::sync::Arc;

use axum::{
    extract::{Multipart, Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use super::{
    auth::{api_key_scopes, require_api_key_for_path, require_api_key_scope_for_path},
    jobs::{get_job, job_response_from_record, JobResponse},
    submission::{
        create_direct_job_from_input, create_job, preview_typst, CreateJobRequest,
        CreateJobResponse, DirectJobInput, PreviewTypstRequest,
    },
    template::{build_template_workspace_response, TemplateWorkspaceResponse},
    AgentState, ApiError, ApiResult, PrintOptions, PrinterDetail, PrinterSummary,
    PrintersListResponse, API_SCOPE_JOB_READ, API_SCOPE_PREVIEW_CREATE, API_SCOPE_PRINTER_READ,
    API_SCOPE_PRINT_CREATE, API_SCOPE_TEMPLATE_READ,
};
use crate::storage::{
    fetch_job_by_request_id_at_path, fetch_template_by_id_at_path, list_printer_summaries,
    load_printer_detail_by_id, ApiKeyRecord,
};

#[derive(Debug, Deserialize)]
pub(super) struct OpenPreviewRequest {
    pub(super) template_id: String,
    pub(super) data: Value,
    #[serde(default)]
    pub(super) print_options: PrintOptions,
}

#[derive(Debug, Deserialize)]
pub(super) struct OpenPrintRequest {
    pub(super) request_id: String,
    pub(super) template_id: String,
    pub(super) printer_id: String,
    pub(super) data: Value,
    #[serde(default)]
    pub(super) print_options: PrintOptions,
}

#[derive(Debug, serde::Serialize)]
pub(super) struct OpenMeResponse {
    pub(super) api_key: OpenApiKeyResponse,
}

#[derive(Debug, serde::Serialize)]
pub(super) struct OpenApiKeyResponse {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) key_prefix: String,
    pub(super) scopes: Vec<String>,
    pub(super) status: String,
    pub(super) created_at: i64,
    pub(super) updated_at: i64,
    pub(super) last_used_at: Option<i64>,
    pub(super) expires_at: Option<i64>,
}

pub(super) async fn open_me(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
) -> ApiResult<Json<OpenMeResponse>> {
    let api_key = require_api_key_for_path(state.db_path.as_ref(), &headers)?;
    Ok(Json(OpenMeResponse {
        api_key: open_api_key_response(api_key)?,
    }))
}

pub(super) async fn open_list_templates(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
) -> ApiResult<Json<TemplateWorkspaceResponse>> {
    require_api_key_scope_for_path(state.db_path.as_ref(), &headers, API_SCOPE_TEMPLATE_READ)?;
    Ok(Json(build_template_workspace_response(
        state.db_path.as_ref(),
    )?))
}

pub(super) async fn open_list_printers(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
) -> ApiResult<Json<PrintersListResponse<PrinterSummary>>> {
    require_api_key_scope_for_path(state.db_path.as_ref(), &headers, API_SCOPE_PRINTER_READ)?;
    let printers = list_printer_summaries(state.db_path.as_ref())
        .map_err(super::submission::map_print_backend_error)?;
    Ok(Json(PrintersListResponse { printers }))
}

pub(super) async fn open_get_printer_detail(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    AxumPath(printer_id): AxumPath<String>,
) -> ApiResult<Json<PrinterDetail>> {
    require_api_key_scope_for_path(state.db_path.as_ref(), &headers, API_SCOPE_PRINTER_READ)?;
    let printer = load_printer_detail_by_id(state.db_path.as_ref(), &printer_id)
        .map_err(super::submission::map_print_backend_error)?
        .ok_or_else(|| ApiError::NotFound(format!("printer not found: {printer_id}")))?;

    Ok(Json(printer))
}

pub(super) async fn open_preview_typst(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    Json(payload): Json<OpenPreviewRequest>,
) -> ApiResult<Response> {
    require_api_key_scope_for_path(state.db_path.as_ref(), &headers, API_SCOPE_PREVIEW_CREATE)?;
    let template = fetch_template_by_id_at_path(state.db_path.as_ref(), &payload.template_id)?
        .ok_or_else(|| {
            ApiError::NotFound(format!("template not found: {}", payload.template_id))
        })?;
    preview_typst(
        State(state),
        Json(PreviewTypstRequest {
            template_content: template.typst_code,
            data: payload.data,
            print_options: payload.print_options,
        }),
    )
    .await
}

pub(super) async fn open_create_job(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    Json(payload): Json<OpenPrintRequest>,
) -> ApiResult<(StatusCode, Json<CreateJobResponse>)> {
    require_api_key_scope_for_path(state.db_path.as_ref(), &headers, API_SCOPE_PRINT_CREATE)?;
    let template = fetch_template_by_id_at_path(state.db_path.as_ref(), &payload.template_id)?
        .ok_or_else(|| {
            ApiError::NotFound(format!("template not found: {}", payload.template_id))
        })?;
    create_job(
        State(state),
        Json(CreateJobRequest {
            request_id: payload.request_id,
            printer_id: payload.printer_id,
            template_content: template.typst_code,
            data: payload.data,
            print_options: payload.print_options,
        }),
    )
    .await
}

pub(super) async fn open_create_direct_job(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    multipart: Multipart,
) -> ApiResult<(StatusCode, Json<CreateJobResponse>)> {
    require_api_key_scope_for_path(state.db_path.as_ref(), &headers, API_SCOPE_PRINT_CREATE)?;
    let input = parse_open_direct_multipart(multipart).await?;
    create_direct_job_from_input(state, input).await
}

pub(super) async fn open_get_job(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    AxumPath(job_id): AxumPath<String>,
) -> ApiResult<Json<JobResponse>> {
    require_api_key_scope_for_path(state.db_path.as_ref(), &headers, API_SCOPE_JOB_READ)?;
    get_job(State(state), AxumPath(job_id)).await
}

pub(super) async fn open_get_job_by_request_id(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    AxumPath(request_id): AxumPath<String>,
) -> ApiResult<Json<JobResponse>> {
    require_api_key_scope_for_path(state.db_path.as_ref(), &headers, API_SCOPE_JOB_READ)?;
    let job = fetch_job_by_request_id_at_path(state.db_path.as_ref(), &request_id)?
        .ok_or_else(|| ApiError::NotFound(format!("job not found: {request_id}")))?;

    Ok(Json(job_response_from_record(job)))
}

fn open_api_key_response(api_key: ApiKeyRecord) -> ApiResult<OpenApiKeyResponse> {
    let scopes = api_key_scopes(&api_key)?;
    Ok(OpenApiKeyResponse {
        id: api_key.id,
        name: api_key.name,
        key_prefix: api_key.key_prefix,
        scopes,
        status: api_key.status,
        created_at: api_key.created_at,
        updated_at: api_key.updated_at,
        last_used_at: api_key.last_used_at,
        expires_at: api_key.expires_at,
    })
}

async fn parse_open_direct_multipart(mut multipart: Multipart) -> ApiResult<DirectJobInput> {
    let mut request_id: Option<String> = None;
    let mut printer_id: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut content_type: Option<String> = None;
    let mut print_options = PrintOptions::default();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| ApiError::BadRequest(format!("invalid multipart payload: {err}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "request_id" => {
                request_id = Some(read_multipart_text(field, "request_id").await?);
            }
            "printer_id" => {
                printer_id = Some(read_multipart_text(field, "printer_id").await?);
            }
            "content_type" => {
                let value = read_multipart_text(field, "content_type").await?;
                if !value.trim().is_empty() {
                    content_type = Some(value.trim().to_string());
                }
            }
            "print_options" => {
                let value = read_multipart_text(field, "print_options").await?;
                if !value.trim().is_empty() {
                    print_options =
                        serde_json::from_str::<PrintOptions>(&value).map_err(|err| {
                            ApiError::BadRequest(format!(
                                "print_options must be a valid JSON object: {err}"
                            ))
                        })?;
                }
            }
            "file" => {
                let detected_name = field
                    .file_name()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string);
                let detected_content_type = field
                    .content_type()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string);
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|err| ApiError::BadRequest(format!("unable to read file: {err}")))?
                    .to_vec();
                file_name = detected_name.or(file_name);
                content_type = content_type.or(detected_content_type);
                file_bytes = Some(bytes);
            }
            _ => {}
        }
    }

    Ok(DirectJobInput {
        request_id: request_id.ok_or_else(|| {
            ApiError::BadRequest("multipart field request_id is required".to_string())
        })?,
        printer_id: printer_id.ok_or_else(|| {
            ApiError::BadRequest("multipart field printer_id is required".to_string())
        })?,
        file_name: file_name
            .ok_or_else(|| ApiError::BadRequest("multipart file field is required".to_string()))?,
        file_bytes: file_bytes
            .ok_or_else(|| ApiError::BadRequest("multipart file field is required".to_string()))?,
        content_type,
        print_options,
    })
}

async fn read_multipart_text(
    field: axum::extract::multipart::Field<'_>,
    label: &str,
) -> ApiResult<String> {
    field
        .text()
        .await
        .map_err(|err| ApiError::BadRequest(format!("unable to read {label}: {err}")))
}
