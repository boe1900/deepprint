use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;

use super::super::{ApiError, ApiResult};

pub(crate) fn decode_base64_payload(raw: &str) -> ApiResult<Vec<u8>> {
    let trimmed = raw.trim();
    let payload = if trimmed.starts_with("data:") {
        trimmed
            .split_once(',')
            .map(|(_, base64)| base64)
            .ok_or_else(|| ApiError::BadRequest("invalid data URL payload".to_string()))?
    } else {
        trimmed
    };

    BASE64_STANDARD
        .decode(payload)
        .map_err(|_| ApiError::BadRequest("file_content_base64 is invalid base64".to_string()))
}
