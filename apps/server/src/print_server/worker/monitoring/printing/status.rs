use super::super::super::super::{
    models::{JOB_STATUS_CANCELED, JOB_STATUS_FAILED, JOB_STATUS_SUCCEEDED},
    AgentState, BackendJobState,
};
use super::super::terminal::mark_printing_job_terminal;
use super::attention::handle_unknown_status;
use crate::storage::record_backend_poll_result;

pub(super) fn record_backend_status(state: &AgentState, job_id: &str, status: BackendJobState) {
    let _ = record_backend_poll_result(
        state.db_path.as_ref(),
        job_id,
        match status {
            BackendJobState::Pending => "pending",
            BackendJobState::Processing => "processing",
            BackendJobState::Completed => "completed",
            BackendJobState::Failed => "failed",
            BackendJobState::Canceled => "canceled",
            BackendJobState::Unknown => "unknown",
        },
        None,
        !matches!(status, BackendJobState::Unknown),
    );
}

pub(super) fn apply_backend_status(
    state: &AgentState,
    job_id: &str,
    status: BackendJobState,
    unknown_since_at: Option<i64>,
    now: i64,
    job_kind: &str,
    source_file_path: Option<&str>,
) {
    match status {
        BackendJobState::Completed => {
            let _ = mark_printing_job_terminal(
                state.db_path.as_ref(),
                job_id,
                JOB_STATUS_SUCCEEDED,
                "printer backend confirmed completion",
                None,
                None,
                job_kind,
                source_file_path,
            );
        }
        BackendJobState::Canceled => {
            let _ = mark_printing_job_terminal(
                state.db_path.as_ref(),
                job_id,
                JOB_STATUS_CANCELED,
                "printer backend confirmed cancelation",
                Some("BACKEND_JOB_CANCELED"),
                Some("job canceled by printer backend"),
                job_kind,
                source_file_path,
            );
        }
        BackendJobState::Failed => {
            let _ = mark_printing_job_terminal(
                state.db_path.as_ref(),
                job_id,
                JOB_STATUS_FAILED,
                "printer backend confirmed failure",
                Some("BACKEND_JOB_FAILED"),
                Some("job failed on printer backend"),
                job_kind,
                source_file_path,
            );
        }
        BackendJobState::Pending | BackendJobState::Processing => {}
        BackendJobState::Unknown => handle_unknown_status(state, job_id, unknown_since_at, now),
    }
}
