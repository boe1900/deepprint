use super::super::AgentConfig;
use super::helpers::{bool_env, set_parsed, set_string};

pub(super) fn apply_core_env(cfg: &mut AgentConfig) {
    set_string("DEEPPRINT_AGENT_BIND", &mut cfg.bind_addr);
    set_parsed("DEEPPRINT_AGENT_PORT", &mut cfg.port, |value| value);
    set_string("DEEPPRINT_CUPS_BASE_URL", &mut cfg.cups_base_url);
    set_parsed(
        "DEEPPRINT_AGENT_WORKER_POLL_MS",
        &mut cfg.worker_poll_ms,
        |value: u64| value.max(100),
    );
    set_parsed(
        "DEEPPRINT_AGENT_WORKER_CONCURRENCY",
        &mut cfg.worker_concurrency,
        |value: u16| value.clamp(1, 64),
    );
    set_parsed(
        "DEEPPRINT_RETRY_MAX_ATTEMPTS",
        &mut cfg.retry_max_attempts,
        |value: u16| value.clamp(1, 20),
    );
    set_parsed(
        "DEEPPRINT_RETRY_BACKOFF_BASE_SEC",
        &mut cfg.retry_backoff_base_sec,
        |value: u64| value.max(1),
    );
    set_parsed(
        "DEEPPRINT_RETRY_BACKOFF_MAX_SEC",
        &mut cfg.retry_backoff_max_sec,
        |value: u64| value.max(1),
    );

    if let Some(value) = bool_env("DEEPPRINT_AGENT_MOCK") {
        cfg.mock_mode = value;
    }
}

pub(super) fn apply_render_env(cfg: &mut AgentConfig) {
    if let Ok(value) = std::env::var("DEEPPRINT_RENDER_ENGINE") {
        let normalized = value.trim().to_lowercase();
        if matches!(normalized.as_str(), "typst" | "text") {
            cfg.render_engine = normalized;
        }
    }

    set_parsed(
        "DEEPPRINT_RENDER_TIMEOUT_SEC",
        &mut cfg.render_timeout_sec,
        |value: u64| value.max(5),
    );
    set_parsed(
        "DEEPPRINT_RENDER_CACHE_TTL_SEC",
        &mut cfg.render_cache_ttl_sec,
        |value| value,
    );
    set_parsed(
        "DEEPPRINT_RENDER_CACHE_MAX_ENTRIES",
        &mut cfg.render_cache_max_entries,
        |value| value,
    );
    set_parsed(
        "DEEPPRINT_RENDER_CACHE_DISK_HIGH_WATERMARK_MB",
        &mut cfg.render_cache_disk_high_watermark_mb,
        |value| value,
    );
    set_parsed(
        "DEEPPRINT_RENDER_CACHE_DISK_LOW_WATERMARK_MB",
        &mut cfg.render_cache_disk_low_watermark_mb,
        |value| value,
    );
    set_parsed(
        "DEEPPRINT_RENDER_CACHE_CLEANUP_INTERVAL_SEC",
        &mut cfg.render_cache_cleanup_interval_sec,
        |value: u64| value.max(10),
    );
}

pub(super) fn apply_backend_env(cfg: &mut AgentConfig) {
    set_parsed(
        "DEEPPRINT_BACKEND_STATUS_POLL_MS",
        &mut cfg.backend_status_poll_ms,
        |value: u64| value.max(200),
    );
    set_parsed(
        "DEEPPRINT_BACKEND_STATUS_TIMEOUT_SEC",
        &mut cfg.backend_status_timeout_sec,
        |value: u64| value.max(10),
    );
    set_parsed(
        "DEEPPRINT_SUBMISSION_RECOVERY_TIMEOUT_SEC",
        &mut cfg.submission_recovery_timeout_sec,
        |value: u64| value.max(10),
    );
    set_parsed(
        "DEEPPRINT_BACKEND_UNKNOWN_TO_ATTENTION_SEC",
        &mut cfg.backend_unknown_to_attention_sec,
        |value: u64| value.max(30),
    );
}

pub(super) fn apply_log_env(cfg: &mut AgentConfig) {
    set_string("DEEPPRINT_LOG_DIR", &mut cfg.log_dir);
    set_string("DEEPPRINT_LOG_FILE_PREFIX", &mut cfg.log_file_prefix);
    set_parsed("DEEPPRINT_LOG_MAX_FILES", &mut cfg.log_max_files, |value| {
        value
    });
    set_parsed(
        "DEEPPRINT_LOG_MAX_TOTAL_MB",
        &mut cfg.log_max_total_mb,
        |value| value,
    );
    set_parsed(
        "DEEPPRINT_LOG_CLEANUP_INTERVAL_SEC",
        &mut cfg.log_cleanup_interval_sec,
        |value: u64| value.max(30),
    );
    set_string("DEEPPRINT_DIAGNOSTICS_DIR", &mut cfg.diagnostics_dir);
    set_parsed(
        "DEEPPRINT_DIAGNOSTICS_MAX_FILES",
        &mut cfg.diagnostics_max_files,
        |value| value,
    );
    set_parsed(
        "DEEPPRINT_DIRECT_JOB_MAX_BYTES",
        &mut cfg.direct_job_max_bytes,
        |value: u64| value.max(1024),
    );
}

pub(super) fn apply_auth_env(cfg: &mut AgentConfig) {
    set_string(
        "DEEPPRINT_AUTH_SESSION_COOKIE_NAME",
        &mut cfg.auth_session_cookie_name,
    );
    set_parsed(
        "DEEPPRINT_AUTH_SESSION_TTL_SEC",
        &mut cfg.auth_session_ttl_sec,
        |value: u64| value.max(300),
    );
    if let Some(value) = bool_env("DEEPPRINT_AUTH_COOKIE_SECURE") {
        cfg.auth_cookie_secure = value;
    }
}
