use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};

use super::super::{
    models::{
        ApiError, ApiResult, JOB_STATUS_CANCELED, JOB_STATUS_NEEDS_ATTENTION, JOB_STATUS_PRINTING,
        JOB_STATUS_QUEUED,
    },
    AgentState,
};
use super::CancelResponse;
use crate::storage::{
    cancel_needs_attention_job, cancel_printing_job, cancel_queued_job, fetch_job_by_id_at_path,
};

pub(super) async fn cancel_job(
    State(state): State<Arc<AgentState>>,
    AxumPath(job_id): AxumPath<String>,
) -> ApiResult<Json<CancelResponse>> {
    let Some(job) = fetch_job_by_id_at_path(state.db_path.as_ref(), &job_id)? else {
        return Err(ApiError::NotFound(format!("job not found: {job_id}")));
    };

    match job.status.as_str() {
        JOB_STATUS_QUEUED => {
            if !cancel_queued_job(state.db_path.as_ref(), &job_id)? {
                return Err(ApiError::Conflict("job state changed, retry".to_string()));
            }

            Ok(Json(CancelResponse {
                job_id,
                status: JOB_STATUS_CANCELED.to_string(),
            }))
        }
        JOB_STATUS_PRINTING => {
            let backend_job_ref_json = job.backend_job_ref_json.clone().ok_or_else(|| {
                ApiError::Conflict(
                    "job is printing but backend_job_ref_json is missing".to_string(),
                )
            })?;

            let backend = state.backend.clone();
            tokio::task::spawn_blocking(move || backend.cancel_job(&backend_job_ref_json))
                .await
                .map_err(|err| ApiError::Internal(format!("cancel_job join error: {err}")))?
                .map_err(|err| {
                    ApiError::ServiceUnavailable(format!(
                        "cancel job failed [{}]: {}",
                        err.code(),
                        err.message()
                    ))
                })?;

            if !cancel_printing_job(state.db_path.as_ref(), &job_id)? {
                return Err(ApiError::Conflict(
                    "job state changed while canceling, retry query".to_string(),
                ));
            }

            Ok(Json(CancelResponse {
                job_id,
                status: JOB_STATUS_CANCELED.to_string(),
            }))
        }
        JOB_STATUS_NEEDS_ATTENTION => {
            if !cancel_needs_attention_job(state.db_path.as_ref(), &job_id)? {
                return Err(ApiError::Conflict("job state changed, retry".to_string()));
            }

            Ok(Json(CancelResponse {
                job_id,
                status: JOB_STATUS_CANCELED.to_string(),
            }))
        }
        _ => Err(ApiError::Conflict(
            "only queued/printing/needs_attention jobs can be canceled".to_string(),
        )),
    }
}
