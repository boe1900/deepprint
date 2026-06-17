use std::path::Path;

use super::super::super::{
    models::JOB_STATUS_PRINTING, rendering::cleanup_direct_job_source, shared::JOB_KIND_DIRECT_FILE,
};
use crate::storage::{transition_job_status, try_insert_job_event};

pub(super) fn mark_printing_job_terminal(
    db_path: &Path,
    job_id: &str,
    target_status: &str,
    message: &str,
    error_code: Option<&str>,
    error_message: Option<&str>,
    job_kind: &str,
    source_file_path: Option<&str>,
) -> rusqlite::Result<()> {
    let transitioned = transition_job_status(
        db_path,
        job_id,
        JOB_STATUS_PRINTING,
        target_status,
        message,
        error_code,
        error_message,
    )?;

    if transitioned && job_kind == JOB_KIND_DIRECT_FILE {
        if let Some(source_path) = source_file_path
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            cleanup_direct_job_source(Path::new(source_path));
            let _ = try_insert_job_event(
                db_path,
                job_id,
                "direct_source_cleanup",
                None,
                None,
                target_status,
            );
        }
    }

    Ok(())
}
