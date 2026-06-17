use serde::{Deserialize, Serialize};

use super::super::models::{HealthComponentProbe, LogUsageSnapshot};

#[derive(Serialize)]
pub(crate) struct HealthResponse {
    pub(crate) status: &'static str,
    pub(crate) version: String,
    pub(crate) uptime_seconds: u64,
    pub(crate) database_driver: &'static str,
    pub(crate) mock_mode: bool,
    pub(crate) cups_base_url: String,
    pub(crate) worker_concurrency: u16,
    pub(crate) retry_max_attempts: u16,
    pub(crate) retry_backoff_base_sec: u64,
    pub(crate) retry_backoff_max_sec: u64,
    pub(crate) backend_name: String,
    pub(crate) render_engine: String,
    pub(crate) render_timeout_sec: u64,
    pub(crate) render_cache_ttl_sec: u64,
    pub(crate) render_cache_max_entries: u64,
    pub(crate) render_cache_entries: i64,
    pub(crate) render_cache_disk_usage_bytes: i64,
    pub(crate) render_cache_disk_high_watermark_mb: u64,
    pub(crate) render_cache_disk_low_watermark_mb: u64,
    pub(crate) render_cache_cleanup_interval_sec: u64,
    pub(crate) render_cache_hit_total: i64,
    pub(crate) render_cache_miss_total: i64,
    pub(crate) render_cache_evict_total: i64,
    pub(crate) render_cache_disk_cleanup_total: i64,
    pub(crate) render_cache_hit_ratio: f64,
    pub(crate) retry_scheduled_total: i64,
    pub(crate) dead_letter_total: i64,
    pub(crate) dead_letter_count: i64,
    pub(crate) queue_length: i64,
    pub(crate) rendering_jobs: i64,
    pub(crate) submitting_jobs: i64,
    pub(crate) printing_jobs: i64,
    pub(crate) needs_attention_jobs: i64,
    pub(crate) succeeded_total: i64,
    pub(crate) failed_total: i64,
    pub(crate) canceled_total: i64,
    pub(crate) terminal_total: i64,
    pub(crate) success_rate: f64,
    pub(crate) failure_rate: f64,
    pub(crate) avg_succeeded_duration_sec: f64,
    pub(crate) log_dir: String,
    pub(crate) log_file_prefix: String,
    pub(crate) log_max_files: u64,
    pub(crate) log_max_total_mb: u64,
    pub(crate) log_cleanup_interval_sec: u64,
    pub(crate) log_files_count: i64,
    pub(crate) log_disk_usage_bytes: i64,
    pub(crate) log_cleanup_total: i64,
    pub(crate) diagnostics_dir: String,
    pub(crate) diagnostics_max_files: u64,
    pub(crate) typst_local_packages_root: String,
    pub(crate) typst_preview_cache_root: String,
    pub(crate) typst_fonts_root: String,
    pub(crate) direct_job_max_bytes: u64,
    pub(crate) backend_status_poll_ms: u64,
    pub(crate) backend_status_timeout_sec: u64,
}

#[derive(Serialize)]
pub(crate) struct DeepHealthResponse {
    pub(crate) status: &'static str,
    pub(crate) version: String,
    pub(crate) uptime_seconds: u64,
    pub(crate) database_driver: &'static str,
    pub(crate) overall_ok: bool,
    pub(crate) db: HealthComponentProbe,
    pub(crate) backend: HealthComponentProbe,
    pub(crate) renderer_subprocess: HealthComponentProbe,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub(crate) struct DiagnosticExportRequest {
    pub(crate) include_logs: bool,
    pub(crate) log_max_files: u64,
    pub(crate) log_tail_lines: u64,
    pub(crate) log_max_bytes_per_file: u64,
    pub(crate) failed_jobs_limit: u64,
}

impl Default for DiagnosticExportRequest {
    fn default() -> Self {
        Self {
            include_logs: true,
            log_max_files: 5,
            log_tail_lines: 3000,
            log_max_bytes_per_file: 512 * 1024,
            failed_jobs_limit: 200,
        }
    }
}

impl DiagnosticExportRequest {
    pub(crate) fn normalized(self) -> Self {
        Self {
            include_logs: self.include_logs,
            log_max_files: self.log_max_files.min(50),
            log_tail_lines: self.log_tail_lines.clamp(100, 20_000),
            log_max_bytes_per_file: self
                .log_max_bytes_per_file
                .clamp(64 * 1024, 4 * 1024 * 1024),
            failed_jobs_limit: self.failed_jobs_limit.clamp(10, 5000),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct DiagnosticExportResponse {
    pub(crate) bundle_id: String,
    pub(crate) bundle_path: String,
    pub(crate) size_bytes: i64,
    pub(crate) created_at: i64,
    pub(crate) failed_jobs_count: usize,
    pub(crate) included_log_files: usize,
}

#[derive(Serialize)]
pub(crate) struct DiagnosticManifest {
    pub(crate) bundle_id: String,
    pub(crate) created_at: i64,
    pub(crate) version: String,
    pub(crate) uptime_seconds: u64,
    pub(crate) platform: String,
    pub(crate) arch: String,
    pub(crate) config: DiagnosticConfigSnapshot,
}

#[derive(Serialize)]
pub(crate) struct DiagnosticConfigSnapshot {
    pub(crate) bind_addr: String,
    pub(crate) port: u16,
    pub(crate) cups_base_url: String,
    pub(crate) mock_mode: bool,
    pub(crate) worker_concurrency: u16,
    pub(crate) render_engine: String,
    pub(crate) render_timeout_sec: u64,
    pub(crate) backend_status_poll_ms: u64,
    pub(crate) backend_status_timeout_sec: u64,
    pub(crate) log_dir: String,
    pub(crate) log_file_prefix: String,
    pub(crate) direct_job_max_bytes: u64,
}

#[derive(Serialize)]
pub(crate) struct DiagnosticHealthSnapshot {
    pub(crate) cache: crate::storage::CacheMetricsSnapshot,
    pub(crate) queue: crate::storage::QueueMetricsSnapshot,
    pub(crate) log_usage: LogUsageSnapshot,
}
