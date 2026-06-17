use super::super::super::super::AgentState;
use crate::storage::{move_printing_job_to_attention, record_backend_unknown};

pub(super) fn handle_retryable_query_error(
    state: &AgentState,
    job_id: &str,
    unknown_since_at: Option<i64>,
    now: i64,
    message: &str,
) {
    let _ = record_backend_unknown(state.db_path.as_ref(), job_id, "unknown", message);
    move_to_attention_if_unknown_expired(
        state,
        job_id,
        unknown_since_at,
        now,
        "BACKEND_STATUS_UNAVAILABLE",
        message,
        "backend status remained unavailable past attention timeout",
    );
}

pub(super) fn handle_query_join_error(
    state: &AgentState,
    job_id: &str,
    unknown_since_at: Option<i64>,
    now: i64,
    message: &str,
) {
    let _ = record_backend_unknown(state.db_path.as_ref(), job_id, "unknown", message);
    move_to_attention_if_unknown_expired(
        state,
        job_id,
        unknown_since_at,
        now,
        "BACKEND_QUERY_JOIN_FAILED",
        message,
        "backend query join failure exceeded attention timeout",
    );
}

pub(super) fn handle_unknown_status(
    state: &AgentState,
    job_id: &str,
    unknown_since_at: Option<i64>,
    now: i64,
) {
    let message = "backend status remained unknown";
    let _ = record_backend_unknown(state.db_path.as_ref(), job_id, "unknown", message);
    move_to_attention_if_unknown_expired(
        state,
        job_id,
        unknown_since_at,
        now,
        "BACKEND_STATUS_UNKNOWN",
        message,
        "backend status remained unknown past attention timeout",
    );
}

fn move_to_attention_if_unknown_expired(
    state: &AgentState,
    job_id: &str,
    unknown_since_at: Option<i64>,
    now: i64,
    code: &str,
    message: &str,
    attention_message: &str,
) {
    let unknown_since = unknown_since_at.unwrap_or(now);
    if now.saturating_sub(unknown_since) >= state.config.backend_unknown_to_attention_sec as i64 {
        let _ = move_printing_job_to_attention(
            state.db_path.as_ref(),
            job_id,
            code,
            message,
            attention_message,
        );
    }
}
