use std::collections::HashSet;

use super::super::super::{
    models::ApiError, API_SCOPE_JOB_READ, API_SCOPE_PREVIEW_CREATE, API_SCOPE_PRINTER_READ,
    API_SCOPE_PRINT_CREATE, API_SCOPE_TEMPLATE_READ,
};
use super::super::core::normalize_auth_text;

pub(crate) fn normalize_api_key_scopes(scopes: &[String]) -> Result<Vec<String>, ApiError> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for scope in scopes {
        let value = normalize_auth_text(scope, 64)
            .map_err(ApiError::BadRequest)?
            .to_lowercase();
        if !is_supported_api_scope(&value) {
            return Err(ApiError::BadRequest(format!(
                "unsupported api key scope: {value}"
            )));
        }
        if seen.insert(value.clone()) {
            normalized.push(value);
        }
    }

    if normalized.is_empty() {
        return Err(ApiError::BadRequest(
            "api key scopes must not be empty".to_string(),
        ));
    }

    Ok(normalized)
}

fn is_supported_api_scope(scope: &str) -> bool {
    matches!(
        scope,
        API_SCOPE_TEMPLATE_READ
            | API_SCOPE_PREVIEW_CREATE
            | API_SCOPE_PRINT_CREATE
            | API_SCOPE_PRINTER_READ
            | API_SCOPE_JOB_READ
    )
}
