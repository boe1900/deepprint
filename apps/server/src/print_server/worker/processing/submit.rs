use std::path::PathBuf;

use super::super::super::{models::ProcessJobError, AgentState, JobRecord};
use crate::printer::{PrintOptions, SubmitJobRequest};
use crate::storage::save_backend_submission;

pub(super) async fn submit_rendered_job(
    state: &AgentState,
    job: &JobRecord,
    job_id: &str,
    artifact_path: &str,
    print_options: PrintOptions,
) -> Result<(), ProcessJobError> {
    let backend = state.backend.clone();
    let submit_req = SubmitJobRequest {
        local_file: PathBuf::from(artifact_path),
        printer_uri: job.printer_uri.clone().ok_or_else(|| {
            ProcessJobError::new(
                "PRINTER_URI_MISSING",
                "job printer_uri snapshot is missing before submission",
            )
        })?,
        job_name: format!("deepprint:{job_id}"),
        document_format: if artifact_path.to_ascii_lowercase().ends_with(".pdf") {
            Some("application/pdf".to_string())
        } else {
            job.source_content_type.clone()
        },
        options: print_options,
    };

    let submit_result = tokio::task::spawn_blocking(move || backend.submit_job(&submit_req))
        .await
        .map_err(|err| ProcessJobError::new("BACKEND_JOIN_ERROR", err.to_string()))?
        .map_err(|err| {
            if err.retryable() {
                ProcessJobError::retryable(err.code(), err.message())
            } else {
                ProcessJobError::new(err.code(), err.message())
            }
        })?;

    let saved = save_backend_submission(
        state.db_path.as_ref(),
        job_id,
        &submit_result.backend,
        submit_result.backend_job_ref_json.as_deref(),
    )
    .map_err(|err| {
        ProcessJobError::new(
            "DB_WRITE_FAILED",
            format!("failed to save backend metadata: {err}"),
        )
    })?;

    if !saved {
        return Err(ProcessJobError::new(
            "STATE_STALE",
            "job state changed before backend submission persisted",
        ));
    }

    Ok(())
}
