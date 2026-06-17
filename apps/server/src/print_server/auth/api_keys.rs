#[path = "api_keys/records.rs"]
mod records;
#[path = "api_keys/responses.rs"]
mod responses;
#[path = "api_keys/routes.rs"]
mod routes;
#[path = "api_keys/scopes.rs"]
mod scopes;
#[path = "api_keys/tokens.rs"]
mod tokens;

#[cfg(test)]
pub(crate) use records::{insert_api_key_record, revoke_api_key_record};
pub(crate) use responses::api_key_scopes;
pub(crate) use routes::{create_api_key, list_api_keys, revoke_api_key};
#[cfg(test)]
pub(crate) use tokens::api_key_prefix_from_token;
pub(crate) use tokens::require_api_key_for_path;
#[cfg(test)]
pub(crate) use tokens::require_api_key_scope;
pub(crate) use tokens::require_api_key_scope_for_path;
