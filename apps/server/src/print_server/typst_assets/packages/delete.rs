use std::sync::Arc;

use axum::{extract::State, Json};

use super::super::super::{AgentState, ApiError, ApiResult};
use super::super::{
    ClearTypstPreviewCacheResponse, DeleteTypstPackageRequest, DeleteTypstPackageResponse,
    TypstPackageOrigin,
};
use super::fs::prune_empty_package_parent_dirs;
use crate::renderer;

pub(crate) async fn delete_typst_package(
    State(state): State<Arc<AgentState>>,
    Json(payload): Json<DeleteTypstPackageRequest>,
) -> ApiResult<Json<DeleteTypstPackageResponse>> {
    let namespace =
        super::super::validation::sanitize_package_segment(&payload.namespace, "namespace")?;
    let name = super::super::validation::sanitize_package_segment(&payload.name, "name")?;
    let version = super::super::validation::sanitize_package_segment(&payload.version, "version")?;

    if payload.origin == TypstPackageOrigin::Local && namespace != "local" {
        return Err(ApiError::BadRequest(
            "local package origin only supports namespace=local".to_string(),
        ));
    }
    if payload.origin == TypstPackageOrigin::PreviewCache && namespace != "preview" {
        return Err(ApiError::BadRequest(
            "preview_cache origin only supports namespace=preview".to_string(),
        ));
    }

    let root = match payload.origin {
        TypstPackageOrigin::Local => state.typst_local_packages_root.as_ref(),
        TypstPackageOrigin::PreviewCache => state.typst_preview_cache_root.as_ref(),
    };
    let target_dir = root.join(&namespace).join(&name).join(&version);
    if !target_dir.exists() {
        return Err(ApiError::NotFound(format!(
            "package not found: @{namespace}/{name}:{version}"
        )));
    }
    std::fs::remove_dir_all(&target_dir)
        .map_err(|err| ApiError::Internal(format!("remove package failed: {err}")))?;
    prune_empty_package_parent_dirs(root, &namespace, &name);
    renderer::invalidate_preview_renderer();

    Ok(Json(DeleteTypstPackageResponse {
        origin: payload.origin,
        namespace,
        name,
        version,
        deleted: true,
    }))
}

pub(crate) async fn clear_typst_preview_cache(
    State(state): State<Arc<AgentState>>,
) -> ApiResult<Json<ClearTypstPreviewCacheResponse>> {
    let preview_root = state.typst_preview_cache_root.as_ref();
    let preview_namespace_dir = preview_root.join("preview");
    let removed = if preview_namespace_dir.exists() {
        std::fs::remove_dir_all(&preview_namespace_dir)
            .map_err(|err| ApiError::Internal(format!("clear preview cache failed: {err}")))?;
        true
    } else {
        false
    };
    std::fs::create_dir_all(preview_root)
        .map_err(|err| ApiError::Internal(format!("ensure preview cache root failed: {err}")))?;
    renderer::invalidate_preview_renderer();

    Ok(Json(ClearTypstPreviewCacheResponse { removed }))
}
