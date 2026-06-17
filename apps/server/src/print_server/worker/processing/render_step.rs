#[path = "render_step/cache_maintenance.rs"]
mod cache_maintenance;
#[path = "render_step/events.rs"]
mod events;

use tracing::warn;

use super::super::super::{
    models::ProcessJobError,
    rendering::{build_direct_file_render_result, render_job_artifact_via_subprocess},
    shared::{JOB_KIND_DIRECT_FILE, JOB_KIND_TEMPLATE},
    submission::{build_render_cache_key, to_storage_render_cache_key, RenderCacheKey},
    AgentState, JobRecord,
};
use crate::printer::PrintOptions;
use crate::renderer::RenderResult;
use crate::storage::{
    increment_agent_metric, try_load_render_cache, try_upsert_render_cache,
    METRIC_RENDER_CACHE_MISS_TOTAL,
};

pub(super) async fn render_job_artifact(
    state: &AgentState,
    job: &JobRecord,
    print_options: &PrintOptions,
) -> Result<RenderResult, ProcessJobError> {
    if job.job_kind == JOB_KIND_DIRECT_FILE {
        events::record_direct_file_ready(state, &job.id);
        return build_direct_file_render_result(job, print_options);
    }

    if job.job_kind != JOB_KIND_TEMPLATE {
        return Err(ProcessJobError::new(
            "JOB_KIND_INVALID",
            format!("unsupported job_kind={}", job.job_kind),
        ));
    }

    render_template_job(state, job, print_options).await
}

async fn render_template_job(
    state: &AgentState,
    job: &JobRecord,
    print_options: &PrintOptions,
) -> Result<RenderResult, ProcessJobError> {
    cache_maintenance::evict_render_cache_before_render(state, &job.id);

    let cache_key = build_render_cache_key(job);
    match try_load_render_cache(
        state.db_path.as_ref(),
        &to_storage_render_cache_key(&cache_key),
        &job.id,
    ) {
        Ok(Some(cached)) => {
            events::record_render_cache_hit(state, &job.id);
            Ok(cached)
        }
        Ok(None) => render_template_cache_miss(state, job, print_options, &cache_key).await,
        Err(err) => {
            let _ =
                increment_agent_metric(state.db_path.as_ref(), METRIC_RENDER_CACHE_MISS_TOTAL, 1);
            warn!("render cache lookup failed for job {}: {err}", job.id);
            render_job_artifact_via_subprocess(job, print_options, state.config.render_timeout_sec)
                .await
        }
    }
}

async fn render_template_cache_miss(
    state: &AgentState,
    job: &JobRecord,
    print_options: &PrintOptions,
    cache_key: &RenderCacheKey,
) -> Result<RenderResult, ProcessJobError> {
    events::record_render_cache_miss(state, &job.id);
    let rendered =
        render_job_artifact_via_subprocess(job, print_options, state.config.render_timeout_sec)
            .await?;

    if let Err(err) = try_upsert_render_cache(
        state.db_path.as_ref(),
        &to_storage_render_cache_key(cache_key),
        &rendered,
    ) {
        warn!("unable to update render cache for job {}: {err}", job.id);
        return Ok(rendered);
    }

    cache_maintenance::evict_render_cache_after_store(state, &job.id);
    cache_maintenance::cleanup_render_cache_disk_usage(state, &job.id);
    Ok(rendered)
}
