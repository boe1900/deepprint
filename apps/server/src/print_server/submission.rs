use std::{path::Path, sync::Arc};

use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ApiError, ApiResult, PrintOptions};
use crate::{
    print_backend::{PrintBackendError, PrinterCapabilities},
    storage::load_printer_detail_by_id,
};

#[path = "submission/create.rs"]
mod create;
#[path = "submission/direct.rs"]
mod direct;
#[path = "submission/preview.rs"]
mod preview;

#[derive(Debug, Deserialize)]
pub(super) struct CreateJobRequest {
    pub(super) request_id: String,
    pub(super) printer_id: String,
    pub(super) template_content: String,
    pub(super) data: Value,
    #[serde(default)]
    pub(super) print_options: PrintOptions,
}

#[derive(Debug, Deserialize)]
pub(super) struct CreateDirectJobRequest {
    pub(super) request_id: String,
    pub(super) printer_id: String,
    pub(super) file_name: String,
    pub(super) file_content_base64: String,
    #[serde(default)]
    pub(super) content_type: Option<String>,
    #[serde(default)]
    pub(super) print_options: PrintOptions,
}

#[derive(Debug, Deserialize)]
pub(super) struct PreviewTypstRequest {
    pub(super) template_content: String,
    pub(super) data: Value,
    #[serde(default)]
    pub(super) print_options: PrintOptions,
}

#[derive(Debug, Serialize)]
pub(super) struct CreateJobResponse {
    pub(super) job_id: String,
    pub(super) status: String,
    pub(super) idempotent: bool,
}

#[derive(Debug, Clone)]
pub(super) struct RenderCacheKey {
    key: String,
    template_hash: String,
    data_hash: String,
    print_options_hash: String,
}

#[derive(Debug, Clone)]
struct JobPrinterTarget {
    printer_id: String,
    printer_name_snapshot: String,
    printer_uri: String,
    capabilities: PrinterCapabilities,
}

pub(super) async fn create_job(
    state: State<Arc<super::AgentState>>,
    payload: Json<CreateJobRequest>,
) -> ApiResult<(StatusCode, Json<CreateJobResponse>)> {
    create::create_job(state, payload).await
}

pub(super) async fn create_direct_job(
    state: State<Arc<super::AgentState>>,
    payload: Json<CreateDirectJobRequest>,
) -> ApiResult<(StatusCode, Json<CreateJobResponse>)> {
    direct::create_direct_job(state, payload).await
}

pub(super) async fn create_direct_job_from_input(
    state: Arc<super::AgentState>,
    payload: direct::DirectJobInput,
) -> ApiResult<(StatusCode, Json<CreateJobResponse>)> {
    direct::create_direct_job_from_input(state, payload).await
}

pub(super) use direct::DirectJobInput;

pub(super) async fn preview_typst(
    state: State<Arc<super::AgentState>>,
    payload: Json<PreviewTypstRequest>,
) -> ApiResult<Response> {
    preview::preview_typst(state, payload).await
}

pub(super) fn build_render_cache_key(job: &super::JobRecord) -> RenderCacheKey {
    preview::build_render_cache_key(job)
}

pub(super) fn to_storage_render_cache_key(
    cache_key: &RenderCacheKey,
) -> crate::storage::RenderCacheKey {
    preview::to_storage_render_cache_key(cache_key)
}

pub(super) fn map_print_backend_error(err: PrintBackendError) -> ApiError {
    match err {
        PrintBackendError::InvalidTarget(message) => {
            ApiError::bad_request("INVALID_PRINTER_TARGET", message)
        }
        PrintBackendError::Unsupported(message) => {
            ApiError::bad_request("UNSUPPORTED_PRINTER_TARGET", message)
        }
        PrintBackendError::Conflict(message) => ApiError::conflict("PRINTER_CONFLICT", message),
        PrintBackendError::PrintOptionUnsupported { .. }
        | PrintBackendError::PrintOptionInvalidForPrinter { .. }
        | PrintBackendError::PrinterCapabilityUnknown { .. } => {
            let code = err.api_code().unwrap_or("CONFLICT");
            let message = err
                .api_message()
                .unwrap_or("printer capability conflict")
                .to_string();
            let api_error = ApiError::conflict(code, message);
            if let Some(details) = err.api_details() {
                api_error.with_details(details)
            } else {
                api_error
            }
        }
        PrintBackendError::Unreachable(message) => {
            ApiError::service_unavailable("PRINTER_UNREACHABLE", message)
        }
        PrintBackendError::Backend(message) => ApiError::internal(message),
        PrintBackendError::Db(err) => ApiError::Db(err),
        PrintBackendError::Serde(err) => ApiError::internal(format!("serialization error: {err}")),
    }
}

fn load_job_printer_target(db_path: &Path, printer_id: &str) -> ApiResult<JobPrinterTarget> {
    let printer_id = printer_id.trim();
    if printer_id.is_empty() {
        return Err(ApiError::BadRequest("printer_id is required".to_string()));
    }

    let printer = load_printer_detail_by_id(db_path, printer_id)
        .map_err(map_print_backend_error)?
        .ok_or_else(|| {
            ApiError::not_found(
                "PRINTER_NOT_FOUND",
                format!("printer not found: {printer_id}"),
            )
        })?;

    if !printer.enabled {
        return Err(ApiError::conflict(
            "PRINTER_DISABLED",
            format!("printer is disabled: {}", printer.id),
        ));
    }

    Ok(JobPrinterTarget {
        printer_id: printer.id,
        printer_name_snapshot: printer.name,
        printer_uri: printer.uri,
        capabilities: printer.capabilities,
    })
}

fn insert_preview_header(headers: &mut HeaderMap, key: &'static str, value: &str) -> ApiResult<()> {
    let header_value = HeaderValue::from_str(value)
        .map_err(|err| ApiError::Internal(format!("invalid preview header {key}: {err}")))?;
    headers.insert(key, header_value);
    Ok(())
}

fn is_unique_request_id_violation(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(_, Some(message))
            if message.contains("UNIQUE constraint failed: jobs.request_id")
    )
}
