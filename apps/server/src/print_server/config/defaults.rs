use super::AgentConfig;
use crate::print_server::platform_paths::{default_diagnostics_dir, default_log_dir};

pub(super) const DEFAULT_AUTH_SESSION_COOKIE_NAME: &str = "deepprint_session";
pub(super) const DEFAULT_AUTH_SESSION_TTL_SEC: u64 = 7 * 24 * 60 * 60;
pub(super) const DEFAULT_EXTERNAL_CUPS_BASE_URL: &str = "http://127.0.0.1:631/";
pub(super) const DEFAULT_DIRECT_JOB_MAX_BYTES: u64 = 25 * 1024 * 1024;

pub(super) fn default_agent_config() -> AgentConfig {
    AgentConfig {
        bind_addr: "127.0.0.1".to_string(),
        port: 17801,
        cups_base_url: DEFAULT_EXTERNAL_CUPS_BASE_URL.to_string(),
        worker_poll_ms: 300,
        worker_concurrency: 2,
        retry_max_attempts: 3,
        retry_backoff_base_sec: 2,
        retry_backoff_max_sec: 60,
        mock_mode: false,
        render_engine: "typst".to_string(),
        render_timeout_sec: 30,
        render_cache_ttl_sec: 86_400,
        render_cache_max_entries: 2_000,
        render_cache_disk_high_watermark_mb: 1_024,
        render_cache_disk_low_watermark_mb: 768,
        render_cache_cleanup_interval_sec: 60,
        backend_status_poll_ms: 1200,
        backend_status_timeout_sec: 180,
        submission_recovery_timeout_sec: 45,
        backend_unknown_to_attention_sec: 120,
        log_dir: default_log_dir().to_string_lossy().to_string(),
        log_file_prefix: "agent.log".to_string(),
        log_max_files: 14,
        log_max_total_mb: 1024,
        log_cleanup_interval_sec: 300,
        diagnostics_dir: default_diagnostics_dir().to_string_lossy().to_string(),
        diagnostics_max_files: 50,
        direct_job_max_bytes: DEFAULT_DIRECT_JOB_MAX_BYTES,
        auth_session_cookie_name: DEFAULT_AUTH_SESSION_COOKIE_NAME.to_string(),
        auth_session_ttl_sec: DEFAULT_AUTH_SESSION_TTL_SEC,
        auth_cookie_secure: false,
    }
}
