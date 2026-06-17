use thiserror::Error;

#[path = "config/defaults.rs"]
mod defaults;
#[path = "config/env.rs"]
mod env;

use super::mb_to_bytes;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub bind_addr: String,
    pub port: u16,
    pub cups_base_url: String,
    pub worker_poll_ms: u64,
    pub worker_concurrency: u16,
    pub retry_max_attempts: u16,
    pub retry_backoff_base_sec: u64,
    pub retry_backoff_max_sec: u64,
    pub mock_mode: bool,
    pub render_engine: String,
    pub render_timeout_sec: u64,
    pub render_cache_ttl_sec: u64,
    pub render_cache_max_entries: u64,
    pub render_cache_disk_high_watermark_mb: u64,
    pub render_cache_disk_low_watermark_mb: u64,
    pub render_cache_cleanup_interval_sec: u64,
    pub backend_status_poll_ms: u64,
    pub backend_status_timeout_sec: u64,
    pub submission_recovery_timeout_sec: u64,
    pub backend_unknown_to_attention_sec: u64,
    pub log_dir: String,
    pub log_file_prefix: String,
    pub log_max_files: u64,
    pub log_max_total_mb: u64,
    pub log_cleanup_interval_sec: u64,
    pub diagnostics_dir: String,
    pub diagnostics_max_files: u64,
    pub direct_job_max_bytes: u64,
    pub auth_session_cookie_name: String,
    pub auth_session_ttl_sec: u64,
    pub auth_cookie_secure: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        defaults::default_agent_config()
    }
}

impl AgentConfig {
    pub fn from_env() -> Self {
        env::agent_config_from_env()
    }

    pub fn render_cache_disk_high_watermark_bytes(&self) -> u64 {
        mb_to_bytes(self.render_cache_disk_high_watermark_mb)
    }

    pub fn render_cache_disk_low_watermark_bytes(&self) -> u64 {
        mb_to_bytes(self.render_cache_disk_low_watermark_mb)
    }
}

#[derive(Debug, Error)]
pub enum AgentBootError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

pub type PrintServerConfig = AgentConfig;
pub type PrintServerBootError = AgentBootError;
