use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::open_connection;

pub const METRIC_RENDER_CACHE_HIT_TOTAL: &str = "render_cache_hit_total";
pub const METRIC_RENDER_CACHE_MISS_TOTAL: &str = "render_cache_miss_total";
pub const METRIC_RENDER_CACHE_EVICT_TOTAL: &str = "render_cache_evict_total";
pub const METRIC_RENDER_CACHE_DISK_CLEANUP_TOTAL: &str = "render_cache_disk_cleanup_total";
pub const METRIC_RETRY_SCHEDULED_TOTAL: &str = "retry_scheduled_total";
pub const METRIC_DEAD_LETTER_TOTAL: &str = "dead_letter_total";
pub const METRIC_LOG_CLEANUP_TOTAL: &str = "log_cleanup_total";
pub const METRIC_TEMPLATE_WORKSPACE_SEEDED_V1: &str = "template_workspace_seeded_v1";

#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct CacheMetricsSnapshot {
    pub entries: i64,
    pub disk_usage_bytes: i64,
    pub hit_total: i64,
    pub miss_total: i64,
    pub evict_total: i64,
    pub disk_cleanup_total: i64,
    pub retry_scheduled_total: i64,
    pub dead_letter_total: i64,
    pub dead_letter_count: i64,
    pub log_cleanup_total: i64,
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct QueueMetricsSnapshot {
    pub queued_count: i64,
    pub rendering_count: i64,
    pub submitting_count: i64,
    pub printing_count: i64,
    pub needs_attention_count: i64,
    pub succeeded_count: i64,
    pub failed_count: i64,
    pub canceled_count: i64,
    pub terminal_total: i64,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub avg_succeeded_duration_sec: f64,
}

pub fn load_cache_metrics_snapshot(db_path: &Path) -> rusqlite::Result<CacheMetricsSnapshot> {
    let conn = open_connection(db_path)?;
    let entries = conn.query_row("SELECT COUNT(1) FROM render_cache", [], |row| row.get(0))?;
    let disk_usage_bytes = conn.query_row(
        "SELECT COALESCE(SUM(artifact_size_bytes), 0) FROM render_cache",
        [],
        |row| row.get(0),
    )?;
    let hit_total = read_agent_metric(&conn, METRIC_RENDER_CACHE_HIT_TOTAL)?;
    let miss_total = read_agent_metric(&conn, METRIC_RENDER_CACHE_MISS_TOTAL)?;
    let evict_total = read_agent_metric(&conn, METRIC_RENDER_CACHE_EVICT_TOTAL)?;
    let disk_cleanup_total = read_agent_metric(&conn, METRIC_RENDER_CACHE_DISK_CLEANUP_TOTAL)?;
    let retry_scheduled_total = read_agent_metric(&conn, METRIC_RETRY_SCHEDULED_TOTAL)?;
    let dead_letter_total = read_agent_metric(&conn, METRIC_DEAD_LETTER_TOTAL)?;
    let log_cleanup_total = read_agent_metric(&conn, METRIC_LOG_CLEANUP_TOTAL)?;
    let dead_letter_count =
        conn.query_row("SELECT COUNT(1) FROM dead_letter", [], |row| row.get(0))?;

    Ok(CacheMetricsSnapshot {
        entries,
        disk_usage_bytes,
        hit_total,
        miss_total,
        evict_total,
        disk_cleanup_total,
        retry_scheduled_total,
        dead_letter_total,
        dead_letter_count,
        log_cleanup_total,
    })
}

pub fn load_queue_metrics_snapshot(db_path: &Path) -> rusqlite::Result<QueueMetricsSnapshot> {
    let conn = open_connection(db_path)?;
    let mut snapshot = QueueMetricsSnapshot::default();

    let mut stmt = conn.prepare("SELECT status, COUNT(1) FROM jobs GROUP BY status")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let status: String = row.get(0)?;
        let count: i64 = row.get(1)?;
        match status.as_str() {
            "queued" => snapshot.queued_count = count,
            "rendering" => snapshot.rendering_count = count,
            "submitting" => snapshot.submitting_count = count,
            "printing" => snapshot.printing_count = count,
            "needs_attention" => snapshot.needs_attention_count = count,
            "succeeded" => snapshot.succeeded_count = count,
            "failed" => snapshot.failed_count = count,
            "canceled" => snapshot.canceled_count = count,
            _ => {}
        }
    }

    snapshot.terminal_total =
        snapshot.succeeded_count + snapshot.failed_count + snapshot.canceled_count;
    if snapshot.terminal_total > 0 {
        snapshot.success_rate = snapshot.succeeded_count as f64 / snapshot.terminal_total as f64;
        snapshot.failure_rate = snapshot.failed_count as f64 / snapshot.terminal_total as f64;
    }

    snapshot.avg_succeeded_duration_sec = conn.query_row(
        "SELECT COALESCE(AVG(CAST(updated_at - created_at AS REAL)), 0.0)
         FROM jobs
         WHERE status = 'succeeded'",
        [],
        |row| row.get(0),
    )?;

    Ok(snapshot)
}

pub fn read_agent_metric(conn: &Connection, key: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT value FROM agent_metrics WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
    .map(|value| value.unwrap_or(0))
}

pub fn increment_agent_metric(db_path: &Path, key: &str, delta: i64) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    increment_agent_metric_conn(&conn, key, delta)
}

pub fn increment_agent_metric_conn(
    conn: &Connection,
    key: &str,
    delta: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO agent_metrics (key, value)
         VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = value + excluded.value",
        params![key, delta],
    )?;
    Ok(())
}
