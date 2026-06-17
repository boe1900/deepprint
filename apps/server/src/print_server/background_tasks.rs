use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use tracing::{info, warn};

use super::{
    diagnostic_fs::apply_log_retention, increment_agent_metric, mb_to_bytes, worker, AgentState,
    METRIC_LOG_CLEANUP_TOTAL,
};
use crate::{renderer, storage::cleanup_render_cache_by_disk_watermark};

pub(super) fn spawn_background_tasks(state: Arc<AgentState>) {
    for worker_index in 0..state.config.worker_concurrency {
        let worker_state = state.clone();
        tokio::spawn(async move {
            worker_loop(worker_state, worker_index).await;
        });
    }

    let monitor_state = state.clone();
    tokio::spawn(async move {
        backend_monitor_loop(monitor_state).await;
    });

    let cleanup_state = state.clone();
    tokio::spawn(async move {
        render_cache_cleanup_loop(cleanup_state).await;
    });

    let log_cleanup_state = state.clone();
    tokio::spawn(async move {
        log_cleanup_loop(log_cleanup_state).await;
    });

    let warmup_state = state;
    tokio::spawn(async move {
        warmup_preview_renderer_task(warmup_state).await;
    });
}

async fn worker_loop(state: Arc<AgentState>, worker_index: u16) {
    worker::worker_loop(state, worker_index).await;
}

async fn backend_monitor_loop(state: Arc<AgentState>) {
    let interval = Duration::from_millis(state.config.backend_status_poll_ms.max(200));
    info!(
        "backend monitor loop started, poll_ms={}, submission_recovery_timeout_sec={}, unknown_to_attention_sec={}",
        interval.as_millis(),
        state.config.submission_recovery_timeout_sec,
        state.config.backend_unknown_to_attention_sec,
    );

    loop {
        if let Err(err) = monitor_submitting_jobs(state.as_ref()).await {
            warn!("submission recovery loop failed: {err}");
        }
        if let Err(err) = monitor_printing_jobs(state.as_ref()).await {
            warn!("printing monitor loop failed: {err}");
        }
        tokio::time::sleep(interval).await;
    }
}

async fn render_cache_cleanup_loop(state: Arc<AgentState>) {
    let interval = Duration::from_secs(state.config.render_cache_cleanup_interval_sec.max(10));
    info!(
        "render cache cleanup loop started, interval_sec={}, high_mb={}, low_mb={}",
        interval.as_secs(),
        state.config.render_cache_disk_high_watermark_mb,
        state.config.render_cache_disk_low_watermark_mb,
    );

    loop {
        match cleanup_render_cache_by_disk_watermark(
            state.db_path.as_ref(),
            state.config.render_cache_disk_high_watermark_bytes(),
            state.config.render_cache_disk_low_watermark_bytes(),
        ) {
            Ok(snapshot) if snapshot.stale_removed > 0 || snapshot.watermark_evicted > 0 => {
                info!(
                    "render cache cleanup applied: stale_removed={}, watermark_evicted={}, disk_usage_bytes={}",
                    snapshot.stale_removed,
                    snapshot.watermark_evicted,
                    snapshot.disk_usage_bytes,
                );
            }
            Ok(_) => {}
            Err(err) => warn!("render cache cleanup loop failed: {err}"),
        }

        tokio::time::sleep(interval).await;
    }
}

async fn log_cleanup_loop(state: Arc<AgentState>) {
    let interval = Duration::from_secs(state.config.log_cleanup_interval_sec.max(30));
    info!(
        "log cleanup loop started, interval_sec={}, dir={}, max_files={}, max_total_mb={}",
        interval.as_secs(),
        state.config.log_dir,
        state.config.log_max_files,
        state.config.log_max_total_mb,
    );

    loop {
        match apply_log_retention(
            Path::new(&state.config.log_dir),
            &state.config.log_file_prefix,
            state.config.log_max_files,
            mb_to_bytes(state.config.log_max_total_mb),
        ) {
            Ok(snapshot) if snapshot.removed_files > 0 => {
                info!(
                    "log retention applied: removed_files={}, remaining_files={}, remaining_bytes={}",
                    snapshot.removed_files,
                    snapshot.files_count,
                    snapshot.disk_usage_bytes
                );
                let _ = increment_agent_metric(
                    state.db_path.as_ref(),
                    METRIC_LOG_CLEANUP_TOTAL,
                    snapshot.removed_files,
                );
            }
            Ok(_) => {}
            Err(err) => warn!("log cleanup loop failed: {err}"),
        }

        tokio::time::sleep(interval).await;
    }
}

async fn warmup_preview_renderer_task(state: Arc<AgentState>) {
    let started = Instant::now();
    match renderer::warmup_preview_renderer().await {
        Ok(()) => info!(
            elapsed_ms = started.elapsed().as_millis(),
            bind_addr = %state.config.bind_addr,
            port = state.config.port,
            "typst preview renderer warmup completed"
        ),
        Err(err) => warn!(
            elapsed_ms = started.elapsed().as_millis(),
            bind_addr = %state.config.bind_addr,
            port = state.config.port,
            "typst preview renderer warmup failed: {err}"
        ),
    }
}

pub(super) async fn monitor_submitting_jobs(state: &AgentState) -> rusqlite::Result<()> {
    worker::monitor_submitting_jobs(state).await
}

pub(super) async fn monitor_printing_jobs(state: &AgentState) -> rusqlite::Result<()> {
    worker::monitor_printing_jobs(state).await
}
