use std::time::Instant;

use tracing::{debug, warn};

use super::super::RenderCacheKey;
use super::keys::to_storage_render_cache_key;
use crate::{
    print_server::{rendering, AgentState, ApiResult},
    renderer::{self, RenderRequest, RenderResult},
    storage::{
        evict_render_cache_if_needed, increment_agent_metric, try_load_render_cache,
        try_upsert_render_cache, METRIC_RENDER_CACHE_HIT_TOTAL, METRIC_RENDER_CACHE_MISS_TOTAL,
    },
};

pub(super) async fn load_or_render_preview(
    state: &AgentState,
    render_request: &RenderRequest,
    cache_key: &RenderCacheKey,
    preview_started: Instant,
) -> ApiResult<RenderResult> {
    match try_load_render_cache(
        state.db_path.as_ref(),
        &to_storage_render_cache_key(cache_key),
        &render_request.job_id,
    ) {
        Ok(Some(cached)) => {
            let _ =
                increment_agent_metric(state.db_path.as_ref(), METRIC_RENDER_CACHE_HIT_TOTAL, 1);
            debug!(
                job_id = %render_request.job_id,
                elapsed_ms = preview_started.elapsed().as_millis(),
                "typst preview cache hit"
            );
            Ok(cached)
        }
        Ok(None) => {
            render_preview_cache_miss(state, render_request, cache_key, preview_started).await
        }
        Err(err) => {
            warn!("preview render cache lookup failed: {err}");
            renderer::render_preview_in_process(render_request.clone())
                .await
                .map_err(rendering::map_preview_render_error)
        }
    }
}

async fn render_preview_cache_miss(
    state: &AgentState,
    render_request: &RenderRequest,
    cache_key: &RenderCacheKey,
    preview_started: Instant,
) -> ApiResult<RenderResult> {
    let _ = increment_agent_metric(state.db_path.as_ref(), METRIC_RENDER_CACHE_MISS_TOTAL, 1);

    let rendered = renderer::render_preview_in_process(render_request.clone())
        .await
        .map_err(rendering::map_preview_render_error)?;
    debug!(
        job_id = %render_request.job_id,
        elapsed_ms = preview_started.elapsed().as_millis(),
        "typst preview rendered in-process"
    );

    if let Err(err) = try_upsert_render_cache(
        state.db_path.as_ref(),
        &to_storage_render_cache_key(cache_key),
        &rendered,
    ) {
        warn!(
            "unable to update preview render cache for {}: {err}",
            render_request.job_id
        );
    }

    if let Err(err) = evict_render_cache_if_needed(
        state.db_path.as_ref(),
        state.config.render_cache_ttl_sec,
        state.config.render_cache_max_entries,
    ) {
        warn!("preview render cache eviction failed: {err}");
    }

    Ok(rendered)
}
