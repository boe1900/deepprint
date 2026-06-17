use std::path::Path;

use rusqlite::{params, Connection};

use super::{
    events::insert_job_event,
    models::{DirectJobInsertInput, RenderArtifactJobUpdateInput, TemplateJobInsertInput},
    time::now_unix,
};
use crate::storage::sqlite::open_connection;

pub fn insert_template_job(
    conn: &Connection,
    input: TemplateJobInsertInput<'_>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO jobs (
            id,
            request_id,
            job_kind,
            printer_id,
            printer_name_snapshot,
            printer_uri,
            template_content,
            data_json,
            print_options_json,
            source_file_path,
            source_file_name,
            source_content_type,
            source_file_size_bytes,
            status,
            attempt_count,
            created_at,
            updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, NULL, NULL, NULL, 'queued', 0, ?10, ?10)",
        params![
            input.id,
            input.request_id,
            "template",
            input.printer_id,
            input.printer_name_snapshot,
            input.printer_uri,
            input.template_content,
            input.data_json,
            input.print_options_json,
            input.created_at,
        ],
    )?;
    insert_job_event(
        conn,
        input.id,
        "accepted",
        None,
        Some("queued"),
        "job accepted",
    )?;
    Ok(())
}

pub fn insert_direct_job(
    conn: &Connection,
    input: DirectJobInsertInput<'_>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO jobs (
            id,
            request_id,
            job_kind,
            printer_id,
            printer_name_snapshot,
            printer_uri,
            template_content,
            data_json,
            print_options_json,
            source_file_path,
            source_file_name,
            source_content_type,
            source_file_size_bytes,
            status,
            attempt_count,
            created_at,
            updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', ?7, ?8, ?9, ?10, ?11, ?12, 'queued', 0, ?13, ?13)",
        params![
            input.id,
            input.request_id,
            "direct_file",
            input.printer_id,
            input.printer_name_snapshot,
            input.printer_uri,
            input.data_json,
            input.print_options_json,
            input.source_file_path,
            input.source_file_name,
            input.source_content_type,
            input.source_file_size_bytes,
            input.created_at,
        ],
    )?;
    insert_job_event(
        conn,
        input.id,
        "accepted",
        None,
        Some("queued"),
        "direct file job accepted",
    )?;
    Ok(())
}

pub fn insert_template_job_at_path(
    db_path: &Path,
    input: TemplateJobInsertInput<'_>,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    insert_template_job(&conn, input)
}

pub fn insert_direct_job_at_path(
    db_path: &Path,
    input: DirectJobInsertInput<'_>,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    insert_direct_job(&conn, input)
}

pub fn save_render_artifact_result(
    db_path: &Path,
    job_id: &str,
    input: RenderArtifactJobUpdateInput<'_>,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    conn.execute(
        "UPDATE jobs
         SET render_artifact_path = ?1,
             render_output_kind = ?2,
             render_page_count = ?3,
             render_page_width_pt = ?4,
             render_page_height_pt = ?5,
             updated_at = ?6
         WHERE id = ?7",
        params![
            input.artifact_path,
            input.output_kind,
            input.page_count,
            input.page_width_pt,
            input.page_height_pt,
            now_unix(),
            job_id
        ],
    )?;
    Ok(())
}
