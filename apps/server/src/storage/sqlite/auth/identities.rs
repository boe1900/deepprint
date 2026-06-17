use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use super::{
    models::{LocalAuthIdentityRecord, LOCAL_AUTH_PROVIDER, USER_STATUS_ACTIVE},
    rows::map_local_auth_identity_row,
};
use crate::storage::sqlite::open_connection;

pub fn load_local_auth_identity(
    conn: &Connection,
    provider_key: &str,
) -> rusqlite::Result<Option<LocalAuthIdentityRecord>> {
    conn.query_row(
        "SELECT
           u.id, u.username, u.email, u.display_name, u.role, u.status, u.must_change_password,
           u.created_at, u.updated_at,
           ai.password_hash
         FROM auth_identities ai
         JOIN users u ON u.id = ai.user_id
         WHERE ai.provider_type = ?1
           AND ai.provider_key = ?2
           AND ai.password_hash IS NOT NULL
         LIMIT 1",
        params![LOCAL_AUTH_PROVIDER, provider_key],
        map_local_auth_identity_row,
    )
    .optional()
}

pub fn load_local_auth_identity_at_path(
    db_path: &Path,
    provider_key: &str,
) -> rusqlite::Result<Option<LocalAuthIdentityRecord>> {
    let conn = open_connection(db_path)?;
    load_local_auth_identity(&conn, provider_key)
}

pub fn load_local_auth_identity_by_user_id(
    conn: &Connection,
    user_id: &str,
) -> rusqlite::Result<Option<LocalAuthIdentityRecord>> {
    conn.query_row(
        "SELECT
           u.id, u.username, u.email, u.display_name, u.role, u.status, u.must_change_password,
           u.created_at, u.updated_at,
           ai.password_hash
         FROM auth_identities ai
         JOIN users u ON u.id = ai.user_id
         WHERE ai.provider_type = ?1
           AND ai.user_id = ?2
           AND ai.password_hash IS NOT NULL
           AND u.status = ?3
         LIMIT 1",
        params![LOCAL_AUTH_PROVIDER, user_id, USER_STATUS_ACTIVE],
        map_local_auth_identity_row,
    )
    .optional()
}

pub fn load_local_auth_identity_by_user_id_at_path(
    db_path: &Path,
    user_id: &str,
) -> rusqlite::Result<Option<LocalAuthIdentityRecord>> {
    let conn = open_connection(db_path)?;
    load_local_auth_identity_by_user_id(&conn, user_id)
}

pub fn has_active_local_auth_user(conn: &Connection) -> rusqlite::Result<bool> {
    let exists: i64 = conn.query_row(
        "SELECT EXISTS(
           SELECT 1
           FROM auth_identities ai
           JOIN users u ON u.id = ai.user_id
           WHERE ai.provider_type = ?1
             AND ai.password_hash IS NOT NULL
             AND u.status = ?2
           LIMIT 1
         )",
        params![LOCAL_AUTH_PROVIDER, USER_STATUS_ACTIVE],
        |row| row.get(0),
    )?;
    Ok(exists != 0)
}

pub fn has_active_local_auth_user_at_path(db_path: &Path) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    has_active_local_auth_user(&conn)
}
