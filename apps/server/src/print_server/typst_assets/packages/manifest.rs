use std::path::{Path, PathBuf};

use super::super::super::{ApiError, ApiResult};
use super::super::{TypstPackageToml, TypstPackageTomlPackage};

pub(crate) fn locate_typst_package_root(staging_dir: &Path) -> ApiResult<PathBuf> {
    if staging_dir.join("typst.toml").is_file() {
        return Ok(staging_dir.to_path_buf());
    }

    let mut candidates = Vec::new();
    let entries = std::fs::read_dir(staging_dir)
        .map_err(|err| ApiError::Internal(format!("read staging dir failed: {err}")))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| ApiError::Internal(format!("read staging entry failed: {err}")))?;
        let path = entry.path();
        if path.is_dir() && path.join("typst.toml").is_file() {
            candidates.push(path);
        }
    }

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err(ApiError::BadRequest(
            "typst.toml not found in uploaded package archive".to_string(),
        )),
        _ => Err(ApiError::BadRequest(
            "multiple typst.toml roots found in archive".to_string(),
        )),
    }
}

pub(crate) fn read_typst_package_manifest(path: &Path) -> ApiResult<TypstPackageTomlPackage> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| ApiError::BadRequest(format!("read typst.toml failed: {err}")))?;
    let parsed: TypstPackageToml = toml::from_str(&raw)
        .map_err(|err| ApiError::BadRequest(format!("invalid typst.toml: {err}")))?;
    Ok(parsed.package)
}
