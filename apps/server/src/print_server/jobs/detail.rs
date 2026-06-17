use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};
use serde_json::Value;

use super::super::{AgentState, ApiError, ApiResult, JobRecord};
use super::JobResponse;
use crate::storage::fetch_job_by_id_at_path;

pub(super) async fn get_job(
    State(state): State<Arc<AgentState>>,
    AxumPath(job_id): AxumPath<String>,
) -> ApiResult<Json<JobResponse>> {
    let job = fetch_job_by_id_at_path(state.db_path.as_ref(), &job_id)?
        .ok_or_else(|| ApiError::NotFound(format!("job not found: {job_id}")))?;

    Ok(Json(to_job_response(job)))
}

pub(super) fn to_job_response(job: JobRecord) -> JobResponse {
    let data = serde_json::from_str::<Value>(&job.data_json).unwrap_or(Value::Null);
    let print_options = serde_json::from_str::<Value>(&job.print_options_json)
        .unwrap_or_else(|_| serde_json::json!({}));

    JobResponse {
        job_id: job.id,
        request_id: job.request_id,
        job_kind: job.job_kind,
        printer_id: job.printer_id,
        printer_name_snapshot: job.printer_name_snapshot,
        printer_uri: job.printer_uri,
        status: job.status,
        attempt_count: job.attempt_count,
        created_at: job.created_at,
        updated_at: job.updated_at,
        last_error_code: job.last_error_code,
        last_error_message: job.last_error_message,
        render_artifact_path: job.render_artifact_path,
        render_output_kind: job.render_output_kind,
        render_page_count: job.render_page_count,
        render_page_width_pt: job.render_page_width_pt,
        render_page_height_pt: job.render_page_height_pt,
        backend_name: job.backend_name,
        backend_job_ref_json: job.backend_job_ref_json,
        submit_started_at: job.submit_started_at,
        submitted_at: job.submitted_at,
        last_polled_at: job.last_polled_at,
        backend_state: job.backend_state,
        backend_state_message: job.backend_state_message,
        unknown_since_at: job.unknown_since_at,
        needs_attention_reason: job.needs_attention_reason,
        source_file_name: job.source_file_name,
        source_content_type: job.source_content_type,
        source_file_size_bytes: job.source_file_size_bytes,
        data,
        print_options,
    }
}
