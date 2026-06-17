use std::{io::Read as _, path::Path};

use binstall_tar::Archive;
use flate2::read::GzDecoder;

use super::super::super::{ApiError, ApiResult};

pub(super) fn unpack_typst_package_archive(archive_bytes: &[u8], out_dir: &Path) -> ApiResult<()> {
    let tar_payload = if archive_bytes.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(archive_bytes);
        let mut output = Vec::new();
        decoder
            .read_to_end(&mut output)
            .map_err(|err| ApiError::BadRequest(format!("invalid gzip archive: {err}")))?;
        output
    } else {
        archive_bytes.to_vec()
    };

    let mut archive = Archive::new(&tar_payload[..]);
    archive
        .unpack(out_dir)
        .map_err(|err| ApiError::BadRequest(format!("invalid tar archive: {err}")))?;
    Ok(())
}

pub(super) fn copy_dir_recursive(source: &Path, target: &Path) -> ApiResult<()> {
    if !source.is_dir() {
        return Err(ApiError::BadRequest(format!(
            "package root is not a directory: {}",
            source.display()
        )));
    }

    std::fs::create_dir_all(target)
        .map_err(|err| ApiError::Internal(format!("create target dir failed: {err}")))?;
    let entries = std::fs::read_dir(source)
        .map_err(|err| ApiError::Internal(format!("read package root failed: {err}")))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| ApiError::Internal(format!("read package entry failed: {err}")))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|err| ApiError::Internal(format!("resolve entry type failed: {err}")))?;

        if file_type.is_symlink() {
            return Err(ApiError::BadRequest(
                "package archive contains symlink, which is not supported".to_string(),
            ));
        }
        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
            continue;
        }
        if file_type.is_file() {
            std::fs::copy(&source_path, &target_path).map_err(|err| {
                ApiError::Internal(format!(
                    "copy package file failed: {} -> {} ({err})",
                    source_path.display(),
                    target_path.display()
                ))
            })?;
        }
    }

    Ok(())
}

pub(super) fn prune_empty_package_parent_dirs(root: &Path, namespace: &str, name: &str) {
    let name_dir = root.join(namespace).join(name);
    if is_empty_directory(&name_dir) {
        let _ = std::fs::remove_dir(&name_dir);
    }

    let namespace_dir = root.join(namespace);
    if is_empty_directory(&namespace_dir) {
        let _ = std::fs::remove_dir(&namespace_dir);
    }
}

fn is_empty_directory(path: &Path) -> bool {
    match std::fs::read_dir(path) {
        Ok(mut entries) => entries.next().is_none(),
        Err(_) => false,
    }
}
