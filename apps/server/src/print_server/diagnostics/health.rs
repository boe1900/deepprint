use std::{path::Path, sync::Arc};

use axum::{extract::State, Json};

use super::{AgentState, HealthResponse};
use crate::print_server::diagnostic_fs::load_log_usage_snapshot;
use crate::print_server::models::LogUsageSnapshot;
use crate::storage::{load_cache_metrics_snapshot, load_queue_metrics_snapshot};
use crate::storage::{CacheMetricsSnapshot, QueueMetricsSnapshot};

pub(super) async fn health(State(state): State<Arc<AgentState>>) -> Json<HealthResponse> {
    let cache_metrics = match load_cache_metrics_snapshot(state.db_path.as_ref()) {
        Ok(metrics) => metrics,
        Err(err) => {
            tracing::warn!("failed to load cache metrics for health endpoint: {err}");
            CacheMetricsSnapshot::default()
        }
    };
    let queue_metrics = match load_queue_metrics_snapshot(state.db_path.as_ref()) {
        Ok(metrics) => metrics,
        Err(err) => {
            tracing::warn!("failed to load queue metrics for health endpoint: {err}");
            QueueMetricsSnapshot::default()
        }
    };
    let log_usage = match load_log_usage_snapshot(
        Path::new(&state.config.log_dir),
        &state.config.log_file_prefix,
    ) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            tracing::warn!("failed to load log usage for health endpoint: {err}");
            LogUsageSnapshot::default()
        }
    };

    let hit_ratio = if cache_metrics.hit_total + cache_metrics.miss_total > 0 {
        cache_metrics.hit_total as f64 / (cache_metrics.hit_total + cache_metrics.miss_total) as f64
    } else {
        0.0
    };

    Json(HealthResponse {
        status: "ok",
        version: state.version.clone(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
        database_driver: state.database_target.driver_name(),
        mock_mode: state.config.mock_mode,
        cups_base_url: state.current_cups_base_url(),
        worker_concurrency: state.config.worker_concurrency,
        retry_max_attempts: state.config.retry_max_attempts,
        retry_backoff_base_sec: state.config.retry_backoff_base_sec,
        retry_backoff_max_sec: state.config.retry_backoff_max_sec,
        backend_name: state.backend.backend_name().to_string(),
        render_engine: state.config.render_engine.clone(),
        render_timeout_sec: state.config.render_timeout_sec,
        render_cache_ttl_sec: state.config.render_cache_ttl_sec,
        render_cache_max_entries: state.config.render_cache_max_entries,
        render_cache_entries: cache_metrics.entries,
        render_cache_disk_usage_bytes: cache_metrics.disk_usage_bytes,
        render_cache_disk_high_watermark_mb: state.config.render_cache_disk_high_watermark_mb,
        render_cache_disk_low_watermark_mb: state.config.render_cache_disk_low_watermark_mb,
        render_cache_cleanup_interval_sec: state.config.render_cache_cleanup_interval_sec,
        render_cache_hit_total: cache_metrics.hit_total,
        render_cache_miss_total: cache_metrics.miss_total,
        render_cache_evict_total: cache_metrics.evict_total,
        render_cache_disk_cleanup_total: cache_metrics.disk_cleanup_total,
        render_cache_hit_ratio: hit_ratio,
        retry_scheduled_total: cache_metrics.retry_scheduled_total,
        dead_letter_total: cache_metrics.dead_letter_total,
        dead_letter_count: cache_metrics.dead_letter_count,
        queue_length: queue_metrics.queued_count,
        rendering_jobs: queue_metrics.rendering_count,
        submitting_jobs: queue_metrics.submitting_count,
        printing_jobs: queue_metrics.printing_count,
        needs_attention_jobs: queue_metrics.needs_attention_count,
        succeeded_total: queue_metrics.succeeded_count,
        failed_total: queue_metrics.failed_count,
        canceled_total: queue_metrics.canceled_count,
        terminal_total: queue_metrics.terminal_total,
        success_rate: queue_metrics.success_rate,
        failure_rate: queue_metrics.failure_rate,
        avg_succeeded_duration_sec: queue_metrics.avg_succeeded_duration_sec,
        log_dir: state.config.log_dir.clone(),
        log_file_prefix: state.config.log_file_prefix.clone(),
        log_max_files: state.config.log_max_files,
        log_max_total_mb: state.config.log_max_total_mb,
        log_cleanup_interval_sec: state.config.log_cleanup_interval_sec,
        log_files_count: log_usage.files_count,
        log_disk_usage_bytes: log_usage.disk_usage_bytes,
        log_cleanup_total: cache_metrics.log_cleanup_total,
        diagnostics_dir: state.config.diagnostics_dir.clone(),
        diagnostics_max_files: state.config.diagnostics_max_files,
        typst_local_packages_root: state
            .typst_local_packages_root
            .to_string_lossy()
            .to_string(),
        typst_preview_cache_root: state.typst_preview_cache_root.to_string_lossy().to_string(),
        typst_fonts_root: state.typst_fonts_root.to_string_lossy().to_string(),
        direct_job_max_bytes: state.config.direct_job_max_bytes,
        backend_status_poll_ms: state.config.backend_status_poll_ms,
        backend_status_timeout_sec: state.config.backend_status_timeout_sec,
    })
}
