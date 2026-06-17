mod cancel;
mod events;
mod failure;
mod models;
mod queries;
mod rows;
mod time;
mod transitions;
mod worker;
mod writes;

pub use cancel::{cancel_needs_attention_job, cancel_printing_job, cancel_queued_job};
pub use events::try_insert_job_event;
pub use failure::handle_job_failure_at_path;
pub use models::{
    DirectJobInsertInput, JobFailureInput, JobRecord, RenderArtifactJobUpdateInput,
    TemplateJobInsertInput,
};
pub use queries::{
    count_active_jobs_by_artifact_path, count_inflight_jobs_for_printer_at_path, count_jobs_at_path,
    fetch_job_by_id_at_path, fetch_job_by_request_id_at_path, list_jobs_page_at_path,
    list_recent_jobs_records_at_path, load_failed_jobs_snapshot_at_path, probe_database_health_at_path,
};
#[cfg(test)]
pub use queries::{fetch_job_by_id, load_failed_jobs_snapshot};
pub use transitions::{
    move_printing_job_to_attention, move_submitting_job_to_attention, record_backend_poll_result,
    record_backend_unknown, save_backend_submission, save_reconciled_backend_submission,
    transition_job_status,
};
pub use worker::recover_inflight_jobs;
pub use worker::{
    claim_next_job, list_printing_jobs_for_monitor, list_submitting_jobs_for_monitor,
};
pub use writes::{
    insert_direct_job_at_path, insert_template_job_at_path, save_render_artifact_result,
};
