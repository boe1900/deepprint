use tracing::warn;

use super::super::super::super::{AgentState, BackendJobState};

pub(super) async fn query_backend_job_status(
    state: &AgentState,
    job_id: &str,
    backend_job_ref_json: &str,
) -> Result<BackendJobState, PrintingQueryError> {
    let backend = state.backend.clone();
    let backend_job_ref_json = backend_job_ref_json.to_string();
    let query_result =
        tokio::task::spawn_blocking(move || backend.query_job_status(&backend_job_ref_json)).await;

    match query_result {
        Ok(Ok(status)) => Ok(status),
        Ok(Err(err)) if err.retryable() => {
            Err(PrintingQueryError::Retryable(err.message().to_string()))
        }
        Ok(Err(err)) => Err(PrintingQueryError::NonRetryable {
            code: err.code().to_string(),
            message: err.message().to_string(),
        }),
        Err(err) => {
            let message = err.to_string();
            warn!("backend query join failure for job {job_id}: {message}");
            Err(PrintingQueryError::Join(message))
        }
    }
}

pub(super) enum PrintingQueryError {
    Retryable(String),
    NonRetryable { code: String, message: String },
    Join(String),
}
