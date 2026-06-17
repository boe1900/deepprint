use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use tracing::warn;

pub(crate) fn cleanup_old_diagnostic_bundles(dir: &Path, max_files: u64) -> std::io::Result<()> {
    if max_files == 0 || !dir.exists() {
        return Ok(());
    }

    let mut bundles = collect_diagnostic_bundles(dir)?;
    if bundles.len() <= max_files as usize {
        return Ok(());
    }

    bundles.sort_by_key(|(_, modified)| *modified);
    let overflow = bundles.len().saturating_sub(max_files as usize);
    for (path, _) in bundles.into_iter().take(overflow) {
        match std::fs::remove_file(&path) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => warn!(
                "failed to remove old diagnostics bundle {}: {err}",
                path.display()
            ),
        }
    }

    Ok(())
}

fn collect_diagnostic_bundles(
    dir: &Path,
) -> std::io::Result<Vec<(std::path::PathBuf, SystemTime)>> {
    let mut bundles = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if !metadata.is_file() {
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("zip") {
            continue;
        }
        bundles.push((path, metadata.modified().unwrap_or(UNIX_EPOCH)));
    }
    Ok(bundles)
}
