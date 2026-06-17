use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{extract::State, Json};
use super::super::{
    utils::{clamp_u64_to_i64, decode_base64_payload, system_time_to_unix_ms},
    AgentState, ApiError, ApiResult, MAX_TYPST_FONT_FILE_BYTES,
};
use super::validation::{sanitize_typst_font_file_name, validate_install_typst_font_payload};
use super::{
    DeleteTypstFontRequest, DeleteTypstFontResponse, InstallTypstFontRequest,
    InstallTypstFontResponse, TypstFontInfo, TypstFontsResponse,
};
use crate::renderer;
use crate::print_server::shared::ENV_TYPST_DEFAULT_FONTS_ROOT;

const DEFAULT_FONT_SEED_ROOT: &str = "/opt/deepprint/default-fonts";
const FONTS_INITIALIZED_MARKER_FILE: &str = ".deepprint-fonts-initialized";
const LEGACY_SEEDED_FONT_MANIFEST_FILE: &str = ".deepprint-seeded-fonts.json";

pub(crate) async fn list_typst_fonts(
    State(state): State<Arc<AgentState>>,
) -> ApiResult<Json<TypstFontsResponse>> {
    let fonts = collect_typst_fonts(state.typst_fonts_root.as_ref())?;
    Ok(Json(TypstFontsResponse { fonts }))
}

pub(crate) async fn install_typst_font(
    State(state): State<Arc<AgentState>>,
    Json(payload): Json<InstallTypstFontRequest>,
) -> ApiResult<Json<InstallTypstFontResponse>> {
    validate_install_typst_font_payload(&payload)?;
    let file_bytes = decode_base64_payload(&payload.file_base64)
        .map_err(|_| ApiError::BadRequest("file_base64 is invalid base64".to_string()))?;

    if file_bytes.is_empty() {
        return Err(ApiError::BadRequest(
            "file_base64 decoded bytes must not be empty".to_string(),
        ));
    }
    if file_bytes.len() > MAX_TYPST_FONT_FILE_BYTES {
        return Err(ApiError::BadRequest(format!(
            "font file exceeds size limit: {} bytes (max {} bytes)",
            file_bytes.len(),
            MAX_TYPST_FONT_FILE_BYTES
        )));
    }

    let file_name = sanitize_typst_font_file_name(&payload.file_name)?;
    let target_path = state.typst_fonts_root.join(&file_name);
    let replaced = if target_path.exists() {
        if !payload.replace_existing {
            return Err(ApiError::Conflict(format!("font already exists: {file_name}")));
        }
        if target_path.is_dir() {
            return Err(ApiError::BadRequest(format!("font path is a directory: {file_name}")));
        }
        true
    } else {
        false
    };

    std::fs::write(&target_path, &file_bytes)
        .map_err(|err| ApiError::Internal(format!("write font file failed: {err}")))?;
    renderer::invalidate_preview_renderer();

    let size_bytes = std::fs::metadata(&target_path)
        .map(|metadata| clamp_u64_to_i64(metadata.len()))
        .unwrap_or_else(|_| clamp_u64_to_i64(file_bytes.len() as u64));

    Ok(Json(InstallTypstFontResponse {
        file_name,
        size_bytes,
        replaced,
    }))
}

pub(crate) async fn delete_typst_font(
    State(state): State<Arc<AgentState>>,
    Json(payload): Json<DeleteTypstFontRequest>,
) -> ApiResult<Json<DeleteTypstFontResponse>> {
    let file_name = sanitize_typst_font_file_name(&payload.file_name)?;
    let target_path = state.typst_fonts_root.join(&file_name);
    if !target_path.exists() {
        return Err(ApiError::NotFound(format!("font not found: {file_name}")));
    }
    if target_path.is_dir() {
        return Err(ApiError::BadRequest(format!("font path is a directory: {file_name}")));
    }

    std::fs::remove_file(&target_path)
        .map_err(|err| ApiError::Internal(format!("delete font failed: {err}")))?;
    renderer::invalidate_preview_renderer();
    Ok(Json(DeleteTypstFontResponse {
        file_name,
        deleted: true,
    }))
}

pub(crate) fn collect_typst_fonts(root: &Path) -> ApiResult<Vec<TypstFontInfo>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut fonts = Vec::new();
    let entries = std::fs::read_dir(root)
        .map_err(|err| ApiError::Internal(format!("read fonts root failed: {err}")))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| ApiError::Internal(format!("read font entry failed: {err}")))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let Ok(validated_name) = sanitize_typst_font_file_name(&file_name) else {
            continue;
        };
        let metadata = entry
            .metadata()
            .map_err(|err| ApiError::Internal(format!("read font metadata failed: {err}")))?;
        let modified_at_ms = metadata.modified().ok().and_then(system_time_to_unix_ms);
        fonts.push(TypstFontInfo {
            file_name: validated_name,
            size_bytes: clamp_u64_to_i64(metadata.len()),
            modified_at_ms,
        });
    }

    fonts.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    Ok(fonts)
}

pub(crate) fn ensure_default_typst_fonts(root: &Path) -> ApiResult<()> {
    std::fs::create_dir_all(root)
        .map_err(|err| ApiError::Internal(format!("create fonts root failed: {err}")))?;

    cleanup_legacy_font_state(root)?;

    let marker_path = root.join(FONTS_INITIALIZED_MARKER_FILE);
    if marker_path.exists() {
        return Ok(());
    }

    if has_managed_fonts(root)? {
        write_fonts_initialized_marker(&marker_path)?;
        return Ok(());
    }

    let seed_root = resolve_default_font_root();
    if !seed_root.exists() {
        write_fonts_initialized_marker(&marker_path)?;
        return Ok(());
    }

    let entries = std::fs::read_dir(&seed_root)
        .map_err(|err| ApiError::Internal(format!("read default fonts root failed: {err}")))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| ApiError::Internal(format!("read default font entry failed: {err}")))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        let validated_name = match sanitize_typst_font_file_name(&file_name) {
            Ok(name) => name,
            Err(_) => continue,
        };
        let target_path = root.join(&validated_name);

        if target_path.exists() {
            continue;
        }

        std::fs::copy(&path, &target_path).map_err(|err| {
            ApiError::Internal(format!(
                "copy default font {} to {} failed: {err}",
                path.display(),
                target_path.display()
            ))
        })?;
    }

    write_fonts_initialized_marker(&marker_path)?;
    Ok(())
}

fn resolve_default_font_root() -> PathBuf {
    match std::env::var(ENV_TYPST_DEFAULT_FONTS_ROOT) {
        Ok(raw) if !raw.trim().is_empty() => PathBuf::from(raw),
        _ => PathBuf::from(DEFAULT_FONT_SEED_ROOT),
    }
}

fn has_managed_fonts(root: &Path) -> ApiResult<bool> {
    let entries = std::fs::read_dir(root)
        .map_err(|err| ApiError::Internal(format!("read fonts root failed: {err}")))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| ApiError::Internal(format!("read font entry failed: {err}")))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        if sanitize_typst_font_file_name(&file_name).is_ok() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn write_fonts_initialized_marker(marker_path: &Path) -> ApiResult<()> {
    std::fs::write(marker_path, b"initialized")
        .map_err(|err| ApiError::Internal(format!("write fonts initialized marker failed: {err}")))?;
    Ok(())
}

fn cleanup_legacy_font_state(root: &Path) -> ApiResult<()> {
    let legacy_manifest_path = root.join(LEGACY_SEEDED_FONT_MANIFEST_FILE);
    if !legacy_manifest_path.exists() {
        return Ok(());
    }

    std::fs::remove_file(&legacy_manifest_path).map_err(|err| {
        ApiError::Internal(format!(
            "remove legacy seeded fonts manifest {} failed: {err}",
            legacy_manifest_path.display()
        ))
    })?;
    Ok(())
}
