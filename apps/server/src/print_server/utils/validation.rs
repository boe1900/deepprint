use super::super::{
    submission::{CreateDirectJobRequest, CreateJobRequest, PreviewTypstRequest},
    ApiError, ApiResult, PrintOptions,
};

pub(crate) fn validate_create_job_payload(payload: &CreateJobRequest) -> ApiResult<()> {
    validate_request_id_text(&payload.request_id)?;
    if payload.template_content.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "template_content is required".to_string(),
        ));
    }
    validate_printer_id_text(&payload.printer_id)?;

    validate_print_options(&payload.print_options)
}

pub(crate) fn validate_create_direct_job_payload(
    payload: &CreateDirectJobRequest,
) -> ApiResult<()> {
    validate_request_id_text(&payload.request_id)?;
    if payload.file_name.trim().is_empty() {
        return Err(ApiError::BadRequest("file_name is required".to_string()));
    }
    if payload.file_name.len() > 255 {
        return Err(ApiError::BadRequest(
            "file_name length must be <= 255".to_string(),
        ));
    }
    if payload.file_content_base64.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "file_content_base64 is required".to_string(),
        ));
    }
    validate_printer_id_text(&payload.printer_id)?;
    if let Some(content_type) = payload.content_type.as_deref() {
        if content_type.trim().len() > 255 {
            return Err(ApiError::BadRequest(
                "content_type length must be <= 255".to_string(),
            ));
        }
    }
    validate_print_options(&payload.print_options)
}

pub(crate) fn validate_preview_typst_payload(payload: &PreviewTypstRequest) -> ApiResult<()> {
    if payload.template_content.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "template_content is required".to_string(),
        ));
    }
    if payload.template_content.len() > 2 * 1024 * 1024 {
        return Err(ApiError::BadRequest(
            "template_content is too large (max 2MB)".to_string(),
        ));
    }

    validate_print_options(&payload.print_options)
}

pub(crate) fn validate_print_options(options: &PrintOptions) -> ApiResult<()> {
    if let Some(copies) = options.copies {
        if copies == 0 || copies > 100 {
            return Err(ApiError::BadRequest(
                "copies must be between 1 and 100".to_string(),
            ));
        }
    }

    if let Some(page_ranges) = options
        .page_ranges
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        validate_page_ranges(page_ranges)?;
    }

    Ok(())
}

pub(crate) fn validate_request_id_text(request_id: &str) -> ApiResult<()> {
    if request_id.trim().is_empty() {
        return Err(ApiError::BadRequest("request_id is required".to_string()));
    }
    if request_id.len() > 128 {
        return Err(ApiError::BadRequest(
            "request_id length must be <= 128".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_printer_id_text(printer_id: &str) -> ApiResult<()> {
    if printer_id.trim().is_empty() {
        return Err(ApiError::BadRequest("printer_id is required".to_string()));
    }
    if printer_id.len() > 128 {
        return Err(ApiError::BadRequest(
            "printer_id length must be <= 128".to_string(),
        ));
    }
    Ok(())
}

fn validate_page_ranges(page_ranges: &str) -> ApiResult<()> {
    let valid = page_ranges
        .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
        .filter(|segment| !segment.trim().is_empty())
        .all(|segment| {
            let part = segment.trim();
            if let Some((start, end)) = part.split_once('-') {
                matches!(
                    (start.trim().parse::<u32>(), end.trim().parse::<u32>()),
                    (Ok(start), Ok(end)) if start > 0 && end >= start
                )
            } else {
                matches!(part.parse::<u32>(), Ok(page) if page > 0)
            }
        });
    if !valid {
        return Err(ApiError::BadRequest(
            "pageRanges must use positive page numbers like `1-3 5 7-9`".to_string(),
        ));
    }
    Ok(())
}
