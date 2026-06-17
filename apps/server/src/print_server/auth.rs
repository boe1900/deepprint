#[path = "auth/api_keys.rs"]
mod api_keys;
#[path = "auth/core.rs"]
mod core;
#[path = "auth/sessions.rs"]
mod sessions;
#[path = "auth/users.rs"]
mod users;

#[cfg(test)]
pub(super) use api_keys::api_key_prefix_from_token;
pub(super) use api_keys::{
    api_key_scopes, create_api_key, list_api_keys, require_api_key_for_path,
    require_api_key_scope_for_path, revoke_api_key,
};
#[cfg(test)]
pub(super) use api_keys::{insert_api_key_record, require_api_key_scope, revoke_api_key_record};
pub(super) use core::{
    hash_password, normalize_auth_provider_key, normalize_auth_text, normalize_optional_auth_text,
};
#[cfg(test)]
pub(super) use core::verify_password;
pub(super) use sessions::{
    auth_change_password, auth_login, auth_logout, auth_me, enforce_console_session,
};
pub(super) use users::{create_user, delete_user, list_users, reset_user_password, update_user};
#[cfg(test)]
pub(super) use users::{delete_auth_user_for_path, insert_local_auth_user};
