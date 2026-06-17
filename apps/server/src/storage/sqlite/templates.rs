use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use super::open_connection;

#[derive(Debug, Clone)]
pub struct TemplateGroupRecord {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct TemplateRecordRow {
    pub id: String,
    pub group_id: String,
    pub name: String,
    pub description: String,
    pub output_name: String,
    pub typst_code: String,
    pub sample_data: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

pub type TemplateRecordInput = (String, String, String, String, String, String);

pub fn list_template_groups_at_path(db_path: &Path) -> rusqlite::Result<Vec<TemplateGroupRecord>> {
    let conn = open_connection(db_path)?;
    list_template_groups(&conn)
}

pub fn list_template_groups(conn: &Connection) -> rusqlite::Result<Vec<TemplateGroupRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, sort_order, created_at, updated_at
         FROM template_groups
         ORDER BY sort_order ASC, created_at ASC, id ASC",
    )?;
    let rows = stmt.query_map([], map_template_group_row)?;
    rows.collect()
}

pub fn list_templates_at_path(db_path: &Path) -> rusqlite::Result<Vec<TemplateRecordRow>> {
    let conn = open_connection(db_path)?;
    list_templates(&conn)
}

pub fn list_templates(conn: &Connection) -> rusqlite::Result<Vec<TemplateRecordRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, group_id, name, description, output_name, typst_code, sample_data,
                sort_order, created_at, updated_at
         FROM templates
         ORDER BY group_id ASC, sort_order ASC, created_at ASC, id ASC",
    )?;
    let rows = stmt.query_map([], map_template_row)?;
    rows.collect()
}

pub fn list_templates_by_group(
    conn: &Connection,
    group_id: &str,
) -> rusqlite::Result<Vec<TemplateRecordRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, group_id, name, description, output_name, typst_code, sample_data,
                sort_order, created_at, updated_at
         FROM templates
         WHERE group_id = ?1
         ORDER BY sort_order ASC, created_at ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![group_id], map_template_row)?;
    rows.collect()
}

pub fn list_templates_by_group_at_path(
    db_path: &Path,
    group_id: &str,
) -> rusqlite::Result<Vec<TemplateRecordRow>> {
    let conn = open_connection(db_path)?;
    list_templates_by_group(&conn, group_id)
}

pub fn fetch_template_by_id(
    conn: &Connection,
    template_id: &str,
) -> rusqlite::Result<Option<TemplateRecordRow>> {
    conn.query_row(
        "SELECT id, group_id, name, description, output_name, typst_code, sample_data,
                sort_order, created_at, updated_at
         FROM templates
         WHERE id = ?1",
        params![template_id],
        map_template_row,
    )
    .optional()
}

pub fn fetch_template_by_id_at_path(
    db_path: &Path,
    template_id: &str,
) -> rusqlite::Result<Option<TemplateRecordRow>> {
    let conn = open_connection(db_path)?;
    fetch_template_by_id(&conn, template_id)
}

pub fn insert_template_group(
    conn: &Connection,
    name: &str,
) -> rusqlite::Result<TemplateGroupRecord> {
    let now = now_unix();
    let sort_order: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM template_groups",
        [],
        |row| row.get(0),
    )?;
    let group_id = format!("group-{}", Uuid::new_v4());
    let normalized_name = name.trim();
    conn.execute(
        "INSERT INTO template_groups (id, name, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![group_id, normalized_name, sort_order, now],
    )?;
    fetch_template_group_by_id(conn, &group_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn insert_template_group_at_path(
    db_path: &Path,
    name: &str,
) -> rusqlite::Result<TemplateGroupRecord> {
    let conn = open_connection(db_path)?;
    insert_template_group(&conn, name)
}

pub fn update_template_group_record(
    conn: &Connection,
    group_id: &str,
    name: &str,
) -> rusqlite::Result<Option<TemplateGroupRecord>> {
    let now = now_unix();
    let changed = conn.execute(
        "UPDATE template_groups
         SET name = ?1,
             updated_at = ?2
         WHERE id = ?3",
        params![name.trim(), now, group_id],
    )?;
    if changed == 0 {
        return Ok(None);
    }
    fetch_template_group_by_id(conn, group_id)
}

pub fn update_template_group_record_at_path(
    db_path: &Path,
    group_id: &str,
    name: &str,
) -> rusqlite::Result<Option<TemplateGroupRecord>> {
    let conn = open_connection(db_path)?;
    update_template_group_record(&conn, group_id, name)
}

pub fn count_templates_in_group(conn: &Connection, group_id: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(1) FROM templates WHERE group_id = ?1",
        params![group_id],
        |row| row.get(0),
    )
}

pub fn count_templates_in_group_at_path(db_path: &Path, group_id: &str) -> rusqlite::Result<i64> {
    let conn = open_connection(db_path)?;
    count_templates_in_group(&conn, group_id)
}

pub fn delete_template_group_by_id(conn: &Connection, group_id: &str) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "DELETE FROM template_groups WHERE id = ?1",
        params![group_id],
    )?;
    Ok(changed == 1)
}

pub fn delete_template_group_by_id_at_path(
    db_path: &Path,
    group_id: &str,
) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    delete_template_group_by_id(&conn, group_id)
}

pub fn insert_template_record(
    conn: &Connection,
    input: TemplateRecordInput,
) -> rusqlite::Result<TemplateRecordRow> {
    let (group_id, name, description, output_name, typst_code, sample_data) = input;
    ensure_template_group_exists(conn, &group_id)?;

    let now = now_unix();
    let sort_order: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM templates WHERE group_id = ?1",
        params![group_id],
        |row| row.get(0),
    )?;
    let template_id = format!("template-{}", Uuid::new_v4());
    conn.execute(
        "INSERT INTO templates (
           id, group_id, name, description, output_name, typst_code, sample_data,
           sort_order, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            template_id,
            group_id,
            name,
            description,
            output_name,
            typst_code,
            sample_data,
            sort_order,
            now,
        ],
    )?;
    fetch_template_by_id(conn, &template_id)
        .map(|value| value.expect("template should exist after insert"))
}

pub fn insert_template_record_at_path(
    db_path: &Path,
    input: TemplateRecordInput,
) -> rusqlite::Result<TemplateRecordRow> {
    let conn = open_connection(db_path)?;
    insert_template_record(&conn, input)
}

pub fn update_template_record(
    conn: &Connection,
    template_id: &str,
    input: TemplateRecordInput,
) -> rusqlite::Result<Option<TemplateRecordRow>> {
    let (group_id, name, description, output_name, typst_code, sample_data) = input;
    ensure_template_group_exists(conn, &group_id)?;
    let now = now_unix();
    let changed = conn.execute(
        "UPDATE templates
         SET group_id = ?1,
             name = ?2,
             description = ?3,
             output_name = ?4,
             typst_code = ?5,
             sample_data = ?6,
             updated_at = ?7
         WHERE id = ?8",
        params![
            group_id,
            name,
            description,
            output_name,
            typst_code,
            sample_data,
            now,
            template_id,
        ],
    )?;
    if changed == 0 {
        return Ok(None);
    }
    fetch_template_by_id(conn, template_id)
}

pub fn update_template_record_at_path(
    db_path: &Path,
    template_id: &str,
    input: TemplateRecordInput,
) -> rusqlite::Result<Option<TemplateRecordRow>> {
    let conn = open_connection(db_path)?;
    update_template_record(&conn, template_id, input)
}

pub fn delete_template_record(conn: &Connection, template_id: &str) -> rusqlite::Result<bool> {
    let changed = conn.execute("DELETE FROM templates WHERE id = ?1", params![template_id])?;
    Ok(changed == 1)
}

pub fn delete_template_record_at_path(db_path: &Path, template_id: &str) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    delete_template_record(&conn, template_id)
}

fn fetch_template_group_by_id(
    conn: &Connection,
    group_id: &str,
) -> rusqlite::Result<Option<TemplateGroupRecord>> {
    conn.query_row(
        "SELECT id, name, sort_order, created_at, updated_at
         FROM template_groups
         WHERE id = ?1",
        params![group_id],
        map_template_group_row,
    )
    .optional()
}

fn ensure_template_group_exists(conn: &Connection, group_id: &str) -> rusqlite::Result<()> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM template_groups WHERE id = ?1",
            params![group_id],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

fn map_template_group_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TemplateGroupRecord> {
    Ok(TemplateGroupRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        sort_order: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn map_template_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TemplateRecordRow> {
    Ok(TemplateRecordRow {
        id: row.get(0)?,
        group_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        output_name: row.get(4)?,
        typst_code: row.get(5)?,
        sample_data: row.get(6)?,
        sort_order: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
