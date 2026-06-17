use std::path::PathBuf;

use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};

use super::super::insert_preview_header;
use crate::{
    print_server::{
        ApiError, ApiResult, PREVIEW_EXPOSE_HEADERS, PREVIEW_HEADER_OUTPUT_KIND,
        PREVIEW_HEADER_PAGE_COUNT, PREVIEW_HEADER_PAGE_HEIGHT_PT, PREVIEW_HEADER_PAGE_WIDTH_PT,
    },
    renderer::RenderResult,
};

#[derive(Debug, serde::Serialize)]
struct PreviewTypstMetadata {
    output_kind: String,
    page_count: u32,
    page_width_pt: Option<f64>,
    page_height_pt: Option<f64>,
}

pub(super) fn build_preview_response(render_result: RenderResult) -> ApiResult<Response> {
    if render_result.output_kind != "typst" {
        return Err(ApiError::BadRequest(format!(
            "preview only supports typst output, got {}",
            render_result.output_kind
        )));
    }

    let artifact = PathBuf::from(&render_result.artifact_path);
    let payload = std::fs::read(&artifact).map_err(|err| {
        ApiError::Internal(format!(
            "read preview artifact {} failed: {err}",
            artifact.display()
        ))
    })?;

    let metadata = PreviewTypstMetadata {
        output_kind: render_result.output_kind,
        page_count: render_result.page_count,
        page_width_pt: render_result.page_width_pt,
        page_height_pt: render_result.page_height_pt,
    };

    Ok((
        StatusCode::OK,
        build_preview_headers(&metadata)?,
        Body::from(payload),
    )
        .into_response())
}

fn build_preview_headers(metadata: &PreviewTypstMetadata) -> ApiResult<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", HeaderValue::from_static("application/pdf"));
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
    headers.insert(
        "access-control-expose-headers",
        HeaderValue::from_static(PREVIEW_EXPOSE_HEADERS),
    );
    insert_preview_header(
        &mut headers,
        PREVIEW_HEADER_OUTPUT_KIND,
        &metadata.output_kind,
    )?;
    insert_preview_header(
        &mut headers,
        PREVIEW_HEADER_PAGE_COUNT,
        &metadata.page_count.to_string(),
    )?;
    if let Some(width_pt) = metadata.page_width_pt {
        insert_preview_header(
            &mut headers,
            PREVIEW_HEADER_PAGE_WIDTH_PT,
            &width_pt.to_string(),
        )?;
    }
    if let Some(height_pt) = metadata.page_height_pt {
        insert_preview_header(
            &mut headers,
            PREVIEW_HEADER_PAGE_HEIGHT_PT,
            &height_pt.to_string(),
        )?;
    }
    Ok(headers)
}
