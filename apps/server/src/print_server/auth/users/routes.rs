use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    http::HeaderMap,
    Json,
};

use super::super::super::{
    models::{
        ApiError, CreateUserRequest, DeleteUserResponse, ListUsersResponse,
        ResetUserPasswordRequest, ResetUserPasswordResponse, UpdateUserRequest, UserResponse,
    },
    utils::now_unix,
    AgentState,
};
use super::super::core::{
    auth_user_response, validate_create_user_payload, validate_new_auth_password,
};
use super::super::sessions::require_admin_session_for_path;
use super::records::{
    delete_auth_user_for_path, insert_local_auth_user_for_path, reset_local_auth_password_for_path,
    update_auth_user_for_path,
};
use crate::storage::list_auth_users_at_path;

pub(crate) async fn list_users(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<ListUsersResponse>, ApiError> {
    require_admin_session_for_path(state.db_path.as_ref(), &state.config, &headers)?;
    let users = list_auth_users_at_path(state.db_path.as_ref())?
        .into_iter()
        .map(auth_user_response)
        .collect();
    Ok(Json(ListUsersResponse { users }))
}

pub(crate) async fn create_user(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    validate_create_user_payload(&payload)?;
    require_admin_session_for_path(state.db_path.as_ref(), &state.config, &headers)?;
    let user = insert_local_auth_user_for_path(state.db_path.as_ref(), &payload, now_unix())?;
    Ok(Json(UserResponse {
        user: auth_user_response(user),
    }))
}

pub(crate) async fn update_user(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    AxumPath(user_id): AxumPath<String>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let admin_session =
        require_admin_session_for_path(state.db_path.as_ref(), &state.config, &headers)?;
    let user = update_auth_user_for_path(
        state.db_path.as_ref(),
        &user_id,
        &payload,
        &admin_session.user.id,
        now_unix(),
    )?;
    Ok(Json(UserResponse {
        user: auth_user_response(user),
    }))
}

pub(crate) async fn reset_user_password(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    AxumPath(user_id): AxumPath<String>,
    Json(payload): Json<ResetUserPasswordRequest>,
) -> Result<Json<ResetUserPasswordResponse>, ApiError> {
    validate_new_auth_password(&payload.password)?;
    let admin_session =
        require_admin_session_for_path(state.db_path.as_ref(), &state.config, &headers)?;
    if admin_session.user.id == user_id {
        return Err(ApiError::BadRequest(
            "use change password for the current user".to_string(),
        ));
    }
    let user = reset_local_auth_password_for_path(
        state.db_path.as_ref(),
        &user_id,
        &payload.password,
        now_unix(),
    )?;
    Ok(Json(ResetUserPasswordResponse {
        user: auth_user_response(user),
    }))
}

pub(crate) async fn delete_user(
    State(state): State<Arc<AgentState>>,
    headers: HeaderMap,
    AxumPath(user_id): AxumPath<String>,
) -> Result<Json<DeleteUserResponse>, ApiError> {
    let admin_session =
        require_admin_session_for_path(state.db_path.as_ref(), &state.config, &headers)?;
    let user = delete_auth_user_for_path(state.db_path.as_ref(), &user_id, &admin_session.user.id)?;
    Ok(Json(DeleteUserResponse {
        deleted: true,
        user: auth_user_response(user),
    }))
}
