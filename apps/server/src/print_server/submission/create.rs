use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use uuid::Uuid;

use super::{is_unique_request_id_violation, load_job_printer_target, CreateJobRequest};
use super::{map_print_backend_error, CreateJobResponse};
use crate::{
    print_backend::validate_print_options_against_capabilities,
    print_server::{utils, AgentState, ApiError, ApiResult},
    storage::{
        fetch_job_by_request_id_at_path, insert_template_job_at_path, TemplateJobInsertInput,
    },
};

pub(super) async fn create_job(
    State(state): State<Arc<AgentState>>,
    Json(payload): Json<CreateJobRequest>,
) -> ApiResult<(StatusCode, Json<CreateJobResponse>)> {
    utils::validate_create_job_payload(&payload)?;

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
    let data_json = serde_json::to_string(&payload.data)
        .map_err(|_| ApiError::bad_request("DATA_INVALID_JSON", "data must be valid JSON"))?;
    let print_options_json = serde_json::to_string(&payload.print_options).map_err(|_| {
        ApiError::bad_request(
            "PRINT_OPTIONS_INVALID_JSON",
            "print_options must be valid JSON",
        )
    })?;

    let insert_result = insert_template_job_at_path(
        state.db_path.as_ref(),
        TemplateJobInsertInput {
            id: &job_id,
            request_id: &payload.request_id,
            printer_id: &printer_target.printer_id,
            printer_name_snapshot: &printer_target.printer_name_snapshot,
            printer_uri: &printer_target.printer_uri,
            template_content: &payload.template_content,
            data_json: &data_json,
            print_options_json: &print_options_json,
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
        Err(err) => Err(ApiError::Db(err)),
    }
}
