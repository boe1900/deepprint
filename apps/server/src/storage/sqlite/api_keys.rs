use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use super::open_connection;

pub const API_KEY_STATUS_ACTIVE: &str = "active";
pub const API_KEY_STATUS_REVOKED: &str = "revoked";

#[derive(Debug, Clone)]
pub struct ApiKeyRecord {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub scopes_json: String,
    pub status: String,
    pub created_by_user_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_used_at: Option<i64>,
    pub revoked_at: Option<i64>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ApiKeyRecordInput {
    pub name: String,
    pub key_prefix: String,
    pub secret_hash: String,
    pub scopes_json: String,
    pub status: String,
    pub created_by_user_id: Option<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

pub fn list_api_key_records(conn: &Connection) -> rusqlite::Result<Vec<ApiKeyRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, key_prefix, scopes_json, status, created_by_user_id,
           created_at, updated_at, last_used_at, revoked_at, expires_at
         FROM api_keys
         ORDER BY created_at DESC, id DESC",
    )?;
    let rows = stmt.query_map([], map_api_key_row)?;
    rows.collect()
}

pub fn list_api_key_records_at_path(db_path: &Path) -> rusqlite::Result<Vec<ApiKeyRecord>> {
    let conn = open_connection(db_path)?;
    list_api_key_records(&conn)
}

pub fn load_api_key_by_prefix_and_hash(
    conn: &Connection,
    key_prefix: &str,
    secret_hash: &str,
) -> rusqlite::Result<Option<ApiKeyRecord>> {
    conn.query_row(
        "SELECT id, name, key_prefix, scopes_json, status, created_by_user_id,
           created_at, updated_at, last_used_at, revoked_at, expires_at
         FROM api_keys
         WHERE key_prefix = ?1 AND secret_hash = ?2
         LIMIT 1",
        params![key_prefix, secret_hash],
        map_api_key_row,
    )
    .optional()
}

pub fn load_api_key_by_prefix_and_hash_at_path(
    db_path: &Path,
    key_prefix: &str,
    secret_hash: &str,
) -> rusqlite::Result<Option<ApiKeyRecord>> {
    let conn = open_connection(db_path)?;
    load_api_key_by_prefix_and_hash(&conn, key_prefix, secret_hash)
}

pub fn load_api_key_by_id(
    conn: &Connection,
    api_key_id: &str,
) -> rusqlite::Result<Option<ApiKeyRecord>> {
    conn.query_row(
        "SELECT id, name, key_prefix, scopes_json, status, created_by_user_id,
           created_at, updated_at, last_used_at, revoked_at, expires_at
         FROM api_keys
         WHERE id = ?1
         LIMIT 1",
        params![api_key_id],
        map_api_key_row,
    )
    .optional()
}

pub fn insert_api_key_record(
    conn: &Connection,
    input: ApiKeyRecordInput,
) -> rusqlite::Result<ApiKeyRecord> {
    let api_key_id = format!("api-key-{}", Uuid::new_v4());
    conn.execute(
        "INSERT INTO api_keys
           (id, name, key_prefix, secret_hash, scopes_json, status, created_by_user_id,
            created_at, updated_at, last_used_at, revoked_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, NULL, NULL, ?9)",
        params![
            api_key_id,
            input.name,
            input.key_prefix,
            input.secret_hash,
            input.scopes_json,
            input.status,
            input.created_by_user_id,
            input.created_at,
            input.expires_at
        ],
    )?;

    load_api_key_by_id(conn, &api_key_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn insert_api_key_record_at_path(
    db_path: &Path,
    input: ApiKeyRecordInput,
) -> rusqlite::Result<ApiKeyRecord> {
    let conn = open_connection(db_path)?;
    insert_api_key_record(&conn, input)
}

pub fn revoke_api_key_record(
    conn: &Connection,
    api_key_id: &str,
    revoked_at: i64,
) -> rusqlite::Result<Option<ApiKeyRecord>> {
    let Some(api_key) = load_api_key_by_id(conn, api_key_id)? else {
        return Ok(None);
    };

    if api_key.status != API_KEY_STATUS_REVOKED {
        conn.execute(
            "UPDATE api_keys
             SET status = ?1, revoked_at = ?2, updated_at = ?2
             WHERE id = ?3",
            params![API_KEY_STATUS_REVOKED, revoked_at, api_key_id],
        )?;
    }

    load_api_key_by_id(conn, api_key_id)
}

pub fn revoke_api_key_record_at_path(
    db_path: &Path,
    api_key_id: &str,
    revoked_at: i64,
) -> rusqlite::Result<Option<ApiKeyRecord>> {
    let conn = open_connection(db_path)?;
    revoke_api_key_record(&conn, api_key_id, revoked_at)
}

pub fn touch_api_key(
    conn: &Connection,
    api_key_id: &str,
    last_used_at: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE api_keys SET last_used_at = ?1 WHERE id = ?2",
        params![last_used_at, api_key_id],
    )?;
    Ok(())
}

pub fn touch_api_key_at_path(
    db_path: &Path,
    api_key_id: &str,
    last_used_at: i64,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    touch_api_key(&conn, api_key_id, last_used_at)
}

fn map_api_key_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApiKeyRecord> {
    Ok(ApiKeyRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        key_prefix: row.get(2)?,
        scopes_json: row.get(3)?,
        status: row.get(4)?,
        created_by_user_id: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        last_used_at: row.get(8)?,
        revoked_at: row.get(9)?,
        expires_at: row.get(10)?,
    })
}
