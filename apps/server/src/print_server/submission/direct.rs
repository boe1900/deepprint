use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use uuid::Uuid;

use super::{is_unique_request_id_violation, load_job_printer_target, CreateDirectJobRequest};
use super::{map_print_backend_error, CreateJobResponse};
use crate::{
    print_backend::validate_print_options_against_capabilities,
    print_server::{rendering, utils, AgentState, ApiError, ApiResult},
    printer::PrintOptions,
    storage::{fetch_job_by_request_id_at_path, insert_direct_job_at_path, DirectJobInsertInput},
};

pub(crate) struct DirectJobInput {
    pub(crate) request_id: String,
    pub(crate) printer_id: String,
    pub(crate) file_name: String,
    pub(crate) file_bytes: Vec<u8>,
    pub(crate) content_type: Option<String>,
    pub(crate) print_options: PrintOptions,
}

pub(super) async fn create_direct_job(
    State(state): State<Arc<AgentState>>,
    Json(payload): Json<CreateDirectJobRequest>,
) -> ApiResult<(StatusCode, Json<CreateJobResponse>)> {
    utils::validate_create_direct_job_payload(&payload)?;

    let file_bytes = utils::decode_base64_payload(&payload.file_content_base64)?;
    create_direct_job_from_input(
        state,
        DirectJobInput {
            request_id: payload.request_id,
            printer_id: payload.printer_id,
            file_name: payload.file_name,
            file_bytes,
            content_type: payload.content_type,
            print_options: payload.print_options,
        },
    )
    .await
}

pub(super) async fn create_direct_job_from_input(
    state: Arc<AgentState>,
    payload: DirectJobInput,
) -> ApiResult<(StatusCode, Json<CreateJobResponse>)> {
    validate_direct_job_input(&payload)?;

    let file_bytes = payload.file_bytes;
    if file_bytes.is_empty() {
        return Err(ApiError::BadRequest("file must not be empty".to_string()));
    }

    if (file_bytes.len() as u64) > state.config.direct_job_max_bytes {
        return Err(ApiError::BadRequest(format!(
            "file too large: {} bytes exceeds limit {} bytes",
            file_bytes.len(),
            state.config.direct_job_max_bytes
        )));
    }

    let printer_target = load_job_printer_target(state.db_path.as_ref(), &payload.printer_id)?;
    validate_print_options_against_capabilities(
        &printer_target.capabilities,
        &payload.print_options,
    )
    .map_err(map_print_backend_error)?;

    if let Some(existing) =
        fetch_job_by_request_id_at_path(state.db_path.as_ref(), &payload.request_id)?
    {
        return Ok((
            StatusCode::OK,
            Json(CreateJobResponse {
                job_id: existing.id,
                status: existing.status,
                idempotent: true,
            }),
        ));
    }

    let job_id = Uuid::new_v4().to_string();
    let now = utils::now_unix();
    let source_file_name = rendering::sanitize_source_file_name(&payload.file_name);
    let source_path = rendering::stage_direct_job_source(&job_id, &source_file_name, &file_bytes)?;
    let source_path_text = source_path.to_string_lossy().to_string();
    let source_size = utils::clamp_u64_to_i64(file_bytes.len() as u64);
    let content_type = payload
        .content_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let data_json = serde_json::to_string(&serde_json::json!({}))
        .map_err(|_| ApiError::bad_request("DATA_INVALID_JSON", "data must be valid JSON"))?;
    let print_options_json = serde_json::to_string(&payload.print_options).map_err(|_| {
        ApiError::bad_request(
            "PRINT_OPTIONS_INVALID_JSON",
            "print_options must be valid JSON",
        )
    })?;

    let insert_result = insert_direct_job_at_path(
        state.db_path.as_ref(),
        DirectJobInsertInput {
            id: &job_id,
            request_id: &payload.request_id,
            printer_id: &printer_target.printer_id,
            printer_name_snapshot: &printer_target.printer_name_snapshot,
            printer_uri: &printer_target.printer_uri,
            data_json: &data_json,
            print_options_json: &print_options_json,
            source_file_path: &source_path_text,
            source_file_name: &source_file_name,
            source_content_type: content_type.as_deref(),
            source_file_size_bytes: source_size,
            created_at: now,
        },
    );

    match insert_result {
        Ok(_) => Ok((
            StatusCode::ACCEPTED,
            Json(CreateJobResponse {
                job_id,
                status: "queued".to_string(),
                idempotent: false,
            }),
        )),
        Err(err) if is_unique_request_id_violation(&err) => {
            rendering::cleanup_direct_job_source(&source_path);
            if let Some(existing) =
                fetch_job_by_request_id_at_path(state.db_path.as_ref(), &payload.request_id)?
            {
                Ok((
                    StatusCode::OK,
                    Json(CreateJobResponse {
                        job_id: existing.id,
                        status: existing.status,
                        idempotent: true,
                    }),
                ))
            } else {
                Err(ApiError::Db(err))
            }
        }
        Err(err) => {
            rendering::cleanup_direct_job_source(&source_path);
            Err(ApiError::Db(err))
        }
    }
}

fn validate_direct_job_input(payload: &DirectJobInput) -> ApiResult<()> {
    utils::validate_request_id_text(&payload.request_id)?;
    utils::validate_printer_id_text(&payload.printer_id)?;
    if payload.file_name.trim().is_empty() {
        return Err(ApiError::BadRequest("file_name is required".to_string()));
    }
    if payload.file_name.len() > 255 {
        return Err(ApiError::BadRequest(
            "file_name length must be <= 255".to_string(),
        ));
    }
    utils::validate_print_options(&payload.print_options)
}
