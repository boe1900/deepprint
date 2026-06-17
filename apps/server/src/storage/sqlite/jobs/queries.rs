use std::path::Path;

use rusqlite::{params, params_from_iter, types::Value as SqlValue, Connection, OptionalExtension};

use super::{
    models::{DiagnosticFailedJobSnapshot, JobRecord},
    rows::{build_jobs_filters_clause, map_failed_job_snapshot_row, map_job_row},
};
use crate::storage::sqlite::open_connection;

pub fn fetch_job_by_id(conn: &Connection, job_id: &str) -> rusqlite::Result<Option<JobRecord>> {
    conn.query_row(
        "SELECT
            id,
            request_id,
            job_kind,
            printer_id,
            printer_name_snapshot,
            printer_uri,
            template_content,
            status,
            attempt_count,
            created_at,
            updated_at,
            last_error_code,
            last_error_message,
            render_artifact_path,
            render_output_kind,
            render_page_count,
            render_page_width_pt,
            render_page_height_pt,
            data_json,
            print_options_json,
            backend_name,
            backend_job_ref_json,
            submit_started_at,
            submitted_at,
            last_polled_at,
            backend_state,
            backend_state_message,
            unknown_since_at,
            needs_attention_reason,
            source_file_path,
            source_file_name,
            source_content_type,
            source_file_size_bytes
         FROM jobs
         WHERE id = ?1",
        params![job_id],
        map_job_row,
    )
    .optional()
}

pub fn fetch_job_by_id_at_path(
    db_path: &Path,
    job_id: &str,
) -> rusqlite::Result<Option<JobRecord>> {
    let conn = open_connection(db_path)?;
    fetch_job_by_id(&conn, job_id)
}

pub fn fetch_job_by_request_id(
    conn: &Connection,
    request_id: &str,
) -> rusqlite::Result<Option<JobRecord>> {
    conn.query_row(
        "SELECT
            id,
            request_id,
            job_kind,
            printer_id,
            printer_name_snapshot,
            printer_uri,
            template_content,
            status,
            attempt_count,
            created_at,
            updated_at,
            last_error_code,
            last_error_message,
            render_artifact_path,
            render_output_kind,
            render_page_count,
            render_page_width_pt,
            render_page_height_pt,
            data_json,
            print_options_json,
            backend_name,
            backend_job_ref_json,
            submit_started_at,
            submitted_at,
            last_polled_at,
            backend_state,
            backend_state_message,
            unknown_since_at,
            needs_attention_reason,
            source_file_path,
            source_file_name,
            source_content_type,
            source_file_size_bytes
         FROM jobs
         WHERE request_id = ?1",
        params![request_id],
        map_job_row,
    )
    .optional()
}

pub fn fetch_job_by_request_id_at_path(
    db_path: &Path,
    request_id: &str,
) -> rusqlite::Result<Option<JobRecord>> {
    let conn = open_connection(db_path)?;
    fetch_job_by_request_id(&conn, request_id)
}

pub fn probe_database_health_at_path(db_path: &Path) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    conn.query_row("SELECT 1", [], |_| Ok::<_, rusqlite::Error>(()))
}

pub fn count_jobs(
    conn: &Connection,
    statuses: &[&str],
    printer_id: Option<&str>,
    search_query: Option<&str>,
) -> rusqlite::Result<usize> {
    let (where_clause, params) = build_jobs_filters_clause(statuses, printer_id, search_query);
    let sql = format!("SELECT COUNT(1) FROM jobs {where_clause}");
    let count: i64 = conn.query_row(&sql, params_from_iter(params), |row| row.get(0))?;
    Ok(count.max(0) as usize)
}

pub fn count_jobs_at_path(
    db_path: &Path,
    statuses: &[&str],
    printer_id: Option<&str>,
    search_query: Option<&str>,
) -> rusqlite::Result<usize> {
    let conn = open_connection(db_path)?;
    count_jobs(&conn, statuses, printer_id, search_query)
}

pub fn list_jobs_page(
    conn: &Connection,
    statuses: &[&str],
    printer_id: Option<&str>,
    search_query: Option<&str>,
    offset: usize,
    limit: usize,
) -> rusqlite::Result<Vec<JobRecord>> {
    let (where_clause, mut params) = build_jobs_filters_clause(statuses, printer_id, search_query);
    let sql = format!(
        "SELECT
            id,
            request_id,
            job_kind,
            printer_id,
            printer_name_snapshot,
            printer_uri,
            template_content,
            status,
            attempt_count,
            created_at,
            updated_at,
            last_error_code,
            last_error_message,
            render_artifact_path,
            render_output_kind,
            render_page_count,
            render_page_width_pt,
            render_page_height_pt,
            data_json,
            print_options_json,
            backend_name,
            backend_job_ref_json,
            submit_started_at,
            submitted_at,
            last_polled_at,
            backend_state,
            backend_state_message,
            unknown_since_at,
            needs_attention_reason,
            source_file_path,
            source_file_name,
            source_content_type,
            source_file_size_bytes
         FROM jobs
         {where_clause}
         ORDER BY updated_at DESC, created_at DESC, id DESC
         LIMIT ? OFFSET ?"
    );
    params.push(SqlValue::Integer(limit as i64));
    params.push(SqlValue::Integer(offset as i64));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params), map_job_row)?;
    rows.collect()
}

pub fn list_jobs_page_at_path(
    db_path: &Path,
    statuses: &[&str],
    printer_id: Option<&str>,
    search_query: Option<&str>,
    offset: usize,
    limit: usize,
) -> rusqlite::Result<Vec<JobRecord>> {
    let conn = open_connection(db_path)?;
    list_jobs_page(&conn, statuses, printer_id, search_query, offset, limit)
}

pub fn list_recent_jobs_records(
    conn: &Connection,
    printer_id: Option<&str>,
    limit: usize,
) -> rusqlite::Result<Vec<JobRecord>> {
    let (where_clause, mut params) = build_jobs_filters_clause(&[], printer_id, None);
    let sql = format!(
        "SELECT
            id,
            request_id,
            job_kind,
            printer_id,
            printer_name_snapshot,
            printer_uri,
            template_content,
            status,
            attempt_count,
            created_at,
            updated_at,
            last_error_code,
            last_error_message,
            render_artifact_path,
            render_output_kind,
            render_page_count,
            render_page_width_pt,
            render_page_height_pt,
            data_json,
            print_options_json,
            backend_name,
            backend_job_ref_json,
            submit_started_at,
            submitted_at,
            last_polled_at,
            backend_state,
            backend_state_message,
            unknown_since_at,
            needs_attention_reason,
            source_file_path,
            source_file_name,
            source_content_type,
            source_file_size_bytes
         FROM jobs
         {where_clause}
         ORDER BY updated_at DESC, created_at DESC, id DESC
         LIMIT ?"
    );
    params.push(SqlValue::Integer(limit as i64));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params), map_job_row)?;
    rows.collect()
}

pub fn list_recent_jobs_records_at_path(
    db_path: &Path,
    printer_id: Option<&str>,
    limit: usize,
) -> rusqlite::Result<Vec<JobRecord>> {
    let conn = open_connection(db_path)?;
    list_recent_jobs_records(&conn, printer_id, limit)
}

pub fn load_failed_jobs_snapshot(
    conn: &Connection,
    limit: u64,
) -> rusqlite::Result<Vec<DiagnosticFailedJobSnapshot>> {
    let max_limit = limit.clamp(1, 5000) as i64;
    let mut stmt = conn.prepare(
        "SELECT
            id,
            request_id,
            status,
            attempt_count,
            updated_at,
            last_error_code,
            last_error_message,
            backend_name,
            backend_job_ref_json
         FROM jobs
         WHERE status IN ('failed', 'canceled')
         ORDER BY updated_at DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![max_limit], map_failed_job_snapshot_row)?;
    rows.collect()
}

pub fn load_failed_jobs_snapshot_at_path(
    db_path: &Path,
    limit: u64,
) -> rusqlite::Result<Vec<DiagnosticFailedJobSnapshot>> {
    let conn = open_connection(db_path)?;
    load_failed_jobs_snapshot(&conn, limit)
}

pub fn count_inflight_jobs_for_printer(
    conn: &Connection,
    printer_id: &str,
) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(1)
         FROM jobs
         WHERE printer_id = ?1
           AND status IN ('queued', 'rendering', 'submitting', 'printing')",
        params![printer_id],
        |row| row.get(0),
    )
}

pub fn count_inflight_jobs_for_printer_at_path(
    db_path: &Path,
    printer_id: &str,
) -> rusqlite::Result<i64> {
    let conn = open_connection(db_path)?;
    count_inflight_jobs_for_printer(&conn, printer_id)
}

pub fn count_active_jobs_by_artifact_path(
    conn: &Connection,
    artifact_path: &str,
) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(1) FROM jobs
         WHERE status IN ('rendering', 'submitting', 'printing')
           AND render_artifact_path = ?1",
        params![artifact_path],
        |row| row.get(0),
    )
}
