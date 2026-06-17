use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use super::{
    models::{AuthSessionRecord, USER_STATUS_ACTIVE},
    rows::map_auth_session_row,
};
use crate::storage::sqlite::open_connection;

pub fn insert_auth_session(
    conn: &Connection,
    user_id: &str,
    session_token_hash: &str,
    created_at: i64,
    expires_at: i64,
    ip: Option<String>,
    user_agent: Option<String>,
) -> rusqlite::Result<()> {
    cleanup_auth_sessions(conn, created_at)?;
    conn.execute(
        "INSERT INTO sessions
           (id, user_id, session_token_hash, created_at, expires_at, last_seen_at, ip, user_agent,
            revoked_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
        params![
            format!("session-{}", Uuid::new_v4()),
            user_id,
            session_token_hash,
            created_at,
            expires_at,
            created_at,
            ip,
            user_agent
        ],
    )?;
    Ok(())
}

pub fn insert_auth_session_at_path(
    db_path: &Path,
    user_id: &str,
    session_token_hash: &str,
    created_at: i64,
    expires_at: i64,
    ip: Option<String>,
    user_agent: Option<String>,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    insert_auth_session(
        &conn,
        user_id,
        session_token_hash,
        created_at,
        expires_at,
        ip,
        user_agent,
    )
}

pub fn load_auth_session(
    conn: &Connection,
    session_token_hash: &str,
    now: i64,
) -> rusqlite::Result<Option<AuthSessionRecord>> {
    conn.query_row(
        "SELECT
           u.id, u.username, u.email, u.display_name, u.role, u.status, u.must_change_password,
           u.created_at, u.updated_at,
           s.expires_at
         FROM sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.session_token_hash = ?1
           AND s.revoked_at IS NULL
           AND s.expires_at > ?2
           AND u.status = ?3
         LIMIT 1",
        params![session_token_hash, now, USER_STATUS_ACTIVE],
        map_auth_session_row,
    )
    .optional()
}

pub fn load_auth_session_at_path(
    db_path: &Path,
    session_token_hash: &str,
    now: i64,
) -> rusqlite::Result<Option<AuthSessionRecord>> {
    let conn = open_connection(db_path)?;
    load_auth_session(&conn, session_token_hash, now)
}

pub fn touch_auth_session(
    conn: &Connection,
    session_token_hash: &str,
    last_seen_at: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sessions SET last_seen_at = ?1 WHERE session_token_hash = ?2",
        params![last_seen_at, session_token_hash],
    )?;
    Ok(())
}

pub fn touch_auth_session_at_path(
    db_path: &Path,
    session_token_hash: &str,
    last_seen_at: i64,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    touch_auth_session(&conn, session_token_hash, last_seen_at)
}

pub fn revoke_auth_session(
    conn: &Connection,
    session_token_hash: &str,
    revoked_at: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sessions
         SET revoked_at = ?1
         WHERE session_token_hash = ?2 AND revoked_at IS NULL",
        params![revoked_at, session_token_hash],
    )?;
    Ok(())
}

pub fn revoke_auth_session_at_path(
    db_path: &Path,
    session_token_hash: &str,
    revoked_at: i64,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    revoke_auth_session(&conn, session_token_hash, revoked_at)
}

fn cleanup_auth_sessions(conn: &Connection, now: i64) -> rusqlite::Result<()> {
    let revoked_cutoff = now.saturating_sub(30 * 24 * 60 * 60);
    conn.execute(
        "DELETE FROM sessions
         WHERE expires_at < ?1
            OR (revoked_at IS NOT NULL AND revoked_at < ?2)",
        params![now, revoked_cutoff],
    )?;
    Ok(())
}
