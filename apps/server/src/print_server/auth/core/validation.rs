use super::super::super::models::{
    ApiError, AuthChangePasswordRequest, AuthLoginRequest, CreateUserRequest,
};
use super::normalization::{
    normalize_auth_role, normalize_auth_text, normalize_optional_auth_field,
};

pub(crate) fn validate_auth_login_payload(payload: &AuthLoginRequest) -> Result<(), ApiError> {
    normalize_auth_text(&payload.username, 128).map_err(ApiError::BadRequest)?;
    if payload.password.is_empty() {
        return Err(ApiError::BadRequest("password is required".to_string()));
    }
    Ok(())
}

pub(crate) fn validate_auth_change_password_payload(
    payload: &AuthChangePasswordRequest,
) -> Result<(), ApiError> {
    if payload.current_password.is_empty() {
        return Err(ApiError::BadRequest(
            "current password is required".to_string(),
        ));
    }
    validate_new_auth_password(&payload.new_password)?;
    if payload.current_password == payload.new_password {
        return Err(ApiError::BadRequest(
            "new password must be different".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_create_user_payload(payload: &CreateUserRequest) -> Result<(), ApiError> {
    normalize_auth_text(&payload.username, 128).map_err(ApiError::BadRequest)?;
    validate_new_auth_password(&payload.password)?;
    if let Some(email) = &payload.email {
        normalize_optional_auth_field(email, 320).map_err(ApiError::BadRequest)?;
    }
    if let Some(display_name) = &payload.display_name {
        normalize_auth_text(display_name, 128).map_err(ApiError::BadRequest)?;
    }
    if let Some(role) = &payload.role {
        normalize_auth_role(role)?;
    }
    Ok(())
}

pub(crate) fn validate_new_auth_password(password: &str) -> Result<(), ApiError> {
    if password.len() < 8 {
        return Err(ApiError::BadRequest(
            "password must be at least 8 characters".to_string(),
        ));
    }
    if password.len() > 256 {
        return Err(ApiError::BadRequest(
            "password must be at most 256 characters".to_string(),
        ));
    }
    if password.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(
            "password contains invalid control characters".to_string(),
        ));
    }
    Ok(())
}
