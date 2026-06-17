use serde_json::Value;

use super::super::{rendering::sanitize_source_file_name, ApiError, ApiResult};
use super::NormalizedTemplatePayload;

pub(super) fn validate_template_group_name(name: &str) -> ApiResult<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ApiError::BadRequest("分组名称不能为空".to_string()));
    }
    if trimmed.chars().count() > 60 {
        return Err(ApiError::BadRequest(
            "分组名称不能超过 60 个字符".to_string(),
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
        return Err(ApiError::BadRequest("group_id is required".to_string()));
    }

    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::BadRequest("模板名称不能为空".to_string()));
    }
    if name.chars().count() > 120 {
        return Err(ApiError::BadRequest(
            "模板名称不能超过 120 个字符".to_string(),
        ));
    }

    let description = description.trim().to_string();
    if description.chars().count() > 240 {
        return Err(ApiError::BadRequest(
            "模板描述不能超过 240 个字符".to_string(),
        ));
    }

    let output_name = if output_name.trim().is_empty() {
        build_default_output_name(&name)
    } else {
        sanitize_source_file_name(output_name)
    };

    let typst_code = typst_code.to_string();
    if typst_code.trim().is_empty() {
        return Err(ApiError::BadRequest("typst_code is required".to_string()));
    }
    if typst_code.len() > 2 * 1024 * 1024 {
        return Err(ApiError::BadRequest(
            "typst_code is too large (max 2MB)".to_string(),
        ));
    }

    let sample_data = sample_data.trim().to_string();
    if sample_data.is_empty() {
        return Err(ApiError::BadRequest("sample_data is required".to_string()));
    }
    serde_json::from_str::<Value>(&sample_data)
        .map_err(|err| ApiError::BadRequest(format!("sample_data 必须是有效 JSON: {err}")))?;

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
        return ApiError::BadRequest("关联的模板分组不存在".to_string());
    }
    if let rusqlite::Error::SqliteFailure(_, Some(message)) = &err {
        if message.contains("template_groups.name") {
            return ApiError::Conflict("模板分组名称已存在".to_string());
        }
        if message.contains("templates.group_id, templates.name") {
            return ApiError::Conflict("该分组下已存在同名模板".to_string());
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
