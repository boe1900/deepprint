use rusqlite::types::Value as SqlValue;

use super::models::{DiagnosticFailedJobSnapshot, JobRecord};

pub(crate) fn build_jobs_filters_clause(
    statuses: &[&str],
    printer_id: Option<&str>,
    search_query: Option<&str>,
) -> (String, Vec<SqlValue>) {
    let mut clauses = Vec::new();
    let mut params = Vec::new();

    if !statuses.is_empty() {
        let placeholders = std::iter::repeat_n("?", statuses.len())
            .collect::<Vec<_>>()
            .join(", ");
        clauses.push(format!("status IN ({placeholders})"));
        params.extend(
            statuses
                .iter()
                .map(|status| SqlValue::Text((*status).to_string())),
        );
    }

    if let Some(printer_id) = printer_id.map(str::trim).filter(|value| !value.is_empty()) {
        clauses.push("printer_id = ?".to_string());
        params.push(SqlValue::Text(printer_id.to_string()));
    }

    if let Some(search_query) = search_query
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let pattern = build_like_pattern(search_query);
        clauses.push(
            "(id LIKE ? ESCAPE '\\'
              OR request_id LIKE ? ESCAPE '\\'
              OR job_kind LIKE ? ESCAPE '\\'
              OR printer_id LIKE ? ESCAPE '\\'
              OR printer_name_snapshot LIKE ? ESCAPE '\\'
              OR printer_uri LIKE ? ESCAPE '\\'
              OR last_error_message LIKE ? ESCAPE '\\'
              OR source_file_name LIKE ? ESCAPE '\\')"
                .to_string(),
        );
        params.extend(std::iter::repeat_n(SqlValue::Text(pattern), 8));
    }

    if clauses.is_empty() {
        (String::new(), params)
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), params)
    }
}

fn build_like_pattern(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '%' | '_' | '\\' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    format!("%{escaped}%")
}

pub(crate) fn map_job_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobRecord> {
    Ok(JobRecord {
        id: row.get(0)?,
        request_id: row.get(1)?,
        job_kind: row.get(2)?,
        printer_id: row.get(3)?,
        printer_name_snapshot: row.get(4)?,
        printer_uri: row.get(5)?,
        template_content: row.get(6)?,
        status: row.get(7)?,
        attempt_count: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        last_error_code: row.get(11)?,
        last_error_message: row.get(12)?,
        render_artifact_path: row.get(13)?,
        render_output_kind: row.get(14)?,
        render_page_count: row.get(15)?,
        render_page_width_pt: row.get(16)?,
        render_page_height_pt: row.get(17)?,
        data_json: row.get(18)?,
        print_options_json: row.get(19)?,
        backend_name: row.get(20)?,
        backend_job_ref_json: row.get(21)?,
        submit_started_at: row.get(22)?,
        submitted_at: row.get(23)?,
        last_polled_at: row.get(24)?,
        backend_state: row.get(25)?,
        backend_state_message: row.get(26)?,
        unknown_since_at: row.get(27)?,
        needs_attention_reason: row.get(28)?,
        source_file_path: row.get(29)?,
        source_file_name: row.get(30)?,
        source_content_type: row.get(31)?,
        source_file_size_bytes: row.get(32)?,
    })
}

pub(crate) fn map_failed_job_snapshot_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<DiagnosticFailedJobSnapshot> {
    Ok(DiagnosticFailedJobSnapshot {
        job_id: row.get(0)?,
        request_id: row.get(1)?,
        status: row.get(2)?,
        attempt_count: row.get(3)?,
        updated_at: row.get(4)?,
        last_error_code: row.get(5)?,
        last_error_message: row.get(6)?,
        backend_name: row.get(7)?,
        backend_job_ref_json: row.get(8)?,
    })
}
