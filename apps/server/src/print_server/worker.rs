use std::sync::Arc;

#[path = "worker/monitoring.rs"]
mod monitoring;
#[path = "worker/processing.rs"]
mod processing;

use super::AgentState;

pub(super) async fn worker_loop(state: Arc<AgentState>, worker_index: u16) {
    processing::worker_loop(state, worker_index).await;
}

pub(super) async fn monitor_submitting_jobs(state: &AgentState) -> rusqlite::Result<()> {
    monitoring::monitor_submitting_jobs(state).await
}

pub(super) async fn monitor_printing_jobs(state: &AgentState) -> rusqlite::Result<()> {
    monitoring::monitor_printing_jobs(state).await
}
