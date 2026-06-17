use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use tracing::warn;

use super::super::{models::LogUsageSnapshot, utils::clamp_u64_to_i64};

#[derive(Debug)]
struct LogFileEntry {
    path: PathBuf,
    modified_at: SystemTime,
    size_bytes: u64,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct LogRetentionSnapshot {
    pub(crate) files_count: i64,
    pub(crate) disk_usage_bytes: i64,
    pub(crate) removed_files: i64,
}

pub(crate) fn load_log_usage_snapshot(
    log_dir: &Path,
    file_prefix: &str,
) -> std::io::Result<LogUsageSnapshot> {
    let entries = collect_log_files(log_dir, file_prefix)?;
    let files_count = entries.len() as i64;
    let disk_usage_bytes = entries.iter().fold(0_i64, |acc, it| {
        acc.saturating_add(clamp_u64_to_i64(it.size_bytes))
    });

    Ok(LogUsageSnapshot {
        files_count,
        disk_usage_bytes,
    })
}

pub(crate) fn apply_log_retention(
    log_dir: &Path,
    file_prefix: &str,
    max_files: u64,
    max_total_bytes: u64,
) -> std::io::Result<LogRetentionSnapshot> {
    let mut entries = collect_log_files(log_dir, file_prefix)?;
    entries.sort_by_key(|it| it.modified_at);
    if entries.is_empty() {
        return Ok(LogRetentionSnapshot::default());
    }

    let marked_for_delete = mark_log_files_for_retention(&entries, max_files, max_total_bytes);
    let removed_files = remove_marked_log_files(&entries, &marked_for_delete);

    let usage = load_log_usage_snapshot(log_dir, file_prefix)?;
    Ok(LogRetentionSnapshot {
        files_count: usage.files_count,
        disk_usage_bytes: usage.disk_usage_bytes,
        removed_files,
    })
}

pub(crate) fn collect_recent_log_tails(
    log_dir: &Path,
    file_prefix: &str,
    max_files: u64,
    max_lines: u64,
    max_bytes_per_file: u64,
) -> std::io::Result<Vec<(String, String)>> {
    let mut log_files = collect_log_files(log_dir, file_prefix)?;
    log_files.sort_by_key(|entry| entry.modified_at);
    log_files.reverse();

    let mut output = Vec::new();
    for entry in log_files.into_iter().take(max_files.max(1) as usize) {
        let raw = std::fs::read(&entry.path)?;
        let start = raw.len().saturating_sub(max_bytes_per_file as usize);
        let text = String::from_utf8_lossy(&raw[start..]).to_string();
        let lines: Vec<&str> = text.lines().collect();
        let tail_start = lines.len().saturating_sub(max_lines as usize);
        let tail = lines[tail_start..].join("\n");
        let file_name = entry
            .path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown.log".to_string());
        output.push((file_name, tail));
    }

    Ok(output)
}

fn mark_log_files_for_retention(
    entries: &[LogFileEntry],
    max_files: u64,
    max_total_bytes: u64,
) -> Vec<bool> {
    let mut marked_for_delete = vec![false; entries.len()];
    if max_files > 0 && (entries.len() as u64) > max_files {
        let overflow = (entries.len() as u64 - max_files) as usize;
        for mark in marked_for_delete.iter_mut().take(overflow) {
            *mark = true;
        }
    }

    let mut total_bytes: u64 = entries
        .iter()
        .enumerate()
        .filter_map(|(idx, it)| (!marked_for_delete[idx]).then_some(it.size_bytes))
        .sum();
    if max_total_bytes > 0 && total_bytes > max_total_bytes {
        for (idx, entry) in entries.iter().enumerate() {
            if marked_for_delete[idx] {
                continue;
            }

            marked_for_delete[idx] = true;
            total_bytes = total_bytes.saturating_sub(entry.size_bytes);
            if total_bytes <= max_total_bytes {
                break;
            }
        }
    }

    marked_for_delete
}

fn remove_marked_log_files(entries: &[LogFileEntry], marked_for_delete: &[bool]) -> i64 {
    let mut removed_files = 0_i64;
    for (idx, entry) in entries.iter().enumerate() {
        if !marked_for_delete[idx] {
            continue;
        }

        match std::fs::remove_file(&entry.path) {
            Ok(_) => removed_files += 1,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => removed_files += 1,
            Err(err) => warn!(
                "failed to remove old log file {}: {err}",
                entry.path.display()
            ),
        }
    }
    removed_files
}

fn collect_log_files(log_dir: &Path, file_prefix: &str) -> std::io::Result<Vec<LogFileEntry>> {
    if !log_dir.exists() {
        return Ok(vec![]);
    }

    let mut entries = Vec::new();
    for dir_entry in std::fs::read_dir(log_dir)? {
        let dir_entry = dir_entry?;
        let path = dir_entry.path();
        let metadata = match dir_entry.metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                warn!("failed to stat log file {}: {err}", path.display());
                continue;
            }
        };
        if !metadata.is_file() {
            continue;
        }

        if !file_prefix.trim().is_empty() {
            let name = path
                .file_name()
                .map(|it| it.to_string_lossy())
                .unwrap_or_default();
            if !name.starts_with(file_prefix) {
                continue;
            }
        }

        entries.push(LogFileEntry {
            path,
            modified_at: metadata.modified().unwrap_or(UNIX_EPOCH),
            size_bytes: metadata.len(),
        });
    }

    Ok(entries)
}
