use std::sync::Arc;

use axum::{extract::State, Json};

use super::super::super::{
    utils::decode_base64_payload, AgentState, ApiError, ApiResult, MAX_TYPST_PACKAGE_ARCHIVE_BYTES,
};
use super::super::{InstallTypstPackageRequest, InstallTypstPackageResponse, TypstPackageOrigin};
use super::fs::{copy_dir_recursive, unpack_typst_package_archive};
use super::manifest::{locate_typst_package_root, read_typst_package_manifest};
use crate::renderer;

pub(crate) async fn install_typst_package(
    State(state): State<Arc<AgentState>>,
    Json(payload): Json<InstallTypstPackageRequest>,
) -> ApiResult<Json<InstallTypstPackageResponse>> {
    super::super::validation::validate_install_typst_package_payload(&payload)?;
    let archive_bytes = decode_base64_payload(&payload.archive_base64)
        .map_err(|_| ApiError::BadRequest("archive_base64 is invalid base64".to_string()))?;

    if archive_bytes.len() > MAX_TYPST_PACKAGE_ARCHIVE_BYTES {
        return Err(ApiError::BadRequest(format!(
            "archive exceeds size limit: {} bytes (max {} bytes)",
            archive_bytes.len(),
            MAX_TYPST_PACKAGE_ARCHIVE_BYTES
        )));
    }

    let staging_dir = std::env::temp_dir()
        .join("deepprint-studio")
        .join("typst-package-staging")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&staging_dir)
        .map_err(|err| ApiError::Internal(format!("create staging dir failed: {err}")))?;

    let install_result = (|| -> ApiResult<InstallTypstPackageResponse> {
        unpack_typst_package_archive(&archive_bytes, &staging_dir)?;
        let package_root = locate_typst_package_root(&staging_dir)?;
        let manifest = read_typst_package_manifest(&package_root.join("typst.toml"))?;

        let name =
            super::super::validation::sanitize_package_segment(&manifest.name, "package.name")?;
        let version = super::super::validation::sanitize_package_segment(
            &manifest.version,
            "package.version",
        )?;
        let namespace = "local".to_string();

        let target_dir = state
            .typst_local_packages_root
            .join(&namespace)
            .join(&name)
            .join(&version);

        let replaced = if target_dir.exists() {
            if !payload.replace_existing {
                return Err(ApiError::Conflict(format!(
                    "package already exists: @{namespace}/{name}:{version}"
                )));
            }
            std::fs::remove_dir_all(&target_dir).map_err(|err| {
                ApiError::Internal(format!("remove existing package failed: {err}"))
            })?;
            true
        } else {
            false
        };

        if let Some(parent) = target_dir.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                ApiError::Internal(format!("create package parent failed: {err}"))
            })?;
        }

        copy_dir_recursive(&package_root, &target_dir)?;
        renderer::invalidate_preview_renderer();

        Ok(InstallTypstPackageResponse {
            origin: TypstPackageOrigin::Local,
            namespace: namespace.clone(),
            name: name.clone(),
            version: version.clone(),
            replaced,
            import_snippet: format!("#import \"@{namespace}/{name}:{version}\": *"),
        })
    })();

    let _ = std::fs::remove_dir_all(&staging_dir);
    install_result.map(Json)
}
