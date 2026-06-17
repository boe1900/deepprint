use serde::Serialize;

#[derive(Debug, Default, Clone, Copy, Serialize)]
pub(crate) struct LogUsageSnapshot {
    pub(crate) files_count: i64,
    pub(crate) disk_usage_bytes: i64,
}

#[derive(Serialize)]
pub(crate) struct HealthComponentProbe {
    pub(crate) ok: bool,
    pub(crate) latency_ms: u64,
    pub(crate) detail: String,
}
