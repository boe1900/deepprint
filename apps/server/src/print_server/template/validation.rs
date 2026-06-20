use serde_json::Value;

use super::super::{rendering::sanitize_source_file_name, ApiError, ApiResult};
use super::NormalizedTemplatePayload;

pub(super) fn validate_template_group_name(name: &str) -> ApiResult<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(
            "TEMPLATE_GROUP_NAME_REQUIRED",
            "template group name is required",
        ));
    }
    if trimmed.chars().count() > 60 {
        return Err(ApiError::bad_request(
            "TEMPLATE_GROUP_NAME_TOO_LONG",
            "template group name must be 60 characters or fewer",
        ));
    }
    Ok(())
}

pub(super) fn normalize_template_payload(
    group_id: &str,
    name: &str,
    description: &str,
    output_name: &str,
    typst_code: &str,
    sample_data: &str,
) -> ApiResult<NormalizedTemplatePayload> {
    let group_id = group_id.trim().to_string();
    if group_id.is_empty() {
        return Err(ApiError::bad_request(
            "TEMPLATE_GROUP_ID_REQUIRED",
            "group_id is required",
        ));
    }

    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::bad_request(
            "TEMPLATE_NAME_REQUIRED",
            "template name is required",
        ));
    }
    if name.chars().count() > 120 {
        return Err(ApiError::bad_request(
            "TEMPLATE_NAME_TOO_LONG",
            "template name must be 120 characters or fewer",
        ));
    }

    let description = description.trim().to_string();
    if description.chars().count() > 240 {
        return Err(ApiError::bad_request(
            "TEMPLATE_DESCRIPTION_TOO_LONG",
            "template description must be 240 characters or fewer",
        ));
    }

    let output_name = if output_name.trim().is_empty() {
        build_default_output_name(&name)
    } else {
        sanitize_source_file_name(output_name)
    };

    let typst_code = typst_code.to_string();
    if typst_code.trim().is_empty() {
        return Err(ApiError::bad_request(
            "TEMPLATE_TYPST_CODE_REQUIRED",
            "typst_code is required",
        ));
    }
    if typst_code.len() > 2 * 1024 * 1024 {
        return Err(ApiError::bad_request(
            "TEMPLATE_TYPST_CODE_TOO_LARGE",
            "typst_code is too large (max 2MB)",
        ));
    }

    let sample_data = sample_data.trim().to_string();
    if sample_data.is_empty() {
        return Err(ApiError::bad_request(
            "TEMPLATE_SAMPLE_DATA_REQUIRED",
            "sample_data is required",
        ));
    }
    serde_json::from_str::<Value>(&sample_data).map_err(|err| {
        ApiError::bad_request(
            "TEMPLATE_SAMPLE_DATA_INVALID_JSON",
            format!("sample_data must be valid JSON: {err}"),
        )
    })?;

    Ok((
        group_id,
        name,
        description,
        output_name,
        typst_code,
        sample_data,
    ))
}

pub(super) fn map_template_write_error(err: rusqlite::Error) -> ApiError {
    if matches!(err, rusqlite::Error::QueryReturnedNoRows) {
        return ApiError::bad_request(
            "TEMPLATE_GROUP_NOT_FOUND",
            "linked template group does not exist",
        );
    }
    if let rusqlite::Error::SqliteFailure(_, Some(message)) = &err {
        if message.contains("template_groups.name") {
            return ApiError::conflict(
                "TEMPLATE_GROUP_NAME_EXISTS",
                "template group name already exists",
            );
        }
        if message.contains("templates.group_id, templates.name") {
            return ApiError::conflict(
                "TEMPLATE_NAME_EXISTS_IN_GROUP",
                "template name already exists in this group",
            );
        }
    }
    ApiError::Db(err)
}

fn build_default_output_name(name: &str) -> String {
    let mut output = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_whitespace() || ch == '-' || ch == '_' {
            if !output.ends_with('-') {
                output.push('-');
            }
        }
    }

    let base = output.trim_matches('-');
    let file_name = if base.is_empty() {
        "template-output"
    } else {
        base
    };
    format!("{file_name}.pdf")
}
