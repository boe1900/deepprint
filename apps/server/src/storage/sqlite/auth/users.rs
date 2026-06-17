use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use super::{
    models::{
        AuthUserRecord, AuthUserUpdateInput, LocalAuthUserInsertInput, LOCAL_AUTH_PROVIDER,
        USER_ROLE_ADMIN, USER_STATUS_ACTIVE,
    },
    rows::map_auth_user_row,
};
use crate::storage::sqlite::open_connection;

pub fn list_auth_users_at_path(db_path: &Path) -> rusqlite::Result<Vec<AuthUserRecord>> {
    let conn = open_connection(db_path)?;
    list_auth_users(&conn)
}

pub fn list_auth_users(conn: &Connection) -> rusqlite::Result<Vec<AuthUserRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, username, email, display_name, role, status, must_change_password,
           created_at, updated_at
         FROM users
         ORDER BY
           CASE status WHEN 'active' THEN 0 ELSE 1 END,
           CASE role WHEN 'admin' THEN 0 ELSE 1 END,
           username COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], map_auth_user_row)?;
    rows.collect()
}

pub fn load_auth_user(
    conn: &Connection,
    user_id: &str,
) -> rusqlite::Result<Option<AuthUserRecord>> {
    conn.query_row(
        "SELECT id, username, email, display_name, role, status, must_change_password,
           created_at, updated_at
         FROM users
         WHERE id = ?1 AND status = ?2
         LIMIT 1",
        params![user_id, USER_STATUS_ACTIVE],
        map_auth_user_row,
    )
    .optional()
}

pub fn load_auth_user_at_path(
    db_path: &Path,
    user_id: &str,
) -> rusqlite::Result<Option<AuthUserRecord>> {
    let conn = open_connection(db_path)?;
    load_auth_user(&conn, user_id)
}

pub fn load_auth_user_by_id(
    conn: &Connection,
    user_id: &str,
) -> rusqlite::Result<Option<AuthUserRecord>> {
    conn.query_row(
        "SELECT id, username, email, display_name, role, status, must_change_password,
           created_at, updated_at
         FROM users
         WHERE id = ?1
         LIMIT 1",
        params![user_id],
        map_auth_user_row,
    )
    .optional()
}

pub fn load_auth_user_by_id_at_path(
    db_path: &Path,
    user_id: &str,
) -> rusqlite::Result<Option<AuthUserRecord>> {
    let conn = open_connection(db_path)?;
    load_auth_user_by_id(&conn, user_id)
}

pub fn count_active_admins(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(1) FROM users WHERE role = ?1 AND status = ?2",
        params![USER_ROLE_ADMIN, USER_STATUS_ACTIVE],
        |row| row.get(0),
    )
}

pub fn count_active_admins_at_path(db_path: &Path) -> rusqlite::Result<i64> {
    let conn = open_connection(db_path)?;
    count_active_admins(&conn)
}

pub fn update_local_auth_password(
    conn: &Connection,
    user_id: &str,
    password_hash: &str,
    updated_at: i64,
    current_session_token_hash: &str,
) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE auth_identities
         SET password_hash = ?1, updated_at = ?2
         WHERE user_id = ?3 AND provider_type = ?4",
        params![password_hash, updated_at, user_id, LOCAL_AUTH_PROVIDER],
    )?;
    tx.execute(
        "UPDATE users
         SET must_change_password = 0, password_changed_at = ?1, updated_at = ?1
         WHERE id = ?2",
        params![updated_at, user_id],
    )?;
    tx.execute(
        "UPDATE sessions
         SET revoked_at = ?1
         WHERE user_id = ?2
           AND session_token_hash <> ?3
           AND revoked_at IS NULL",
        params![updated_at, user_id, current_session_token_hash],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn update_local_auth_password_at_path(
    db_path: &Path,
    user_id: &str,
    password_hash: &str,
    updated_at: i64,
    current_session_token_hash: &str,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    update_local_auth_password(
        &conn,
        user_id,
        password_hash,
        updated_at,
        current_session_token_hash,
    )
}

pub fn insert_local_auth_user_record(
    conn: &Connection,
    input: LocalAuthUserInsertInput,
) -> rusqlite::Result<AuthUserRecord> {
    let identity_id = format!("identity-{}", Uuid::new_v4());
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO users
           (id, username, email, display_name, role, status, must_change_password,
            password_changed_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, NULL, ?7, ?8)",
        params![
            input.user_id,
            input.username,
            input.email,
            input.display_name,
            input.role,
            USER_STATUS_ACTIVE,
            input.now,
            input.now
        ],
    )?;
    tx.execute(
        "INSERT INTO auth_identities
           (id, user_id, provider_type, provider_key, password_hash, provider_subject,
            provider_meta, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?6, ?7)",
        params![
            identity_id,
            input.user_id,
            LOCAL_AUTH_PROVIDER,
            input.provider_key,
            input.password_hash,
            input.now,
            input.now
        ],
    )?;
    tx.commit()?;

    load_auth_user_by_id(conn, &input.user_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn insert_local_auth_user_record_at_path(
    db_path: &Path,
    input: LocalAuthUserInsertInput,
) -> rusqlite::Result<AuthUserRecord> {
    let conn = open_connection(db_path)?;
    insert_local_auth_user_record(&conn, input)
}

pub fn update_auth_user_record(
    conn: &Connection,
    user_id: &str,
    input: &AuthUserUpdateInput,
) -> rusqlite::Result<Option<AuthUserRecord>> {
    let changed = conn.execute(
        "UPDATE users
         SET email = ?1, display_name = ?2, role = ?3, status = ?4, updated_at = ?5
         WHERE id = ?6",
        params![
            input.email,
            input.display_name,
            input.role,
            input.status,
            input.updated_at,
            user_id
        ],
    )?;
    if changed == 0 {
        return Ok(None);
    }
    load_auth_user_by_id(conn, user_id)
}

pub fn update_auth_user_record_at_path(
    db_path: &Path,
    user_id: &str,
    input: &AuthUserUpdateInput,
) -> rusqlite::Result<Option<AuthUserRecord>> {
    let conn = open_connection(db_path)?;
    update_auth_user_record(&conn, user_id, input)
}

pub fn delete_auth_user_record(conn: &Connection, user_id: &str) -> rusqlite::Result<bool> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE api_keys
         SET created_by_user_id = NULL
         WHERE created_by_user_id = ?1",
        params![user_id],
    )?;
    tx.execute("DELETE FROM sessions WHERE user_id = ?1", params![user_id])?;
    tx.execute(
        "DELETE FROM auth_identities WHERE user_id = ?1",
        params![user_id],
    )?;
    let changed = tx.execute("DELETE FROM users WHERE id = ?1", params![user_id])?;
    tx.commit()?;
    Ok(changed > 0)
}

pub fn delete_auth_user_record_at_path(db_path: &Path, user_id: &str) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    delete_auth_user_record(&conn, user_id)
}

pub fn reset_local_auth_password_by_admin(
    conn: &Connection,
    user_id: &str,
    password_hash: &str,
    updated_at: i64,
) -> rusqlite::Result<bool> {
    let tx = conn.unchecked_transaction()?;
    let changed = tx.execute(
        "UPDATE auth_identities
         SET password_hash = ?1, updated_at = ?2
         WHERE user_id = ?3 AND provider_type = ?4",
        params![password_hash, updated_at, user_id, LOCAL_AUTH_PROVIDER],
    )?;
    if changed == 0 {
        return Ok(false);
    }
    tx.execute(
        "UPDATE users
         SET must_change_password = 1, password_changed_at = NULL, updated_at = ?1
         WHERE id = ?2",
        params![updated_at, user_id],
    )?;
    tx.execute(
        "UPDATE sessions
         SET revoked_at = ?1
         WHERE user_id = ?2 AND revoked_at IS NULL",
        params![updated_at, user_id],
    )?;
    tx.commit()?;
    Ok(true)
}

pub fn reset_local_auth_password_by_admin_at_path(
    db_path: &Path,
    user_id: &str,
    password_hash: &str,
    updated_at: i64,
) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    reset_local_auth_password_by_admin(&conn, user_id, password_hash, updated_at)
}

pub fn is_unique_auth_user_violation(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(_, Some(message))
            if message.contains("UNIQUE constraint failed: users.username")
                || message.contains("UNIQUE constraint failed: users.email")
                || message.contains("UNIQUE constraint failed: auth_identities.provider_type, auth_identities.provider_key")
    )
}
