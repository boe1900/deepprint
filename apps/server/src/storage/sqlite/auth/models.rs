pub const LOCAL_AUTH_PROVIDER: &str = "local";
pub const USER_STATUS_ACTIVE: &str = "active";
pub const USER_STATUS_DISABLED: &str = "disabled";
pub const USER_ROLE_ADMIN: &str = "admin";
pub const USER_ROLE_OPERATOR: &str = "operator";

#[derive(Debug, Clone)]
pub struct AuthUserRecord {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub must_change_password: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct LocalAuthIdentityRecord {
    pub user: AuthUserRecord,
    pub password_hash: String,
}

#[derive(Debug, Clone)]
pub struct AuthSessionRecord {
    pub user: AuthUserRecord,
    pub expires_at: i64,
}

pub struct LocalAuthUserInsertInput {
    pub user_id: String,
    pub username: String,
    pub email: Option<String>,
    pub display_name: String,
    pub role: String,
    pub provider_key: String,
    pub password_hash: String,
    pub now: i64,
}

pub struct AuthUserUpdateInput {
    pub email: Option<String>,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub updated_at: i64,
}
