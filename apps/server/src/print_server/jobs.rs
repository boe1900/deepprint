use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, Query, State},
    Json,
};

#[path = "jobs/cancel.rs"]
mod cancel;
#[path = "jobs/detail.rs"]
mod detail;
#[path = "jobs/listing.rs"]
mod listing;

use super::{AgentState, ApiResult, JobRecord};

#[derive(Debug, Deserialize, Default)]
pub(super) struct ListJobsQuery {
    pub(super) page: Option<usize>,
    pub(super) page_size: Option<usize>,
    #[serde(default)]
    pub(super) status: Option<String>,
    #[serde(default)]
    pub(super) printer_id: Option<String>,
    #[serde(default)]
    pub(super) q: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct ListRecentJobsQuery {
    pub(super) limit: Option<usize>,
    #[serde(default)]
    pub(super) printer_id: Option<String>,
}

#[derive(Serialize)]
pub(super) struct CancelResponse {
    pub(super) job_id: String,
    pub(super) status: String,
}

#[derive(Debug, Serialize)]
pub(super) struct JobResponse {
    pub(super) job_id: String,
    request_id: String,
    job_kind: String,
    printer_id: Option<String>,
    printer_name_snapshot: Option<String>,
    printer_uri: Option<String>,
    status: String,
    attempt_count: i64,
    created_at: i64,
    updated_at: i64,
    last_error_code: Option<String>,
    last_error_message: Option<String>,
    render_artifact_path: Option<String>,
    render_output_kind: Option<String>,
    render_page_count: Option<i64>,
    render_page_width_pt: Option<f64>,
    render_page_height_pt: Option<f64>,
    backend_name: Option<String>,
    backend_job_ref_json: Option<String>,
    submit_started_at: Option<i64>,
    submitted_at: Option<i64>,
    last_polled_at: Option<i64>,
    backend_state: Option<String>,
    backend_state_message: Option<String>,
    unknown_since_at: Option<i64>,
    needs_attention_reason: Option<String>,
    source_file_name: Option<String>,
    source_content_type: Option<String>,
    source_file_size_bytes: Option<i64>,
    data: Value,
    print_options: Value,
}

#[derive(Debug, Serialize)]
pub(super) struct JobsListResponse {
    pub(super) jobs: Vec<JobResponse>,
    pub(super) page: usize,
    pub(super) page_size: usize,
    pub(super) total: usize,
    pub(super) total_pages: usize,
    pub(super) status_filter: Vec<String>,
    pub(super) printer_id: Option<String>,
    pub(super) q: Option<String>,
    pub(super) defaulted_to_needs_attention: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct RecentJobsResponse {
    pub(super) jobs: Vec<JobResponse>,
    pub(super) limit: usize,
    pub(super) printer_id: Option<String>,
}

pub(super) async fn list_jobs(
    state: State<Arc<AgentState>>,
    query: Query<ListJobsQuery>,
) -> ApiResult<Json<JobsListResponse>> {
    listing::list_jobs(state, query).await
}

pub(super) async fn list_recent_jobs(
    state: State<Arc<AgentState>>,
    query: Query<ListRecentJobsQuery>,
) -> ApiResult<Json<RecentJobsResponse>> {
    listing::list_recent_jobs(state, query).await
}

pub(super) async fn get_job(
    state: State<Arc<AgentState>>,
    job_id: AxumPath<String>,
) -> ApiResult<Json<JobResponse>> {
    detail::get_job(state, job_id).await
}

pub(super) fn job_response_from_record(job: JobRecord) -> JobResponse {
    detail::to_job_response(job)
}

pub(super) async fn cancel_job(
    state: State<Arc<AgentState>>,
    job_id: AxumPath<String>,
) -> ApiResult<Json<CancelResponse>> {
    cancel::cancel_job(state, job_id).await
}
