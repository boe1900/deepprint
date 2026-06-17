use std::{
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension};
use tracing::warn;

use crate::renderer::RenderResult;

use super::{
    count_active_jobs_by_artifact_path, increment_agent_metric_conn, open_connection,
    METRIC_RENDER_CACHE_DISK_CLEANUP_TOTAL, METRIC_RENDER_CACHE_EVICT_TOTAL,
};

type RenderCacheLookupRow = (String, String, i64, Option<f64>, Option<f64>, i64);

#[derive(Debug, Clone)]
pub struct RenderCacheKey {
    pub key: String,
    pub template_hash: String,
    pub data_hash: String,
    pub print_options_hash: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DiskCleanupSnapshot {
    pub stale_removed: i64,
    pub watermark_evicted: i64,
    pub disk_usage_bytes: i64,
}

pub fn evict_render_cache_if_needed(
    db_path: &Path,
    ttl_sec: u64,
    max_entries: u64,
) -> rusqlite::Result<i64> {
    let conn = open_connection(db_path)?;
    let mut evicted = 0_i64;

    if ttl_sec > 0 {
        let cutoff = now_unix().saturating_sub(ttl_sec as i64);
        evicted += evict_render_cache_by_ttl(&conn, cutoff)?;
    }

    if max_entries > 0 {
        evicted += evict_render_cache_overflow(&conn, max_entries as i64)?;
    }

    if evicted > 0 {
        increment_agent_metric_conn(&conn, METRIC_RENDER_CACHE_EVICT_TOTAL, evicted)?;
    }

    Ok(evicted)
}

pub fn cleanup_render_cache_by_disk_watermark(
    db_path: &Path,
    high_watermark_bytes: u64,
    low_watermark_bytes: u64,
) -> rusqlite::Result<DiskCleanupSnapshot> {
    if high_watermark_bytes == 0 {
        return Ok(DiskCleanupSnapshot::default());
    }

    let conn = open_connection(db_path)?;
    cleanup_render_cache_by_disk_watermark_conn(&conn, high_watermark_bytes, low_watermark_bytes)
}

pub fn cleanup_render_cache_by_disk_watermark_conn(
    conn: &Connection,
    high_watermark_bytes: u64,
    low_watermark_bytes: u64,
) -> rusqlite::Result<DiskCleanupSnapshot> {
    let high = clamp_u64_to_i64(high_watermark_bytes);
    if high <= 0 {
        return Ok(DiskCleanupSnapshot::default());
    }

    let low = clamp_u64_to_i64(low_watermark_bytes.min(high_watermark_bytes));

    let mut stmt = conn.prepare(
        "SELECT cache_key, artifact_path, artifact_size_bytes
         FROM render_cache
         ORDER BY updated_at ASC, created_at ASC",
    )?;
    let mut rows = stmt.query([])?;
    let mut stale_candidates = Vec::new();
    let mut lru_candidates = Vec::new();
    let mut usage = 0_i64;
    while let Some(row) = rows.next()? {
        let cache_key: String = row.get(0)?;
        let artifact_path: String = row.get(1)?;
        let stored_size: i64 = row.get(2)?;

        match std::fs::metadata(&artifact_path) {
            Ok(metadata) => {
                let actual_size = clamp_u64_to_i64(metadata.len());
                if actual_size != stored_size {
                    let _ = conn.execute(
                        "UPDATE render_cache SET artifact_size_bytes = ?1 WHERE cache_key = ?2",
                        params![actual_size, cache_key.as_str()],
                    );
                }

                usage = usage.saturating_add(actual_size.max(0));
                lru_candidates.push((cache_key, artifact_path, actual_size.max(0)));
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                stale_candidates.push((cache_key, artifact_path));
            }
            Err(err) => {
                warn!("failed to stat cached artifact {artifact_path}: {err}");
                usage = usage.saturating_add(stored_size.max(0));
                lru_candidates.push((cache_key, artifact_path, stored_size.max(0)));
            }
        }
    }
    drop(rows);
    drop(stmt);

    let mut stale_removed = 0_i64;
    for (cache_key, artifact_path) in stale_candidates {
        stale_removed += remove_cache_entry(conn, &cache_key, &artifact_path)?;
    }

    let mut watermark_evicted = 0_i64;
    if usage > high {
        for (cache_key, artifact_path, artifact_size_bytes) in lru_candidates {
            if usage <= low {
                break;
            }

            let removed = remove_cache_entry(conn, &cache_key, &artifact_path)?;
            if removed == 1 {
                usage = usage.saturating_sub(artifact_size_bytes.max(0));
                watermark_evicted += 1;
            }
        }
    }

    let total_removed = stale_removed + watermark_evicted;
    if total_removed > 0 {
        increment_agent_metric_conn(conn, METRIC_RENDER_CACHE_EVICT_TOTAL, total_removed)?;
        increment_agent_metric_conn(conn, METRIC_RENDER_CACHE_DISK_CLEANUP_TOTAL, 1)?;
    }

    let disk_usage_bytes: i64 = conn.query_row(
        "SELECT COALESCE(SUM(artifact_size_bytes), 0) FROM render_cache",
        [],
        |row| row.get(0),
    )?;

    Ok(DiskCleanupSnapshot {
        stale_removed,
        watermark_evicted,
        disk_usage_bytes,
    })
}

pub fn try_load_render_cache(
    db_path: &Path,
    cache_key: &RenderCacheKey,
    job_id: &str,
) -> rusqlite::Result<Option<RenderResult>> {
    let conn = open_connection(db_path)?;
    let found: Option<RenderCacheLookupRow> = conn
        .query_row(
            "SELECT artifact_path, output_kind, page_count, page_width_pt, page_height_pt, artifact_size_bytes
             FROM render_cache
             WHERE cache_key = ?1",
            params![cache_key.key.as_str()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .optional()?;

    let Some((artifact_path, output_kind, page_count, page_width_pt, page_height_pt, stored_size)) =
        found
    else {
        return Ok(None);
    };

    if !Path::new(&artifact_path).exists() {
        let changed = conn.execute(
            "DELETE FROM render_cache WHERE cache_key = ?1",
            params![cache_key.key.as_str()],
        )?;
        if changed == 1 {
            let _ = increment_agent_metric_conn(&conn, METRIC_RENDER_CACHE_EVICT_TOTAL, 1);
        }
        warn!(
            "render cache entry stale for job {job_id}, removed key={}",
            cache_key.key
        );
        return Ok(None);
    }

    let artifact_size_bytes = file_size_bytes(&artifact_path).unwrap_or(stored_size.max(0));
    let _ = conn.execute(
        "UPDATE render_cache
         SET updated_at = ?1,
             hit_count = hit_count + 1,
             artifact_size_bytes = ?2
         WHERE cache_key = ?3",
        params![now_unix(), artifact_size_bytes, cache_key.key.as_str()],
    );

    Ok(Some(RenderResult {
        artifact_path,
        output_kind,
        page_count: page_count.max(0) as u32,
        page_width_pt,
        page_height_pt,
    }))
}

pub fn try_upsert_render_cache(
    db_path: &Path,
    cache_key: &RenderCacheKey,
    render_result: &RenderResult,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    let now = now_unix();
    let artifact_size_bytes = file_size_bytes(&render_result.artifact_path).unwrap_or(0);
    conn.execute(
        "INSERT INTO render_cache (
            cache_key,
            template_hash,
            data_hash,
            print_options_hash,
            artifact_path,
            artifact_size_bytes,
            output_kind,
            page_count,
            page_width_pt,
            page_height_pt,
            created_at,
            updated_at,
            hit_count
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, 0)
         ON CONFLICT(cache_key) DO UPDATE SET
            artifact_path = excluded.artifact_path,
            artifact_size_bytes = excluded.artifact_size_bytes,
            output_kind = excluded.output_kind,
            page_count = excluded.page_count,
            page_width_pt = excluded.page_width_pt,
            page_height_pt = excluded.page_height_pt,
            updated_at = excluded.updated_at",
        params![
            cache_key.key.as_str(),
            cache_key.template_hash.as_str(),
            cache_key.data_hash.as_str(),
            cache_key.print_options_hash.as_str(),
            render_result.artifact_path.as_str(),
            artifact_size_bytes,
            render_result.output_kind.as_str(),
            i64::from(render_result.page_count),
            render_result.page_width_pt,
            render_result.page_height_pt,
            now,
        ],
    )?;

    Ok(())
}

fn evict_render_cache_by_ttl(conn: &Connection, cutoff_updated_at: i64) -> rusqlite::Result<i64> {
    let mut stmt = conn.prepare(
        "SELECT cache_key, artifact_path
         FROM render_cache
         WHERE updated_at < ?1
         ORDER BY updated_at ASC",
    )?;
    let mut rows = stmt.query(params![cutoff_updated_at])?;
    let mut candidates = Vec::new();
    while let Some(row) = rows.next()? {
        candidates.push((row.get::<_, String>(0)?, row.get::<_, String>(1)?));
    }
    drop(rows);
    drop(stmt);

    let mut evicted = 0_i64;
    for (cache_key, artifact_path) in candidates {
        evicted += remove_cache_entry(conn, &cache_key, &artifact_path)?;
    }

    Ok(evicted)
}

fn evict_render_cache_overflow(conn: &Connection, max_entries: i64) -> rusqlite::Result<i64> {
    if max_entries <= 0 {
        return Ok(0);
    }

    let total: i64 = conn.query_row("SELECT COUNT(1) FROM render_cache", [], |row| row.get(0))?;
    let overflow = total.saturating_sub(max_entries);
    if overflow <= 0 {
        return Ok(0);
    }

    let mut stmt = conn.prepare(
        "SELECT cache_key, artifact_path
         FROM render_cache
         ORDER BY updated_at ASC, created_at ASC
         LIMIT ?1",
    )?;
    let mut rows = stmt.query(params![overflow])?;
    let mut candidates = Vec::new();
    while let Some(row) = rows.next()? {
        candidates.push((row.get::<_, String>(0)?, row.get::<_, String>(1)?));
    }
    drop(rows);
    drop(stmt);

    let mut evicted = 0_i64;
    for (cache_key, artifact_path) in candidates {
        evicted += remove_cache_entry(conn, &cache_key, &artifact_path)?;
    }

    Ok(evicted)
}

fn remove_cache_entry(
    conn: &Connection,
    cache_key: &str,
    artifact_path: &str,
) -> rusqlite::Result<i64> {
    let changed = conn.execute(
        "DELETE FROM render_cache WHERE cache_key = ?1",
        params![cache_key],
    )?;

    if changed == 1 {
        cleanup_cached_artifact_if_unused(conn, artifact_path)?;
        return Ok(1);
    }

    Ok(0)
}

fn cleanup_cached_artifact_if_unused(
    conn: &Connection,
    artifact_path: &str,
) -> rusqlite::Result<()> {
    if artifact_path.trim().is_empty() {
        return Ok(());
    }

    let cache_refs: i64 = conn.query_row(
        "SELECT COUNT(1) FROM render_cache WHERE artifact_path = ?1",
        params![artifact_path],
        |row| row.get(0),
    )?;
    if cache_refs > 0 {
        return Ok(());
    }

    let active_job_refs = count_active_jobs_by_artifact_path(conn, artifact_path)?;
    if active_job_refs > 0 {
        return Ok(());
    }

    match std::fs::remove_file(artifact_path) {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => warn!("failed to remove cached artifact file {artifact_path}: {err}"),
    }

    Ok(())
}

fn file_size_bytes(path: &str) -> Option<i64> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(clamp_u64_to_i64(metadata.len()))
}

fn clamp_u64_to_i64(value: u64) -> i64 {
    if value > i64::MAX as u64 {
        i64::MAX
    } else {
        value as i64
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}
