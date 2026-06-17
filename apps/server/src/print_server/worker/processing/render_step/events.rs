use super::super::super::super::AgentState;
use crate::storage::{
    increment_agent_metric, try_insert_job_event, METRIC_RENDER_CACHE_HIT_TOTAL,
    METRIC_RENDER_CACHE_MISS_TOTAL,
};

pub(super) fn record_direct_file_ready(state: &AgentState, job_id: &str) {
    let _ = try_insert_job_event(
        state.db_path.as_ref(),
        job_id,
        "direct_file_ready",
        Some("rendering"),
        Some("rendering"),
        "skip template render and use uploaded file directly",
    );
}

pub(super) fn record_render_cache_hit(state: &AgentState, job_id: &str) {
    let _ = increment_agent_metric(state.db_path.as_ref(), METRIC_RENDER_CACHE_HIT_TOTAL, 1);
    let _ = try_insert_job_event(
        state.db_path.as_ref(),
        job_id,
        "render_cache_hit",
        Some("rendering"),
        Some("rendering"),
        "render cache hit, skip subprocess rendering",
    );
}

pub(super) fn record_render_cache_miss(state: &AgentState, job_id: &str) {
    let _ = increment_agent_metric(state.db_path.as_ref(), METRIC_RENDER_CACHE_MISS_TOTAL, 1);
    let _ = try_insert_job_event(
        state.db_path.as_ref(),
        job_id,
        "render_cache_miss",
        Some("rendering"),
        Some("rendering"),
        "render cache miss, run subprocess rendering",
    );
}
