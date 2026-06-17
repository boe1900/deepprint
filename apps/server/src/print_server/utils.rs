#[path = "utils/base64.rs"]
mod base64_payload;
#[path = "utils/bytes.rs"]
mod bytes;
#[path = "utils/pagination.rs"]
mod pagination;
#[path = "utils/time.rs"]
mod time;
#[path = "utils/validation.rs"]
mod validation;

pub(super) use base64_payload::decode_base64_payload;
pub(super) use bytes::{clamp_u64_to_i64, mb_to_bytes};
pub(super) use pagination::normalize_pagination;
pub(super) use time::{elapsed_millis, now_unix, system_time_to_unix_ms};
pub(super) use validation::{
    validate_create_direct_job_payload, validate_create_job_payload,
    validate_preview_typst_payload, validate_print_options, validate_printer_id_text,
    validate_request_id_text,
};
