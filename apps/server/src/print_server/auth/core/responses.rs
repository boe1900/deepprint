use super::super::super::{models::AuthUserResponse, AuthUserRecord};

pub(crate) fn auth_user_response(user: AuthUserRecord) -> AuthUserResponse {
    AuthUserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        display_name: user.display_name,
        role: user.role,
        status: user.status,
        must_change_password: user.must_change_password,
        created_at: user.created_at,
        updated_at: user.updated_at,
    }
}
