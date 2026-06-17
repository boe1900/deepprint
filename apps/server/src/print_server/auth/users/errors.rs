use super::super::super::models::ApiError;
use crate::storage::is_unique_auth_user_violation;

pub(super) fn map_unique_user_error(err: rusqlite::Error) -> ApiError {
    if is_unique_auth_user_violation(&err) {
        ApiError::Conflict("username already exists".to_string())
    } else {
        ApiError::Db(err)
    }
}
