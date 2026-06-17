use std::path::Path;

use super::super::super::{models::ProcessJobError, JobRecord};
use crate::storage::fetch_job_by_id_at_path;

pub(crate) fn load_job_for_processing(
    db_path: &Path,
    job_id: &str,
) -> Result<JobRecord, ProcessJobError> {
    fetch_job_by_id_at_path(db_path, job_id)
        .map_err(|err| ProcessJobError::new("DB_READ_FAILED", err.to_string()))?
        .ok_or_else(|| ProcessJobError::new("JOB_NOT_FOUND", format!("job not found: {job_id}")))
}
