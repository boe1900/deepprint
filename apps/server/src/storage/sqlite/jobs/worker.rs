use std::path::Path;

use rusqlite::{params, OptionalExtension};

use super::{
    events::insert_job_event_tx,
    models::{InflightRecoverySummary, PrintingMonitorJobRecord, SubmittingMonitorJobRecord},
    time::now_unix,
};
use crate::storage::sqlite::open_connection;

pub fn list_submitting_jobs_for_monitor(
    db_path: &Path,
) -> rusqlite::Result<Vec<SubmittingMonitorJobRecord>> {
    let conn = open_connection(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, printer_uri, submit_started_at, backend_job_ref_json
         FROM jobs
         WHERE status = 'submitting'
         ORDER BY updated_at ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(SubmittingMonitorJobRecord {
            id: row.get(0)?,
            printer_uri: row.get(1)?,
            submit_started_at: row.get(2)?,
            backend_job_ref_json: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn list_printing_jobs_for_monitor(
    db_path: &Path,
) -> rusqlite::Result<Vec<PrintingMonitorJobRecord>> {
    let conn = open_connection(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, backend_job_ref_json, unknown_since_at, job_kind, source_file_path
         FROM jobs
         WHERE status = 'printing'
         ORDER BY updated_at ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(PrintingMonitorJobRecord {
            id: row.get(0)?,
            backend_job_ref_json: row.get(1)?,
            unknown_since_at: row.get(2)?,
            job_kind: row.get(3)?,
            source_file_path: row.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn recover_inflight_jobs(
    db_path: &Path,
    recover_printing: bool,
) -> rusqlite::Result<InflightRecoverySummary> {
    let conn = open_connection(db_path)?;
    let now = now_unix();
    let rendering_changed = conn.execute(
        "UPDATE jobs
         SET status = 'queued',
             next_retry_at = ?1,
             updated_at = ?1,
             last_error_code = NULL,
             last_error_message = NULL
         WHERE status = 'rendering'",
        params![now],
    )?;

    let printing_changed = if recover_printing {
        conn.execute(
            "UPDATE jobs
             SET status = 'queued',
                 next_retry_at = ?1,
                 updated_at = ?1,
                 last_error_code = NULL,
                 last_error_message = NULL,
                 backend_name = NULL,
                 backend_job_ref_json = NULL,
                 submit_started_at = NULL,
                 submitted_at = NULL,
                 last_polled_at = NULL,
                 backend_state = NULL,
                 backend_state_message = NULL,
                 unknown_since_at = NULL,
                 needs_attention_reason = NULL
             WHERE status = 'printing'",
            params![now],
        )?
    } else {
        0
    };

    Ok(InflightRecoverySummary {
        rendering_requeued: rendering_changed.max(0) as usize,
        printing_requeued: printing_changed.max(0) as usize,
    })
}

pub fn claim_next_job(db_path: &Path) -> rusqlite::Result<Option<String>> {
    let mut conn = open_connection(db_path)?;
    let tx = conn.transaction()?;
    let now = now_unix();

    let candidate: Option<String> = tx
        .query_row(
            "SELECT j.id
             FROM jobs j
             WHERE j.status = 'queued'
               AND COALESCE(j.next_retry_at, 0) <= ?1
               AND NOT EXISTS (
                   SELECT 1
                   FROM jobs active
                   WHERE active.printer_id IS NOT NULL
                     AND active.printer_id = j.printer_id
                     AND active.status IN ('rendering', 'submitting', 'printing')
               )
             ORDER BY COALESCE(j.next_retry_at, 0) ASC, j.created_at ASC
             LIMIT 1",
            params![now],
            |row| row.get(0),
        )
        .optional()?;

    let Some(job_id) = candidate else {
        tx.commit()?;
        return Ok(None);
    };

    let changed = tx.execute(
        "UPDATE jobs
         SET status = 'rendering',
             attempt_count = attempt_count + 1,
             next_retry_at = NULL,
             updated_at = ?1
         WHERE id = ?2 AND status = 'queued'",
        params![now, job_id],
    )?;

    if changed == 1 {
        insert_job_event_tx(
            &tx,
            &job_id,
            "worker_claimed",
            Some("queued"),
            Some("rendering"),
            "job claimed by worker",
        )?;
        tx.commit()?;
        return Ok(Some(job_id));
    }

    tx.commit()?;
    Ok(None)
}
