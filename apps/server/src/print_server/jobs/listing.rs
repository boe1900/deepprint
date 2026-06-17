use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};

use super::super::{
    models::{
        ApiError, ApiResult, JOB_STATUS_CANCELED, JOB_STATUS_FAILED, JOB_STATUS_NEEDS_ATTENTION,
        JOB_STATUS_PRINTING, JOB_STATUS_QUEUED, JOB_STATUS_RENDERING, JOB_STATUS_SUBMITTING,
        JOB_STATUS_SUCCEEDED,
    },
    utils::normalize_pagination,
    AgentState, DEFAULT_JOBS_PAGE_SIZE, DEFAULT_RECENT_JOBS_LIMIT, MAX_JOBS_PAGE_SIZE,
    MAX_RECENT_JOBS_LIMIT,
};
use super::{
    detail::to_job_response, JobsListResponse, ListJobsQuery, ListRecentJobsQuery,
    RecentJobsResponse,
};
use crate::storage::{
    count_jobs_at_path, list_jobs_page_at_path, list_recent_jobs_records_at_path,
};

#[derive(Debug)]
struct ResolvedJobListFilters<'a> {
    statuses: Vec<&'a str>,
    defaulted_to_needs_attention: bool,
}

pub(super) async fn list_jobs(
    State(state): State<Arc<AgentState>>,
    Query(query): Query<ListJobsQuery>,
) -> ApiResult<Json<JobsListResponse>> {
    let resolved = resolve_job_list_filters(query.status.as_deref())?;
    let total = count_jobs_at_path(
        state.db_path.as_ref(),
        &resolved.statuses,
        query.printer_id.as_deref(),
        query.q.as_deref(),
    )?;
    let pagination = normalize_pagination(
        query.page,
        query.page_size,
        total,
        DEFAULT_JOBS_PAGE_SIZE,
        MAX_JOBS_PAGE_SIZE,
    );
    let jobs = list_jobs_page_at_path(
        state.db_path.as_ref(),
        &resolved.statuses,
        query.printer_id.as_deref(),
        query.q.as_deref(),
        pagination.start,
        pagination.page_size,
    )?
    .into_iter()
    .map(to_job_response)
    .collect();

    Ok(Json(JobsListResponse {
        jobs,
        page: pagination.page,
        page_size: pagination.page_size,
        total: pagination.total,
        total_pages: pagination.total_pages,
        status_filter: resolved
            .statuses
            .into_iter()
            .map(ToString::to_string)
            .collect(),
        printer_id: query
            .printer_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        q: query
            .q
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        defaulted_to_needs_attention: resolved.defaulted_to_needs_attention,
    }))
}

pub(super) async fn list_recent_jobs(
    State(state): State<Arc<AgentState>>,
    Query(query): Query<ListRecentJobsQuery>,
) -> ApiResult<Json<RecentJobsResponse>> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_RECENT_JOBS_LIMIT)
        .clamp(1, MAX_RECENT_JOBS_LIMIT);
    let jobs = list_recent_jobs_records_at_path(
        state.db_path.as_ref(),
        query.printer_id.as_deref(),
        limit,
    )?
    .into_iter()
    .map(to_job_response)
    .collect();

    Ok(Json(RecentJobsResponse {
        jobs,
        limit,
        printer_id: query
            .printer_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
    }))
}

fn resolve_job_list_filters(status: Option<&str>) -> ApiResult<ResolvedJobListFilters<'static>> {
    let Some(raw_status) = status.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(ResolvedJobListFilters {
            statuses: vec![JOB_STATUS_NEEDS_ATTENTION],
            defaulted_to_needs_attention: true,
        });
    };

    let mut statuses = Vec::new();
    for part in raw_status.split(',') {
        let normalized = part.trim();
        if normalized.is_empty() {
            continue;
        }
        let Some(canonical) = canonical_job_status(normalized) else {
            return Err(ApiError::bad_request(
                "INVALID_JOB_STATUS_FILTER",
                format!("unsupported job status filter: {normalized}"),
            ));
        };
        if !statuses.contains(&canonical) {
            statuses.push(canonical);
        }
    }

    if statuses.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_JOB_STATUS_FILTER",
            "status filter must not be empty".to_string(),
        ));
    }

    Ok(ResolvedJobListFilters {
        statuses,
        defaulted_to_needs_attention: false,
    })
}

fn canonical_job_status(status: &str) -> Option<&'static str> {
    match status {
        JOB_STATUS_QUEUED => Some(JOB_STATUS_QUEUED),
        JOB_STATUS_RENDERING => Some(JOB_STATUS_RENDERING),
        JOB_STATUS_SUBMITTING => Some(JOB_STATUS_SUBMITTING),
        JOB_STATUS_PRINTING => Some(JOB_STATUS_PRINTING),
        JOB_STATUS_NEEDS_ATTENTION => Some(JOB_STATUS_NEEDS_ATTENTION),
        JOB_STATUS_SUCCEEDED => Some(JOB_STATUS_SUCCEEDED),
        JOB_STATUS_FAILED => Some(JOB_STATUS_FAILED),
        JOB_STATUS_CANCELED => Some(JOB_STATUS_CANCELED),
        _ => None,
    }
}
