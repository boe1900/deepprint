use std::path::Path;

#[cfg(test)]
use rusqlite::Connection;
use uuid::Uuid;

use super::super::super::{
    models::{ApiError, CreateUserRequest, UpdateUserRequest},
    AuthUserRecord, USER_ROLE_ADMIN, USER_ROLE_OPERATOR, USER_STATUS_ACTIVE,
};
use super::super::core::{
    hash_password, normalize_auth_provider_key, normalize_auth_role, normalize_auth_status,
    normalize_auth_text, normalize_optional_auth_field,
};
use super::errors::map_unique_user_error;
use crate::storage::{
    count_active_admins_at_path, delete_auth_user_record_at_path,
    insert_local_auth_user_record_at_path, load_auth_user_by_id_at_path,
    reset_local_auth_password_by_admin_at_path, update_auth_user_record_at_path,
    AuthUserUpdateInput, LocalAuthUserInsertInput,
};

pub(super) fn insert_local_auth_user_for_path(
    db_path: &Path,
    payload: &CreateUserRequest,
    now: i64,
) -> Result<AuthUserRecord, ApiError> {
    let input = build_local_auth_user_insert_input(payload, now)?;
    insert_local_auth_user_record_at_path(db_path, input).map_err(map_unique_user_error)
}

#[cfg(test)]
pub(crate) fn insert_local_auth_user(
    conn: &Connection,
    payload: &CreateUserRequest,
    now: i64,
) -> Result<AuthUserRecord, ApiError> {
    let input = build_local_auth_user_insert_input(payload, now)?;
    crate::storage::insert_local_auth_user_record(conn, input).map_err(map_unique_user_error)
}

pub(super) fn update_auth_user_for_path(
    db_path: &Path,
    user_id: &str,
    payload: &UpdateUserRequest,
    actor_user_id: &str,
    updated_at: i64,
) -> Result<AuthUserRecord, ApiError> {
    let current = load_auth_user_by_id_at_path(db_path, user_id)?
        .ok_or_else(|| ApiError::NotFound(format!("user not found: {user_id}")))?;

    let email = match payload.email.as_deref() {
        Some(value) => normalize_optional_auth_field(value, 320).map_err(ApiError::BadRequest)?,
        None => current.email.clone(),
    };
    let display_name = payload
        .display_name
        .as_deref()
        .map(|value| normalize_auth_text(value, 128).map_err(ApiError::BadRequest))
        .transpose()?
        .unwrap_or_else(|| current.display_name.clone());
    let role = payload
        .role
        .as_deref()
        .map(normalize_auth_role)
        .transpose()?
        .unwrap_or_else(|| current.role.clone());
    let status = payload
        .status
        .as_deref()
        .map(normalize_auth_status)
        .transpose()?
        .unwrap_or_else(|| current.status.clone());

    if current.id == actor_user_id && status != current.status {
        return Err(ApiError::BadRequest(
            "cannot change your own status".to_string(),
        ));
    }
    if current.id == actor_user_id && role != current.role {
        return Err(ApiError::BadRequest(
            "cannot change your own role".to_string(),
        ));
    }
    ensure_not_last_active_admin_for_path(db_path, &current, &role, &status)?;

    update_auth_user_record_at_path(
        db_path,
        user_id,
        &AuthUserUpdateInput {
            email,
            display_name,
            role,
            status,
            updated_at,
        },
    )?
    .ok_or_else(|| ApiError::Internal("updated user not found".to_string()))
}

pub(crate) fn delete_auth_user_for_path(
    db_path: &Path,
    user_id: &str,
    actor_user_id: &str,
) -> Result<AuthUserRecord, ApiError> {
    let current = load_auth_user_by_id_at_path(db_path, user_id)?
        .ok_or_else(|| ApiError::NotFound(format!("user not found: {user_id}")))?;

    if current.id == actor_user_id {
        return Err(ApiError::BadRequest(
            "cannot delete your own account".to_string(),
        ));
    }
    ensure_not_deleting_last_active_admin_for_path(db_path, &current)?;

    if !delete_auth_user_record_at_path(db_path, user_id)? {
        return Err(ApiError::Internal("deleted user not found".to_string()));
    }
    Ok(current)
}

pub(super) fn reset_local_auth_password_for_path(
    db_path: &Path,
    user_id: &str,
    password: &str,
    updated_at: i64,
) -> Result<AuthUserRecord, ApiError> {
    load_auth_user_by_id_at_path(db_path, user_id)?
        .ok_or_else(|| ApiError::NotFound(format!("user not found: {user_id}")))?;
    let password_hash = hash_password(password)
        .map_err(|err| ApiError::Internal(format!("hash password failed: {err}")))?;

    let changed =
        reset_local_auth_password_by_admin_at_path(db_path, user_id, &password_hash, updated_at)?;
    if !changed {
        return Err(ApiError::NotFound(format!(
            "local auth identity not found for user: {user_id}"
        )));
    }

    load_auth_user_by_id_at_path(db_path, user_id)?
        .ok_or_else(|| ApiError::Internal("reset user not found".to_string()))
}

fn build_local_auth_user_insert_input(
    payload: &CreateUserRequest,
    now: i64,
) -> Result<LocalAuthUserInsertInput, ApiError> {
    let username = normalize_auth_text(&payload.username, 128).map_err(ApiError::BadRequest)?;
    let provider_key = normalize_auth_provider_key(&username)?;
    let email = payload
        .email
        .as_deref()
        .map(|value| normalize_optional_auth_field(value, 320).map_err(ApiError::BadRequest))
        .transpose()?
        .flatten();
    let display_name = payload
        .display_name
        .as_deref()
        .map(|value| normalize_auth_text(value, 128).map_err(ApiError::BadRequest))
        .transpose()?
        .unwrap_or_else(|| username.clone());
    let role = payload
        .role
        .as_deref()
        .map(normalize_auth_role)
        .transpose()?
        .unwrap_or_else(|| USER_ROLE_OPERATOR.to_string());
    let password_hash = hash_password(&payload.password)
        .map_err(|err| ApiError::Internal(format!("hash password failed: {err}")))?;
    let user_id = format!("user-{}", Uuid::new_v4());

    Ok(LocalAuthUserInsertInput {
        user_id,
        username,
        email,
        display_name,
        role,
        provider_key,
        password_hash,
        now,
    })
}

fn ensure_not_last_active_admin_for_path(
    db_path: &Path,
    current: &AuthUserRecord,
    next_role: &str,
    next_status: &str,
) -> Result<(), ApiError> {
    let removes_active_admin = current.role == USER_ROLE_ADMIN
        && current.status == USER_STATUS_ACTIVE
        && (next_role != USER_ROLE_ADMIN || next_status != USER_STATUS_ACTIVE);
    if !removes_active_admin {
        return Ok(());
    }

    let active_admin_count = count_active_admins_at_path(db_path)?;
    if active_admin_count <= 1 {
        return Err(ApiError::Conflict(
            "cannot remove the last active admin".to_string(),
        ));
    }
    Ok(())
}

fn ensure_not_deleting_last_active_admin_for_path(
    db_path: &Path,
    current: &AuthUserRecord,
) -> Result<(), ApiError> {
    if current.role != USER_ROLE_ADMIN || current.status != USER_STATUS_ACTIVE {
        return Ok(());
    }

    let active_admin_count = count_active_admins_at_path(db_path)?;
    if active_admin_count <= 1 {
        return Err(ApiError::Conflict(
            "cannot remove the last active admin".to_string(),
        ));
    }
    Ok(())
}
