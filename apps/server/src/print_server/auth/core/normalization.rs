use super::super::super::{
    models::ApiError, USER_ROLE_ADMIN, USER_ROLE_OPERATOR, USER_STATUS_ACTIVE, USER_STATUS_DISABLED,
};

pub(crate) fn normalize_auth_role(raw: &str) -> Result<String, ApiError> {
    let normalized = normalize_auth_text(raw, 64)
        .map_err(ApiError::BadRequest)?
        .to_lowercase();
    if normalized != USER_ROLE_ADMIN && normalized != USER_ROLE_OPERATOR {
        return Err(ApiError::BadRequest("unsupported user role".to_string()));
    }
    Ok(normalized)
}

pub(crate) fn normalize_auth_status(raw: &str) -> Result<String, ApiError> {
    let normalized = normalize_auth_text(raw, 64)
        .map_err(ApiError::BadRequest)?
        .to_lowercase();
    if normalized != USER_STATUS_ACTIVE && normalized != USER_STATUS_DISABLED {
        return Err(ApiError::BadRequest("unsupported user status".to_string()));
    }
    Ok(normalized)
}

pub(crate) fn normalize_auth_provider_key(raw: &str) -> Result<String, ApiError> {
    Ok(normalize_auth_text(raw, 128)
        .map_err(ApiError::BadRequest)?
        .to_lowercase())
}

pub(crate) fn normalize_auth_text(raw: &str, max_len: usize) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("value is required".to_string());
    }
    if trimmed.len() > max_len {
        return Err(format!("value must be at most {max_len} characters"));
    }
    if trimmed.chars().any(char::is_control) {
        return Err("value contains invalid control characters".to_string());
    }
    Ok(trimmed.to_string())
}

pub(crate) fn normalize_optional_auth_text(raw: &str, max_len: usize) -> Option<String> {
    normalize_auth_text(raw, max_len).ok()
}

pub(crate) fn normalize_optional_auth_field(
    raw: &str,
    max_len: usize,
) -> Result<Option<String>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    normalize_auth_text(trimmed, max_len).map(Some)
}

pub(crate) fn validate_api_key_name(name: &str) -> Result<String, ApiError> {
    normalize_auth_text(name, 128).map_err(ApiError::BadRequest)
}
