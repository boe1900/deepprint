use serde_json::Value;

use super::super::RenderCacheKey;
use crate::{
    print_server::{ApiError, ApiResult, JobRecord, PrintOptions},
    storage::RenderCacheKey as StorageRenderCacheKey,
};

pub(crate) fn build_render_cache_key(job: &JobRecord) -> RenderCacheKey {
    build_render_cache_key_from_json(
        job.template_content.as_str(),
        job.data_json.as_str(),
        job.print_options_json.as_str(),
    )
}

pub(crate) fn to_storage_render_cache_key(cache_key: &RenderCacheKey) -> StorageRenderCacheKey {
    StorageRenderCacheKey {
        key: cache_key.key.clone(),
        template_hash: cache_key.template_hash.clone(),
        data_hash: cache_key.data_hash.clone(),
        print_options_hash: cache_key.print_options_hash.clone(),
    }
}

pub(super) fn build_render_cache_key_from_preview_request(
    template_content: &str,
    data: &Value,
    print_options: &PrintOptions,
) -> ApiResult<RenderCacheKey> {
    let data_json = serde_json::to_string(data)
        .map_err(|err| ApiError::BadRequest(format!("data 必须是有效 JSON: {err}")))?;
    let print_options_json = serde_json::to_string(print_options)
        .map_err(|err| ApiError::BadRequest(format!("print_options 必须是有效 JSON: {err}")))?;

    Ok(build_render_cache_key_from_json(
        template_content,
        data_json.as_str(),
        print_options_json.as_str(),
    ))
}

fn build_render_cache_key_from_json(
    template_content: &str,
    data_json: &str,
    print_options_json: &str,
) -> RenderCacheKey {
    let template_hash = super::super::super::sha256_hex(template_content.as_bytes());
    let data_hash = super::super::super::sha256_hex(data_json.as_bytes());
    let print_options_hash = super::super::super::sha256_hex(print_options_json.as_bytes());
    let key = super::super::super::sha256_hex(
        format!("{template_hash}:{data_hash}:{print_options_hash}").as_bytes(),
    );

    RenderCacheKey {
        key,
        template_hash,
        data_hash,
        print_options_hash,
    }
}
