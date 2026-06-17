use std::{sync::Arc, time::Duration};

use tracing::{info, warn};

use super::super::super::{models::ProcessJobError, shared::handle_job_failure, AgentState};
use super::process_job;
use crate::storage::claim_next_job;

pub(crate) async fn worker_loop(state: Arc<AgentState>, worker_index: u16) {
    let interval = Duration::from_millis(state.config.worker_poll_ms.max(100));
    info!(
        "worker loop started, worker_index={}, poll_ms={}",
        worker_index,
        interval.as_millis()
    );

    loop {
        match claim_next_job(state.db_path.as_ref()) {
            Ok(Some(job_id)) => handle_claimed_job(state.clone(), worker_index, &job_id).await,
            Ok(None) => tokio::time::sleep(interval).await,
            Err(err) => {
                warn!("worker loop claim_next_job failed on worker_index={worker_index}: {err}");
                tokio::time::sleep(interval).await;
            }
        }
    }
}

async fn handle_claimed_job(state: Arc<AgentState>, worker_index: u16, job_id: &str) {
    if let Err(err) = process_job(state.clone(), job_id).await {
        warn!(
            "job processing failed for {job_id} on worker_index={worker_index}: [{}] {}",
            err.code, err.message
        );
        update_failed_job(state.as_ref(), worker_index, job_id, &err);
    }
}

fn update_failed_job(state: &AgentState, worker_index: u16, job_id: &str, err: &ProcessJobError) {
    if let Err(update_err) = handle_job_failure(
        state.db_path.as_ref(),
        job_id,
        err,
        state.config.retry_max_attempts,
        state.config.retry_backoff_base_sec,
        state.config.retry_backoff_max_sec,
    ) {
        warn!(
            "job failure update failed for {job_id} on worker_index={worker_index}: {update_err}"
        );
    }
}
