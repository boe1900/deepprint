use tracing::warn;

use super::super::super::{
    models::{JOB_STATUS_PRINTING, JOB_STATUS_SUBMITTING},
    utils::now_unix,
    AgentState,
};
use crate::storage::{
    list_submitting_jobs_for_monitor, move_submitting_job_to_attention,
    save_reconciled_backend_submission, transition_job_status,
};

pub(crate) async fn monitor_submitting_jobs(state: &AgentState) -> rusqlite::Result<()> {
    let jobs = list_submitting_jobs_for_monitor(state.db_path.as_ref())?;
    let now = now_unix();

    for job in jobs {
        let job_id = job.id;
        let printer_uri = job.printer_uri;
        let submit_started_at = job.submit_started_at;
        let backend_job_ref_json = job.backend_job_ref_json;
        if has_backend_submission(backend_job_ref_json.as_deref()) {
            transition_submitting_to_printing(
                state,
                &job_id,
                "backend submission persisted, move to printing monitor",
            )?;
            continue;
        }

        if let Some(printer_uri) = printer_uri
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if reconcile_backend_submission(state, &job_id, printer_uri, submit_started_at).await? {
                continue;
            }
        }

        move_to_attention_if_recovery_expired(state, &job_id, submit_started_at, now)?;
    }

    Ok(())
}

async fn reconcile_backend_submission(
    state: &AgentState,
    job_id: &str,
    printer_uri: &str,
    submit_started_at: Option<i64>,
) -> rusqlite::Result<bool> {
    let backend = state.backend.clone();
    let printer_uri = printer_uri.to_string();
    let job_name = format!("deepprint:{job_id}");
    let reconcile_result = tokio::task::spawn_blocking(move || {
        backend.reconcile_submission(&printer_uri, &job_name, submit_started_at)
    })
    .await;

    match reconcile_result {
        Ok(Ok(Some(recovered_backend_job_ref_json))) => {
            let saved = save_reconciled_backend_submission(
                state.db_path.as_ref(),
                job_id,
                state.backend.backend_name(),
                &recovered_backend_job_ref_json,
            )?;
            if saved {
                transition_submitting_to_printing(
                    state,
                    job_id,
                    "backend submission reconciled, move to printing monitor",
                )?;
            }
            Ok(true)
        }
        Ok(Ok(None)) => Ok(false),
        Ok(Err(err)) => {
            if !err.retryable() {
                let _ = move_submitting_job_to_attention(
                    state.db_path.as_ref(),
                    job_id,
                    "submission recovery failed with non-retryable backend error",
                    err.code(),
                    err.code(),
                    err.message(),
                );
                return Ok(true);
            }
            warn!("submission recovery retryable error for job {job_id}: {err}");
            Ok(false)
        }
        Err(err) => {
            warn!("submission recovery join failure for job {job_id}: {err}");
            Ok(false)
        }
    }
}

fn transition_submitting_to_printing(
    state: &AgentState,
    job_id: &str,
    message: &str,
) -> rusqlite::Result<()> {
    let _ = transition_job_status(
        state.db_path.as_ref(),
        job_id,
        JOB_STATUS_SUBMITTING,
        JOB_STATUS_PRINTING,
        message,
        None,
        None,
    )?;
    Ok(())
}

fn move_to_attention_if_recovery_expired(
    state: &AgentState,
    job_id: &str,
    submit_started_at: Option<i64>,
    now: i64,
) -> rusqlite::Result<()> {
    let started_at = submit_started_at.unwrap_or(now);
    if now.saturating_sub(started_at) >= state.config.submission_recovery_timeout_sec as i64 {
        let _ = move_submitting_job_to_attention(
            state.db_path.as_ref(),
            job_id,
            "submission result remains unknown after recovery timeout",
            "SUBMISSION_RECOVERY_TIMEOUT",
            "submission_recovery_timeout",
            "submission may have reached backend; manual attention required",
        );
    }
    Ok(())
}

fn has_backend_submission(backend_job_ref_json: Option<&str>) -> bool {
    backend_job_ref_json
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
}
