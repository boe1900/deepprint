#[path = "printing/attention.rs"]
mod attention;
#[path = "printing/query.rs"]
mod query;
#[path = "printing/status.rs"]
mod status;

use super::super::super::{
    models::{JOB_STATUS_FAILED, JOB_STATUS_NEEDS_ATTENTION, JOB_STATUS_PRINTING},
    utils::now_unix,
    AgentState,
};
use super::terminal::mark_printing_job_terminal;
use crate::storage::{list_printing_jobs_for_monitor, transition_job_status};

pub(crate) async fn monitor_printing_jobs(state: &AgentState) -> rusqlite::Result<()> {
    let jobs = list_printing_jobs_for_monitor(state.db_path.as_ref())?;
    let now = now_unix();

    for job in jobs {
        let job_id = job.id;
        let backend_job_ref_json = job.backend_job_ref_json;
        let unknown_since_at = job.unknown_since_at;
        let job_kind = job.job_kind;
        let source_file_path = job.source_file_path;
        let Some(backend_job_ref_json) =
            backend_job_ref_json.filter(|value| !value.trim().is_empty())
        else {
            let _ = transition_job_status(
                state.db_path.as_ref(),
                &job_id,
                JOB_STATUS_PRINTING,
                JOB_STATUS_NEEDS_ATTENTION,
                "printing job is missing backend job id",
                Some("BACKEND_JOB_ID_MISSING"),
                Some("backend job id is missing while monitoring printing"),
            );
            continue;
        };

        let backend_status =
            match query::query_backend_job_status(state, &job_id, &backend_job_ref_json).await {
                Ok(status) => status,
                Err(query::PrintingQueryError::Retryable(message)) => {
                    attention::handle_retryable_query_error(
                        state,
                        &job_id,
                        unknown_since_at,
                        now,
                        &message,
                    );
                    continue;
                }
                Err(query::PrintingQueryError::NonRetryable { code, message }) => {
                    let _ = mark_printing_job_terminal(
                        state.db_path.as_ref(),
                        &job_id,
                        JOB_STATUS_FAILED,
                        "backend reports non-retryable query failure",
                        Some(&code),
                        Some(&message),
                        &job_kind,
                        source_file_path.as_deref(),
                    );
                    continue;
                }
                Err(query::PrintingQueryError::Join(message)) => {
                    attention::handle_query_join_error(
                        state,
                        &job_id,
                        unknown_since_at,
                        now,
                        &message,
                    );
                    continue;
                }
            };

        status::record_backend_status(state, &job_id, backend_status);
        status::apply_backend_status(
            state,
            &job_id,
            backend_status,
            unknown_since_at,
            now,
            &job_kind,
            source_file_path.as_deref(),
        );
    }

    Ok(())
}
