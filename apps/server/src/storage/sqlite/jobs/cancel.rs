use std::path::Path;

use rusqlite::params;

use super::{events::insert_job_event, time::now_unix};
use crate::storage::sqlite::open_connection;

pub fn cancel_queued_job(db_path: &Path, job_id: &str) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    let changed = conn.execute(
        "UPDATE jobs
         SET status = 'canceled',
             updated_at = ?1
         WHERE id = ?2 AND status = 'queued'",
        params![now_unix(), job_id],
    )?;

    if changed == 1 {
        insert_job_event(
            &conn,
            job_id,
            "canceled",
            Some("queued"),
            Some("canceled"),
            "job canceled by api",
        )?;
    }

    Ok(changed == 1)
}

pub fn cancel_printing_job(db_path: &Path, job_id: &str) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    let now = now_unix();
    let changed = conn.execute(
        "UPDATE jobs
         SET status = 'canceled',
             updated_at = ?1,
             backend_state = ?2,
             backend_state_message = ?3,
             last_polled_at = ?4,
             last_error_code = ?5,
             last_error_message = ?6
         WHERE id = ?7 AND status = 'printing'",
        params![
            now,
            "canceled",
            "backend cancel accepted by api",
            now,
            "CANCELED_BY_API",
            "job canceled by api",
            job_id
        ],
    )?;

    if changed == 1 {
        insert_job_event(
            &conn,
            job_id,
            "canceled",
            Some("printing"),
            Some("canceled"),
            "job canceled by api during printing",
        )?;
    }

    Ok(changed == 1)
}

pub fn cancel_needs_attention_job(db_path: &Path, job_id: &str) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    let changed = conn.execute(
        "UPDATE jobs
         SET status = 'canceled',
             updated_at = ?1,
             last_error_code = ?2,
             last_error_message = ?3
         WHERE id = ?4 AND status = 'needs_attention'",
        params![
            now_unix(),
            "CANCELED_BY_API",
            "job canceled by api from needs_attention",
            job_id
        ],
    )?;

    if changed == 1 {
        insert_job_event(
            &conn,
            job_id,
            "canceled",
            Some("needs_attention"),
            Some("canceled"),
            "job canceled by api from needs_attention",
        )?;
    }

    Ok(changed == 1)
}
