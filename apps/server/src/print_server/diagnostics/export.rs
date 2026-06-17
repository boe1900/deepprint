use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{extract::State, Json};
use uuid::Uuid;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use super::{
    DiagnosticConfigSnapshot, DiagnosticExportRequest, DiagnosticExportResponse,
    DiagnosticHealthSnapshot, DiagnosticManifest,
};
use crate::print_server::{
    diagnostic_fs::{
        cleanup_old_diagnostic_bundles, collect_recent_log_tails, load_log_usage_snapshot,
        sanitize_zip_entry_name, write_zip_json, write_zip_text,
    },
    utils::{clamp_u64_to_i64, now_unix},
    AgentState, ApiError, ApiResult,
};
use crate::storage::{
    load_cache_metrics_snapshot, load_failed_jobs_snapshot_at_path, load_queue_metrics_snapshot,
};

pub(super) async fn export_diagnostics(
    State(state): State<Arc<AgentState>>,
    payload: Option<Json<DiagnosticExportRequest>>,
) -> ApiResult<Json<DiagnosticExportResponse>> {
    let request = payload.map(|it| it.0).unwrap_or_default().normalized();
    let state_clone = state.clone();
    let exported = tokio::task::spawn_blocking(move || {
        create_diagnostic_bundle(state_clone.as_ref(), request)
    })
    .await
    .map_err(|err| ApiError::Internal(format!("diagnostics export join error: {err}")))??;

    Ok(Json(exported))
}

fn create_diagnostic_bundle(
    state: &AgentState,
    request: DiagnosticExportRequest,
) -> Result<DiagnosticExportResponse, ApiError> {
    let created_at = now_unix();
    let bundle_id = format!("diag-{}-{}", created_at, Uuid::new_v4());
    let diagnostics_dir = PathBuf::from(&state.config.diagnostics_dir);
    std::fs::create_dir_all(&diagnostics_dir)
        .map_err(|err| ApiError::Internal(format!("create diagnostics dir failed: {err}")))?;
    let bundle_path = diagnostics_dir.join(format!("{bundle_id}.zip"));

    let cache_metrics = load_cache_metrics_snapshot(state.db_path.as_ref())?;
    let queue_metrics = load_queue_metrics_snapshot(state.db_path.as_ref())?;
    let log_usage = load_log_usage_snapshot(
        Path::new(&state.config.log_dir),
        &state.config.log_file_prefix,
    )
    .unwrap_or_default();
    let failed_jobs =
        load_failed_jobs_snapshot_at_path(state.db_path.as_ref(), request.failed_jobs_limit)?;

    let manifest = DiagnosticManifest {
        bundle_id: bundle_id.clone(),
        created_at,
        version: state.version.clone(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        config: build_diagnostic_config_snapshot(state),
    };

    let health_snapshot = DiagnosticHealthSnapshot {
        cache: cache_metrics,
        queue: queue_metrics,
        log_usage,
    };

    let log_tails = if request.include_logs {
        collect_recent_log_tails(
            Path::new(&state.config.log_dir),
            &state.config.log_file_prefix,
            request.log_max_files,
            request.log_tail_lines,
            request.log_max_bytes_per_file,
        )
        .map_err(|err| ApiError::Internal(format!("collect recent logs failed: {err}")))?
    } else {
        vec![]
    };

    let file = std::fs::File::create(&bundle_path)
        .map_err(|err| ApiError::Internal(format!("create diagnostics bundle failed: {err}")))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o600);

    write_zip_json(&mut zip, "manifest.json", &manifest, options)?;
    write_zip_json(&mut zip, "health.json", &health_snapshot, options)?;
    write_zip_json(&mut zip, "failed_jobs.json", &failed_jobs, options)?;

    for (file_name, content) in &log_tails {
        let normalized = sanitize_zip_entry_name(file_name);
        let path = format!("logs/{normalized}");
        write_zip_text(&mut zip, &path, content, options)?;
    }

    zip.finish()
        .map_err(|err| ApiError::Internal(format!("finalize diagnostics bundle failed: {err}")))?;
    let size_bytes = std::fs::metadata(&bundle_path)
        .map(|metadata| clamp_u64_to_i64(metadata.len()))
        .unwrap_or(0);

    cleanup_old_diagnostic_bundles(&diagnostics_dir, state.config.diagnostics_max_files).map_err(
        |err| ApiError::Internal(format!("cleanup old diagnostics bundles failed: {err}")),
    )?;

    Ok(DiagnosticExportResponse {
        bundle_id,
        bundle_path: bundle_path.to_string_lossy().to_string(),
        size_bytes,
        created_at,
        failed_jobs_count: failed_jobs.len(),
        included_log_files: log_tails.len(),
    })
}

fn build_diagnostic_config_snapshot(state: &AgentState) -> DiagnosticConfigSnapshot {
    DiagnosticConfigSnapshot {
        bind_addr: state.config.bind_addr.clone(),
        port: state.config.port,
        cups_base_url: state.current_cups_base_url(),
        mock_mode: state.config.mock_mode,
        worker_concurrency: state.config.worker_concurrency,
        render_engine: state.config.render_engine.clone(),
        render_timeout_sec: state.config.render_timeout_sec,
        backend_status_poll_ms: state.config.backend_status_poll_ms,
        backend_status_timeout_sec: state.config.backend_status_timeout_sec,
        log_dir: state.config.log_dir.clone(),
        log_file_prefix: state.config.log_file_prefix.clone(),
        direct_job_max_bytes: state.config.direct_job_max_bytes,
    }
}
