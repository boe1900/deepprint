use serde::Serialize;

#[derive(Debug)]
pub struct JobRecord {
    pub id: String,
    pub request_id: String,
    pub job_kind: String,
    pub printer_id: Option<String>,
    pub printer_name_snapshot: Option<String>,
    pub printer_uri: Option<String>,
    pub template_content: String,
    pub status: String,
    pub attempt_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    pub render_artifact_path: Option<String>,
    pub render_output_kind: Option<String>,
    pub render_page_count: Option<i64>,
    pub render_page_width_pt: Option<f64>,
    pub render_page_height_pt: Option<f64>,
    pub data_json: String,
    pub print_options_json: String,
    pub backend_name: Option<String>,
    pub backend_job_ref_json: Option<String>,
    pub submit_started_at: Option<i64>,
    pub submitted_at: Option<i64>,
    pub last_polled_at: Option<i64>,
    pub backend_state: Option<String>,
    pub backend_state_message: Option<String>,
    pub unknown_since_at: Option<i64>,
    pub needs_attention_reason: Option<String>,
    pub source_file_path: Option<String>,
    pub source_file_name: Option<String>,
    pub source_content_type: Option<String>,
    pub source_file_size_bytes: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticFailedJobSnapshot {
    pub job_id: String,
    pub request_id: String,
    pub status: String,
    pub attempt_count: i64,
    pub updated_at: i64,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    pub backend_name: Option<String>,
    pub backend_job_ref_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SubmittingMonitorJobRecord {
    pub id: String,
    pub printer_uri: Option<String>,
    pub submit_started_at: Option<i64>,
    pub backend_job_ref_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PrintingMonitorJobRecord {
    pub id: String,
    pub backend_job_ref_json: Option<String>,
    pub unknown_since_at: Option<i64>,
    pub job_kind: String,
    pub source_file_path: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InflightRecoverySummary {
    pub rendering_requeued: usize,
    pub printing_requeued: usize,
}

#[derive(Debug)]
pub struct JobFailureHandlingResult {
    pub cleaned_direct_source_job: Option<JobRecord>,
}

pub struct TemplateJobInsertInput<'a> {
    pub id: &'a str,
    pub request_id: &'a str,
    pub printer_id: &'a str,
    pub printer_name_snapshot: &'a str,
    pub printer_uri: &'a str,
    pub template_content: &'a str,
    pub data_json: &'a str,
    pub print_options_json: &'a str,
    pub created_at: i64,
}

pub struct DirectJobInsertInput<'a> {
    pub id: &'a str,
    pub request_id: &'a str,
    pub printer_id: &'a str,
    pub printer_name_snapshot: &'a str,
    pub printer_uri: &'a str,
    pub data_json: &'a str,
    pub print_options_json: &'a str,
    pub source_file_path: &'a str,
    pub source_file_name: &'a str,
    pub source_content_type: Option<&'a str>,
    pub source_file_size_bytes: i64,
    pub created_at: i64,
}

pub struct RenderArtifactJobUpdateInput<'a> {
    pub artifact_path: &'a str,
    pub output_kind: &'a str,
    pub page_count: i64,
    pub page_width_pt: Option<f64>,
    pub page_height_pt: Option<f64>,
}

pub struct JobFailureInput<'a> {
    pub job_id: &'a str,
    pub error_code: &'a str,
    pub error_message: &'a str,
    pub retryable: bool,
    pub retry_max_attempts: u16,
    pub retry_backoff_base_sec: u64,
    pub retry_backoff_max_sec: u64,
    pub retry_metric_key: &'a str,
    pub dead_letter_metric_key: &'a str,
}
