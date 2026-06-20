#[path = "preview/cache.rs"]
mod cache;
#[path = "preview/keys.rs"]
mod keys;
#[path = "preview/response.rs"]
mod response;

use std::{sync::Arc, time::Instant};

use axum::{extract::State, response::Response, Json};
use tracing::debug;
use uuid::Uuid;

use super::PreviewTypstRequest;
use crate::{
    print_server::{utils, AgentState, ApiError, ApiResult},
    renderer::RenderRequest,
};

pub(super) use keys::{build_render_cache_key, to_storage_render_cache_key};

pub(super) async fn preview_typst(
    State(state): State<Arc<AgentState>>,
    Json(payload): Json<PreviewTypstRequest>,
) -> ApiResult<Response> {
    utils::validate_preview_typst_payload(&payload)?;
    let preview_started = Instant::now();

    let cache_key = keys::build_render_cache_key_from_preview_request(
        &payload.template_content,
        &payload.data,
        &payload.print_options,
    )?;
    let render_request = RenderRequest {
        job_id: format!("preview-{}", Uuid::new_v4()),
        request_id: format!("preview-{}", Uuid::new_v4()),
        template_content: payload.template_content,
        data: payload.data,
        print_options: serde_json::to_value(payload.print_options)
            .map_err(|err| {
                ApiError::bad_request(
                    "PRINT_OPTIONS_INVALID_JSON",
                    format!("print_options must be valid JSON: {err}"),
                )
            })?,
    };

    let render_result =
        cache::load_or_render_preview(state.as_ref(), &render_request, &cache_key, preview_started)
            .await?;
    debug!(
        job_id = %render_request.job_id,
        elapsed_ms = preview_started.elapsed().as_millis(),
        "typst preview completed"
    );
    response::build_preview_response(render_result)
}
