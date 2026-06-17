use tracing::warn;

use super::super::super::super::AgentState;
use crate::storage::{
    cleanup_render_cache_by_disk_watermark, evict_render_cache_if_needed, try_insert_job_event,
};

pub(super) fn evict_render_cache_before_render(state: &AgentState, job_id: &str) {
    match evict_render_cache_if_needed(
        state.db_path.as_ref(),
        state.config.render_cache_ttl_sec,
        state.config.render_cache_max_entries,
    ) {
        Ok(evicted) if evicted > 0 => {
            let _ = try_insert_job_event(
                state.db_path.as_ref(),
                job_id,
                "render_cache_evict",
                Some("rendering"),
                Some("rendering"),
                &format!("evicted {evicted} render cache entries before render"),
            );
        }
        Ok(_) => {}
        Err(err) => warn!("render cache eviction failed for job {job_id}: {err}"),
    }
}

pub(super) fn evict_render_cache_after_store(state: &AgentState, job_id: &str) {
    match evict_render_cache_if_needed(
        state.db_path.as_ref(),
        state.config.render_cache_ttl_sec,
        state.config.render_cache_max_entries,
    ) {
        Ok(evicted) if evicted > 0 => {
            let _ = try_insert_job_event(
                state.db_path.as_ref(),
                job_id,
                "render_cache_evict",
                Some("rendering"),
                Some("rendering"),
                &format!("evicted {evicted} render cache entries after cache store"),
            );
        }
        Ok(_) => {}
        Err(err) => warn!("render cache post-store eviction failed for job {job_id}: {err}"),
    }
}

pub(super) fn cleanup_render_cache_disk_usage(state: &AgentState, job_id: &str) {
    match cleanup_render_cache_by_disk_watermark(
        state.db_path.as_ref(),
        state.config.render_cache_disk_high_watermark_bytes(),
        state.config.render_cache_disk_low_watermark_bytes(),
    ) {
        Ok(snapshot) if snapshot.stale_removed > 0 || snapshot.watermark_evicted > 0 => {
            let _ = try_insert_job_event(
                state.db_path.as_ref(),
                job_id,
                "render_cache_disk_cleanup",
                Some("rendering"),
                Some("rendering"),
                &format!(
                    "disk cleanup removed stale={}, evicted={}, usage_bytes={}",
                    snapshot.stale_removed, snapshot.watermark_evicted, snapshot.disk_usage_bytes
                ),
            );
        }
        Ok(_) => {}
        Err(err) => warn!("render cache disk cleanup failed for job {job_id}: {err}"),
    }
}
