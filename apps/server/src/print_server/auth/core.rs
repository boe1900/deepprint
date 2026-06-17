#[path = "core/cookies.rs"]
mod cookies;
#[path = "core/normalization.rs"]
mod normalization;
#[path = "core/passwords.rs"]
mod passwords;
#[path = "core/responses.rs"]
mod responses;
#[path = "core/validation.rs"]
mod validation;

pub(crate) use cookies::{
    build_clear_session_cookie, build_session_cookie, generate_session_token, header_string,
    session_token_from_headers,
};
pub(crate) use normalization::{
    normalize_auth_provider_key, normalize_auth_role, normalize_auth_status, normalize_auth_text,
    normalize_optional_auth_field, normalize_optional_auth_text, validate_api_key_name,
};
pub(crate) use passwords::{hash_password, verify_password};
pub(crate) use responses::auth_user_response;
pub(crate) use validation::{
    validate_auth_change_password_payload, validate_auth_login_payload,
    validate_create_user_payload, validate_new_auth_password,
};
