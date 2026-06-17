use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct AuthLoginRequest {
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthChangePasswordRequest {
    pub(crate) current_password: String,
    pub(crate) new_password: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateUserRequest {
    pub(crate) username: String,
    pub(crate) password: String,
    #[serde(default)]
    pub(crate) email: Option<String>,
    #[serde(default)]
    pub(crate) display_name: Option<String>,
    #[serde(default)]
    pub(crate) role: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct UpdateUserRequest {
    #[serde(default)]
    pub(crate) email: Option<String>,
    #[serde(default)]
    pub(crate) display_name: Option<String>,
    #[serde(default)]
    pub(crate) role: Option<String>,
    #[serde(default)]
    pub(crate) status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ResetUserPasswordRequest {
    pub(crate) password: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateApiKeyRequest {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) scopes: Vec<String>,
    #[serde(default)]
    pub(crate) expires_at: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct AuthUserResponse {
    pub(crate) id: String,
    pub(crate) username: String,
    pub(crate) email: Option<String>,
    pub(crate) display_name: String,
    pub(crate) role: String,
    pub(crate) status: String,
    pub(crate) must_change_password: bool,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthLoginResponse {
    pub(crate) authenticated: bool,
    pub(crate) user: AuthUserResponse,
    pub(crate) expires_at: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthMeResponse {
    pub(crate) authenticated: bool,
    pub(crate) login_enabled: bool,
    pub(crate) user: Option<AuthUserResponse>,
    pub(crate) expires_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthLogoutResponse {
    pub(crate) logged_out: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthChangePasswordResponse {
    pub(crate) changed: bool,
    pub(crate) user: AuthUserResponse,
    pub(crate) expires_at: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct ListUsersResponse {
    pub(crate) users: Vec<AuthUserResponse>,
}

#[derive(Debug, Serialize)]
pub(crate) struct UserResponse {
    pub(crate) user: AuthUserResponse,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResetUserPasswordResponse {
    pub(crate) user: AuthUserResponse,
}

#[derive(Debug, Serialize)]
pub(crate) struct DeleteUserResponse {
    pub(crate) deleted: bool,
    pub(crate) user: AuthUserResponse,
}

#[derive(Debug, Serialize)]
pub(crate) struct ListApiKeysResponse {
    pub(crate) api_keys: Vec<ApiKeyResponseItem>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateApiKeyResponse {
    pub(crate) api_key: ApiKeyResponseItem,
    pub(crate) token: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct RevokeApiKeyResponse {
    pub(crate) api_key: ApiKeyResponseItem,
}

#[derive(Debug, Serialize)]
pub(crate) struct ApiKeyResponseItem {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) key_prefix: String,
    pub(crate) scopes: Vec<String>,
    pub(crate) status: String,
    pub(crate) created_by_user_id: Option<String>,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
    pub(crate) last_used_at: Option<i64>,
    pub(crate) revoked_at: Option<i64>,
    pub(crate) expires_at: Option<i64>,
}
