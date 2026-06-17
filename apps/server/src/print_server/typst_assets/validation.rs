use std::path::Path;

use super::super::{ApiError, ApiResult, SUPPORTED_TYPST_FONT_EXTENSIONS};
use super::{InstallTypstFontRequest, InstallTypstPackageRequest};

pub(crate) fn validate_install_typst_package_payload(
    payload: &InstallTypstPackageRequest,
) -> ApiResult<()> {
    if payload.archive_base64.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "archive_base64 is required".to_string(),
        ));
    }

    if let Some(file_name) = payload.file_name.as_deref() {
        let trimmed = file_name.trim();
        if trimmed.is_empty() {
            return Err(ApiError::BadRequest(
                "file_name cannot be empty when provided".to_string(),
            ));
        }
    }

    Ok(())
}

pub(crate) fn validate_install_typst_font_payload(
    payload: &InstallTypstFontRequest,
) -> ApiResult<()> {
    if payload.file_base64.trim().is_empty() {
        return Err(ApiError::BadRequest("file_base64 is required".to_string()));
    }
    if payload.file_name.trim().is_empty() {
        return Err(ApiError::BadRequest("file_name is required".to_string()));
    }
    Ok(())
}

pub(crate) fn sanitize_package_segment(raw: &str, label: &str) -> ApiResult<String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} cannot be empty")));
    }
    if value == "." || value == ".." {
        return Err(ApiError::BadRequest(format!("{label} is invalid")));
    }
    if value.contains('/') || value.contains('\\') || value.contains('\0') {
        return Err(ApiError::BadRequest(format!(
            "{label} contains invalid path separators"
        )));
    }
    Ok(value.to_string())
}

pub(crate) fn sanitize_typst_font_file_name(raw: &str) -> ApiResult<String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(ApiError::BadRequest(
            "file_name cannot be empty".to_string(),
        ));
    }
    if value == "." || value == ".." {
        return Err(ApiError::BadRequest("file_name is invalid".to_string()));
    }
    if value.contains('/') || value.contains('\\') || value.contains('\0') {
        return Err(ApiError::BadRequest(
            "file_name contains invalid path separators".to_string(),
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(
            "file_name contains control characters".to_string(),
        ));
    }
    let ext = Path::new(value)
        .extension()
        .and_then(|it| it.to_str())
        .map(|it| it.to_ascii_lowercase())
        .ok_or_else(|| {
            ApiError::BadRequest(format!(
                "unsupported font extension (allowed: {})",
                SUPPORTED_TYPST_FONT_EXTENSIONS.join(", ")
            ))
        })?;
    if !SUPPORTED_TYPST_FONT_EXTENSIONS.contains(&ext.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "unsupported font extension .{ext} (allowed: {})",
            SUPPORTED_TYPST_FONT_EXTENSIONS.join(", ")
        )));
    }
    Ok(value.to_string())
}
