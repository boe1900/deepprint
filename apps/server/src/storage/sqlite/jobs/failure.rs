use std::path::Path;

use rusqlite::{params, Connection};
use uuid::Uuid;

use super::{
    events::insert_job_event,
    models::{JobFailureHandlingResult, JobFailureInput},
    queries::fetch_job_by_id_at_path,
    time::now_unix,
    transitions::{move_job_to_attention, schedule_job_retry},
};
use crate::storage::sqlite::open_connection;

pub fn mark_job_failed_terminal(
    conn: &Connection,
    job_id: &str,
    error_code: &str,
    error_message: &str,
    now: i64,
) -> rusqlite::Result<()> {
    let changed = conn.execute(
        "UPDATE jobs
         SET status = 'failed',
             updated_at = ?1,
             last_error_code = ?2,
             last_error_message = ?3,
             next_retry_at = NULL
         WHERE id = ?4 AND status IN (?5, ?6, ?7)",
        params![
            now,
            error_code,
            error_message,
            job_id,
            "queued",
            "rendering",
            "needs_attention"
        ],
    )?;

    if changed == 1 {
        insert_job_event(
            conn,
            job_id,
            "status_transition",
            None,
            Some("failed"),
            "job moved to failed",
        )?;
    }

    Ok(())
}

pub fn upsert_dead_letter(
    conn: &Connection,
    job_id: &str,
    request_id: &str,
    error_code: &str,
    error_message: &str,
    attempts: i64,
    failed_at: i64,
) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "INSERT INTO dead_letter (
           id,
           job_id,
           request_id,
           final_error_code,
           final_error_message,
           attempts,
           failed_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(job_id) DO NOTHING",
        params![
            Uuid::new_v4().to_string(),
            job_id,
            request_id,
            error_code,
            error_message,
            attempts,
            failed_at,
        ],
    )?;
    Ok(changed == 1)
}

fn compute_retry_backoff_sec(attempt_count: u16, base_sec: u64, max_sec: u64) -> u64 {
    let exp = attempt_count.saturating_sub(1).min(20) as u32;
    let raw = base_sec.saturating_mul(2_u64.saturating_pow(exp));
    raw.min(max_sec)
}

pub fn handle_job_failure_at_path(
    db_path: &Path,
    input: JobFailureInput<'_>,
) -> rusqlite::Result<JobFailureHandlingResult> {
    let Some(job) = fetch_job_by_id_at_path(db_path, input.job_id)? else {
        return Ok(JobFailureHandlingResult {
            cleaned_direct_source_job: None,
        });
    };
    let conn = open_connection(db_path)?;
    let now = now_unix();
    let safe_submit_retry_codes = ["IPP_SUBMIT_STATUS_FAILED", "IPP_SUBMIT_FAILED"];
    let safe_to_retry_submission = input.retryable
        && job.status == "submitting"
        && job.backend_job_ref_json.as_deref().is_none()
        && safe_submit_retry_codes.contains(&input.error_code)
        && (job.attempt_count as u16) < input.retry_max_attempts;
    let can_retry = input.retryable
        && job.status == "rendering"
        && (job.attempt_count as u16) < input.retry_max_attempts;

    if can_retry || safe_to_retry_submission {
        let from_status = if safe_to_retry_submission {
            "submitting"
        } else {
            "rendering"
        };
        let delay_sec = compute_retry_backoff_sec(
            job.attempt_count.max(1) as u16,
            input.retry_backoff_base_sec,
            input.retry_backoff_max_sec,
        );
        let next_retry_at = now.saturating_add(delay_sec as i64);

        let changed = schedule_job_retry(
            &conn,
            input.job_id,
            from_status,
            input.error_code,
            input.error_message,
            next_retry_at,
            &format!(
                "retry scheduled in {}s from {} (attempt {}/{})",
                delay_sec, from_status, job.attempt_count, input.retry_max_attempts
            ),
        )?;

        if changed {
            crate::storage::increment_agent_metric_conn(&conn, input.retry_metric_key, 1)?;
        }

        return Ok(JobFailureHandlingResult {
            cleaned_direct_source_job: None,
        });
    }

    if matches!(job.status.as_str(), "submitting" | "printing") {
        let _changed = move_job_to_attention(
            &conn,
            input.job_id,
            job.status.as_str(),
            input.error_code,
            input.error_message,
            input.error_code,
            "job requires manual attention after backend uncertainty",
        )?;

        return Ok(JobFailureHandlingResult {
            cleaned_direct_source_job: None,
        });
    }

    mark_job_failed_terminal(
        &conn,
        input.job_id,
        input.error_code,
        input.error_message,
        now,
    )?;

    if input.retryable && (job.attempt_count as u16) >= input.retry_max_attempts {
        let inserted = upsert_dead_letter(
            &conn,
            input.job_id,
            job.request_id.as_str(),
            input.error_code,
            input.error_message,
            job.attempt_count,
            now,
        )?;
        if inserted {
            crate::storage::increment_agent_metric_conn(&conn, input.dead_letter_metric_key, 1)?;
            insert_job_event(
                &conn,
                input.job_id,
                "dead_letter",
                Some(job.status.as_str()),
                Some("failed"),
                &format!(
                    "job moved to dead letter after {} attempts",
                    job.attempt_count
                ),
            )?;
        }
    }

    let cleaned_direct_source_job = if job.job_kind == "direct_file" {
        Some(job)
    } else {
        None
    };

    Ok(JobFailureHandlingResult {
        cleaned_direct_source_job,
    })
}
