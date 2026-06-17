use std::path::Path;

use rusqlite::{params, Connection, Transaction};
use uuid::Uuid;

use super::time::now_unix;
use crate::storage::sqlite::open_connection;

pub fn try_insert_job_event(
    db_path: &Path,
    job_id: &str,
    event_type: &str,
    from_status: Option<&str>,
    to_status: Option<&str>,
    message: &str,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    insert_job_event(&conn, job_id, event_type, from_status, to_status, message)
}

pub fn insert_job_event(
    conn: &Connection,
    job_id: &str,
    event_type: &str,
    from_status: Option<&str>,
    to_status: Option<&str>,
    message: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO job_events (id, job_id, event_type, from_status, to_status, message, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            Uuid::new_v4().to_string(),
            job_id,
            event_type,
            from_status,
            to_status,
            message,
            now_unix(),
        ],
    )?;
    Ok(())
}

pub fn insert_job_event_tx(
    tx: &Transaction<'_>,
    job_id: &str,
    event_type: &str,
    from_status: Option<&str>,
    to_status: Option<&str>,
    message: &str,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO job_events (id, job_id, event_type, from_status, to_status, message, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            Uuid::new_v4().to_string(),
            job_id,
            event_type,
            from_status,
            to_status,
            message,
            now_unix(),
        ],
    )?;
    Ok(())
}
