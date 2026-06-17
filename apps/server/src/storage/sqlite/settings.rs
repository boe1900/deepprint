use std::path::Path;

use rusqlite::OptionalExtension;

use super::open_connection;

const SETTINGS_KEY_CUPS_BASE_URL: &str = "cups_base_url";

pub fn load_cups_base_url(db_path: &Path) -> rusqlite::Result<Option<String>> {
    let conn = open_connection(db_path)?;
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [SETTINGS_KEY_CUPS_BASE_URL],
        |row| row.get::<_, String>(0),
    )
    .optional()
}

pub fn save_cups_base_url(
    db_path: &Path,
    cups_base_url: &str,
    updated_at: i64,
) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    conn.execute(
        "INSERT INTO app_settings(key, value, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET
           value = excluded.value,
           updated_at = excluded.updated_at",
        (SETTINGS_KEY_CUPS_BASE_URL, cups_base_url, updated_at),
    )?;
    Ok(())
}
