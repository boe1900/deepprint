use std::{fmt::Write as _, path::Path};

use sha2::{Digest, Sha256};

use super::{rendering, ProcessJobError};
use crate::storage::{
    handle_job_failure_at_path, JobFailureInput, METRIC_DEAD_LETTER_TOTAL,
    METRIC_RETRY_SCHEDULED_TOTAL,
};

pub(super) const API_SCOPE_TEMPLATE_READ: &str = "template:read";
pub(super) const API_SCOPE_PREVIEW_CREATE: &str = "preview:create";
pub(super) const API_SCOPE_PRINT_CREATE: &str = "print:create";
pub(super) const API_SCOPE_PRINTER_READ: &str = "printer:read";
pub(super) const API_SCOPE_JOB_READ: &str = "job:read";
pub(super) const MAX_TYPST_PACKAGE_ARCHIVE_BYTES: usize = 32 * 1024 * 1024;
pub(super) const MAX_TYPST_FONT_FILE_BYTES: usize = 20 * 1024 * 1024;
pub(super) const DEFAULT_JOBS_PAGE_SIZE: usize = 20;
pub(super) const MAX_JOBS_PAGE_SIZE: usize = 200;
pub(super) const DEFAULT_RECENT_JOBS_LIMIT: usize = 5;
pub(super) const MAX_RECENT_JOBS_LIMIT: usize = 50;
pub(super) const JOB_KIND_TEMPLATE: &str = "template";
pub(super) const JOB_KIND_DIRECT_FILE: &str = "direct_file";
pub(super) const ENV_TYPST_LOCAL_PACKAGES_ROOT: &str = "DEEPPRINT_TYPST_LOCAL_PACKAGES_ROOT";
pub(super) const ENV_TYPST_PREVIEW_CACHE_ROOT: &str = "DEEPPRINT_TYPST_PREVIEW_CACHE_ROOT";
pub(super) const ENV_TYPST_FONTS_ROOT: &str = "DEEPPRINT_TYPST_FONTS_ROOT";
pub(super) const ENV_TYPST_DEFAULT_FONTS_ROOT: &str = "DEEPPRINT_TYPST_DEFAULT_FONTS_ROOT";
pub(super) const SUPPORTED_TYPST_FONT_EXTENSIONS: [&str; 4] = ["ttf", "otf", "ttc", "otc"];
pub(super) const PREVIEW_HEADER_OUTPUT_KIND: &str = "x-deepprint-preview-output-kind";
pub(super) const PREVIEW_HEADER_PAGE_COUNT: &str = "x-deepprint-preview-page-count";
pub(super) const PREVIEW_HEADER_PAGE_WIDTH_PT: &str = "x-deepprint-preview-page-width-pt";
pub(super) const PREVIEW_HEADER_PAGE_HEIGHT_PT: &str = "x-deepprint-preview-page-height-pt";
pub(super) const PREVIEW_EXPOSE_HEADERS: &str = "x-deepprint-preview-output-kind,x-deepprint-preview-page-count,x-deepprint-preview-page-width-pt,x-deepprint-preview-page-height-pt,content-type";

pub(super) fn sha256_hex(payload: &[u8]) -> String {
    let digest = Sha256::digest(payload);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

pub(super) fn handle_job_failure(
    db_path: &Path,
    job_id: &str,
    err: &ProcessJobError,
    retry_max_attempts: u16,
    retry_backoff_base_sec: u64,
    retry_backoff_max_sec: u64,
) -> rusqlite::Result<()> {
    let result = handle_job_failure_at_path(
        db_path,
        JobFailureInput {
            job_id,
            error_code: err.code.as_str(),
            error_message: err.message.as_str(),
            retryable: err.retryable,
            retry_max_attempts,
            retry_backoff_base_sec,
            retry_backoff_max_sec,
            retry_metric_key: METRIC_RETRY_SCHEDULED_TOTAL,
            dead_letter_metric_key: METRIC_DEAD_LETTER_TOTAL,
        },
    )?;

    if let Some(job) = result.cleaned_direct_source_job.as_ref() {
        rendering::cleanup_direct_source_for_terminal_job(
            db_path,
            job,
            "job moved to failed terminal state",
        );
    }

    Ok(())
}
