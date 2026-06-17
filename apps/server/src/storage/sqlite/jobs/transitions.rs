use std::path::Path;

use rusqlite::{params, Connection};

use super::{events::insert_job_event, time::now_unix};
use crate::storage::sqlite::open_connection;

pub fn save_backend_submission(
    db_path: &Path,
    job_id: &str,
    backend_name: &str,
    backend_job_ref_json: Option<&str>,
) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    let now = now_unix();
    let has_ref = backend_job_ref_json
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let backend_state = if has_ref {
        Some("accepted")
    } else {
        Some("submission_unknown")
    };
    let backend_state_message = if has_ref {
        Some("backend accepted submission")
    } else {
        Some("submission returned without backend job reference")
    };
    let changed = conn.execute(
        "UPDATE jobs
         SET backend_name = ?1,
             backend_job_ref_json = ?2,
             backend_state = ?3,
             backend_state_message = ?4,
             submitted_at = CASE
               WHEN ?2 IS NOT NULL AND TRIM(?2) <> '' THEN ?5
               ELSE submitted_at
             END,
             unknown_since_at = NULL,
             last_polled_at = NULL,
             needs_attention_reason = NULL,
             updated_at = ?5
         WHERE id = ?6 AND status = 'submitting'",
        params![
            backend_name,
            backend_job_ref_json,
            backend_state,
            backend_state_message,
            now,
            job_id,
        ],
    )?;

    if changed == 1 {
        insert_job_event(
            &conn,
            job_id,
            "backend_submitted",
            Some("submitting"),
            Some("submitting"),
            &format!("submitted to backend={backend_name}"),
        )?;
    }

    Ok(changed == 1)
}

pub fn save_reconciled_backend_submission(
    db_path: &Path,
    job_id: &str,
    backend_name: &str,
    backend_job_ref_json: &str,
) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    let now = now_unix();
    let changed = conn.execute(
        "UPDATE jobs
         SET backend_name = ?1,
             backend_job_ref_json = ?2,
             backend_state = 'accepted',
             backend_state_message = 'backend submission reconciled from remote queue',
             submitted_at = ?3,
             unknown_since_at = NULL,
             last_polled_at = NULL,
             needs_attention_reason = NULL,
             updated_at = ?3
         WHERE id = ?4 AND status = 'submitting'",
        params![backend_name, backend_job_ref_json, now, job_id],
    )?;

    if changed == 1 {
        insert_job_event(
            &conn,
            job_id,
            "submission_reconciled",
            Some("submitting"),
            Some("submitting"),
            &format!("reconciled backend submission to backend={backend_name}"),
        )?;
    }

    Ok(changed == 1)
}

pub fn transition_job_status(
    db_path: &Path,
    job_id: &str,
    from_status: &str,
    to_status: &str,
    message: &str,
    error_code: Option<&str>,
    error_message: Option<&str>,
) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    let now = now_unix();
    let submit_started_at = if to_status == "submitting" {
        Some(now)
    } else {
        None
    };
    let reset_unknown_state = matches!(
        to_status,
        "printing" | "succeeded" | "failed" | "canceled" | "queued"
    );
    let changed = conn.execute(
        "UPDATE jobs
         SET status = ?1,
             updated_at = ?2,
             last_error_code = ?3,
             last_error_message = ?4,
             submit_started_at = CASE
               WHEN ?7 IS NOT NULL THEN ?7
               ELSE submit_started_at
             END,
             unknown_since_at = CASE
               WHEN ?8 THEN NULL
               ELSE unknown_since_at
             END,
             needs_attention_reason = CASE
               WHEN ?1 <> ?9 THEN NULL
               ELSE ?10
             END
         WHERE id = ?5 AND status = ?6",
        params![
            to_status,
            now,
            error_code,
            error_message,
            job_id,
            from_status,
            submit_started_at,
            reset_unknown_state,
            "needs_attention",
            error_code,
        ],
    )?;

    if changed == 1 {
        insert_job_event(
            &conn,
            job_id,
            "status_transition",
            Some(from_status),
            Some(to_status),
            message,
        )?;
    }

    Ok(changed == 1)
}

pub fn schedule_job_retry(
    conn: &Connection,
    job_id: &str,
    from_status: &str,
    error_code: &str,
    error_message: &str,
    next_retry_at: i64,
    event_message: &str,
) -> rusqlite::Result<bool> {
    let now = now_unix();
    let changed = conn.execute(
        "UPDATE jobs
         SET status = 'queued',
             updated_at = ?1,
             last_error_code = ?2,
             last_error_message = ?3,
             next_retry_at = ?4
         WHERE id = ?5 AND status = ?6",
        params![
            now,
            error_code,
            error_message,
            next_retry_at,
            job_id,
            from_status,
        ],
    )?;

    if changed == 1 {
        insert_job_event(
            conn,
            job_id,
            "retry_scheduled",
            Some(from_status),
            Some("queued"),
            event_message,
        )?;
    }

    Ok(changed == 1)
}

pub fn move_job_to_attention(
    conn: &Connection,
    job_id: &str,
    from_status: &str,
    error_code: &str,
    error_message: &str,
    needs_attention_reason: &str,
    event_message: &str,
) -> rusqlite::Result<bool> {
    let now = now_unix();
    let changed = conn.execute(
        "UPDATE jobs
         SET status = 'needs_attention',
             updated_at = ?1,
             last_error_code = ?2,
             last_error_message = ?3,
             needs_attention_reason = ?4,
             next_retry_at = NULL,
             backend_state_message = CASE
               WHEN ?5 IN ('submitting', 'printing') THEN ?3
               ELSE backend_state_message
             END
         WHERE id = ?6 AND status = ?5",
        params![
            now,
            error_code,
            error_message,
            needs_attention_reason,
            from_status,
            job_id,
        ],
    )?;

    if changed == 1 {
        insert_job_event(
            conn,
            job_id,
            "needs_attention",
            Some(from_status),
            Some("needs_attention"),
            event_message,
        )?;
    }

    Ok(changed == 1)
}

pub fn record_backend_poll_result(
    db_path: &Path,
    job_id: &str,
    backend_state: &str,
    backend_state_message: Option<&str>,
    reset_unknown_since: bool,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    conn.execute(
        "UPDATE jobs
         SET last_polled_at = ?1,
             backend_state = ?2,
             backend_state_message = ?3,
             unknown_since_at = CASE
               WHEN ?4 THEN NULL
               ELSE unknown_since_at
             END,
             updated_at = ?1
         WHERE id = ?5 AND status = 'printing'",
        params![
            now_unix(),
            backend_state,
            backend_state_message,
            reset_unknown_since,
            job_id,
        ],
    )?;
    Ok(())
}

pub fn record_backend_unknown(
    db_path: &Path,
    job_id: &str,
    backend_state: &str,
    backend_state_message: &str,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    let now = now_unix();
    conn.execute(
        "UPDATE jobs
         SET last_polled_at = ?1,
             backend_state = ?2,
             backend_state_message = ?3,
             unknown_since_at = COALESCE(unknown_since_at, ?1),
             updated_at = ?1
         WHERE id = ?4 AND status = 'printing'",
        params![now, backend_state, backend_state_message, job_id],
    )?;
    Ok(())
}

pub fn move_printing_job_to_attention(
    db_path: &Path,
    job_id: &str,
    reason_code: &str,
    reason_message: &str,
    event_message: &str,
) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    let now = now_unix();
    let changed = conn.execute(
        "UPDATE jobs
         SET status = ?1,
             updated_at = ?2,
             last_error_code = ?3,
             last_error_message = ?4,
             needs_attention_reason = ?3,
             backend_state_message = ?4
         WHERE id = ?5 AND status = 'printing'",
        params!["needs_attention", now, reason_code, reason_message, job_id],
    )?;

    if changed == 1 {
        insert_job_event(
            &conn,
            job_id,
            "needs_attention",
            Some("printing"),
            Some("needs_attention"),
            event_message,
        )?;
    }

    Ok(changed == 1)
}

pub fn move_submitting_job_to_attention(
    db_path: &Path,
    job_id: &str,
    event_message: &str,
    error_code: &str,
    needs_attention_reason: &str,
    error_message: &str,
) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    let now = now_unix();
    let changed = conn.execute(
        "UPDATE jobs
         SET status = ?1,
             updated_at = ?2,
             last_error_code = ?3,
             last_error_message = ?4,
             needs_attention_reason = ?5,
             backend_state_message = ?4
         WHERE id = ?6 AND status = 'submitting'",
        params![
            "needs_attention",
            now,
            error_code,
            error_message,
            needs_attention_reason,
            job_id,
        ],
    )?;

    if changed == 1 {
        insert_job_event(
            &conn,
            job_id,
            "needs_attention",
            Some("submitting"),
            Some("needs_attention"),
            event_message,
        )?;
    }

    Ok(changed == 1)
}
