use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{error::PrintBackendError, types::PrinterCapabilities};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrinterSource {
    Manual,
    CupsImport,
    Mdns,
}

impl PrinterSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::CupsImport => "cups_import",
            Self::Mdns => "mdns",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ManagedPrinterRecord {
    pub id: String,
    pub source: String,
    pub display_name: String,
    pub printer_uri: String,
    pub normalized_uri: String,
    pub is_default: bool,
    pub is_enabled: bool,
    pub last_known_state: Option<String>,
    pub last_state_message: Option<String>,
    pub capabilities_json: Option<String>,
    pub attributes_json: Option<String>,
    pub last_seen_at: Option<i64>,
    pub last_validated_at: Option<i64>,
    pub last_refreshed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrinterSummary {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub source: String,
    pub is_default: bool,
    pub enabled: bool,
    pub state: Option<String>,
    pub state_message: Option<String>,
    pub last_validated_at: Option<i64>,
    pub last_seen_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrinterDetail {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub normalized_uri: String,
    pub source: String,
    pub is_default: bool,
    pub enabled: bool,
    pub state: Option<String>,
    pub state_message: Option<String>,
    pub capabilities: PrinterCapabilities,
    pub attributes: Value,
    pub last_seen_at: Option<i64>,
    pub last_validated_at: Option<i64>,
    pub last_refreshed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddPrinterRequest {
    pub source: PrinterSource,
    pub printer_uri: String,
    #[serde(default)]
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddPrinterResponse {
    pub printer: PrinterSummary,
    pub created: bool,
}

#[derive(Debug, Clone)]
pub struct CreatePrinterRecord {
    pub id: String,
    pub source: PrinterSource,
    pub display_name: String,
    pub printer_uri: String,
    pub normalized_uri: String,
    pub last_known_state: Option<String>,
    pub last_state_message: Option<String>,
    pub capabilities: PrinterCapabilities,
    pub attributes: Value,
    pub last_seen_at: Option<i64>,
    pub last_validated_at: Option<i64>,
    pub last_refreshed_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RefreshPrinterSnapshotInput {
    pub printer_uri: String,
    pub normalized_uri: String,
    pub last_known_state: Option<String>,
    pub last_state_message: Option<String>,
    pub capabilities: PrinterCapabilities,
    pub attributes: Value,
    pub last_seen_at: Option<i64>,
    pub last_validated_at: Option<i64>,
    pub last_refreshed_at: Option<i64>,
}

pub fn list_printers(conn: &Connection) -> Result<Vec<PrinterSummary>, PrintBackendError> {
    let mut stmt = conn.prepare(
        "SELECT
            id,
            source,
            display_name,
            printer_uri,
            normalized_uri,
            is_default,
            is_enabled,
            last_known_state,
            last_state_message,
            capabilities_json,
            attributes_json,
            last_seen_at,
            last_validated_at,
            last_refreshed_at,
            created_at,
            updated_at
         FROM printers
         ORDER BY is_default DESC, display_name COLLATE NOCASE ASC, created_at ASC",
    )?;

    let rows = stmt.query_map([], map_managed_printer_row)?;
    let mut printers = Vec::new();
    for row in rows {
        printers.push(to_printer_summary(row?)?);
    }
    Ok(printers)
}

pub fn get_printer_by_id(
    conn: &Connection,
    printer_id: &str,
) -> Result<Option<ManagedPrinterRecord>, PrintBackendError> {
    conn.query_row(
        "SELECT
            id,
            source,
            display_name,
            printer_uri,
            normalized_uri,
            is_default,
            is_enabled,
            last_known_state,
            last_state_message,
            capabilities_json,
            attributes_json,
            last_seen_at,
            last_validated_at,
            last_refreshed_at,
            created_at,
            updated_at
         FROM printers
         WHERE id = ?1",
        params![printer_id],
        map_managed_printer_row,
    )
    .optional()
    .map_err(PrintBackendError::from)
}

pub fn get_printer_by_normalized_uri(
    conn: &Connection,
    normalized_uri: &str,
) -> Result<Option<ManagedPrinterRecord>, PrintBackendError> {
    conn.query_row(
        "SELECT
            id,
            source,
            display_name,
            printer_uri,
            normalized_uri,
            is_default,
            is_enabled,
            last_known_state,
            last_state_message,
            capabilities_json,
            attributes_json,
            last_seen_at,
            last_validated_at,
            last_refreshed_at,
            created_at,
            updated_at
         FROM printers
         WHERE normalized_uri = ?1",
        params![normalized_uri],
        map_managed_printer_row,
    )
    .optional()
    .map_err(PrintBackendError::from)
}

pub fn create_printer(
    conn: &Connection,
    input: &CreatePrinterRecord,
    now: i64,
) -> Result<ManagedPrinterRecord, PrintBackendError> {
    let is_default = if has_any_printer(conn)? { 0 } else { 1 };
    conn.execute(
        "INSERT INTO printers (
            id,
            source,
            display_name,
            printer_uri,
            normalized_uri,
            is_default,
            is_enabled,
            last_known_state,
            last_state_message,
            capabilities_json,
            attributes_json,
            last_seen_at,
            last_validated_at,
            last_refreshed_at,
            created_at,
            updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
        params![
            input.id,
            input.source.as_str(),
            input.display_name,
            input.printer_uri,
            input.normalized_uri,
            is_default,
            input.last_known_state,
            input.last_state_message,
            serde_json::to_string(&input.capabilities)?,
            serde_json::to_string(&input.attributes)?,
            input.last_seen_at,
            input.last_validated_at,
            input.last_refreshed_at,
            now,
        ],
    )?;

    get_printer_by_id(conn, &input.id)?
        .ok_or_else(|| PrintBackendError::backend("created printer not found after insert"))
}

pub fn to_printer_summary(
    record: ManagedPrinterRecord,
) -> Result<PrinterSummary, PrintBackendError> {
    Ok(PrinterSummary {
        id: record.id,
        name: record.display_name,
        uri: record.printer_uri,
        source: record.source,
        is_default: record.is_default,
        enabled: record.is_enabled,
        state: record.last_known_state,
        state_message: record.last_state_message,
        last_validated_at: record.last_validated_at,
        last_seen_at: record.last_seen_at,
    })
}

pub fn to_printer_detail(record: ManagedPrinterRecord) -> Result<PrinterDetail, PrintBackendError> {
    let capabilities = parse_capabilities_or_default(record.capabilities_json.as_deref());
    let attributes = parse_json_or_default(record.attributes_json.as_deref());
    Ok(PrinterDetail {
        id: record.id,
        name: record.display_name,
        uri: record.printer_uri,
        normalized_uri: record.normalized_uri,
        source: record.source,
        is_default: record.is_default,
        enabled: record.is_enabled,
        state: record.last_known_state,
        state_message: record.last_state_message,
        capabilities,
        attributes,
        last_seen_at: record.last_seen_at,
        last_validated_at: record.last_validated_at,
        last_refreshed_at: record.last_refreshed_at,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

pub fn update_printer_refresh_snapshot(
    conn: &Connection,
    printer_id: &str,
    input: &RefreshPrinterSnapshotInput,
    now: i64,
) -> Result<Option<ManagedPrinterRecord>, PrintBackendError> {
    let changed = conn.execute(
        "UPDATE printers
         SET printer_uri = ?1,
             normalized_uri = ?2,
             last_known_state = ?3,
             last_state_message = ?4,
             capabilities_json = ?5,
             attributes_json = ?6,
             last_seen_at = ?7,
             last_validated_at = ?8,
             last_refreshed_at = ?9,
             updated_at = ?10
         WHERE id = ?11",
        params![
            input.printer_uri,
            input.normalized_uri,
            input.last_known_state,
            input.last_state_message,
            serde_json::to_string(&input.capabilities)?,
            serde_json::to_string(&input.attributes)?,
            input.last_seen_at,
            input.last_validated_at,
            input.last_refreshed_at,
            now,
            printer_id,
        ],
    )?;

    if changed == 0 {
        return Ok(None);
    }

    get_printer_by_id(conn, printer_id)
}

pub fn enable_printer(
    conn: &Connection,
    printer_id: &str,
    now: i64,
) -> Result<Option<ManagedPrinterRecord>, PrintBackendError> {
    let changed = conn.execute(
        "UPDATE printers
         SET is_enabled = 1,
             updated_at = ?1
         WHERE id = ?2",
        params![now, printer_id],
    )?;

    if changed == 0 {
        return Ok(None);
    }

    get_printer_by_id(conn, printer_id)
}

pub fn disable_printer(
    conn: &Connection,
    printer_id: &str,
    now: i64,
) -> Result<Option<ManagedPrinterRecord>, PrintBackendError> {
    let is_default: Option<i64> = conn
        .query_row(
            "SELECT is_default FROM printers WHERE id = ?1",
            params![printer_id],
            |row| row.get(0),
        )
        .optional()?;

    let Some(is_default) = is_default else {
        return Ok(None);
    };

    if is_default != 0 {
        return Err(PrintBackendError::conflict(
            "default printer cannot be disabled",
        ));
    }

    conn.execute(
        "UPDATE printers
         SET is_enabled = 0,
             updated_at = ?1
         WHERE id = ?2",
        params![now, printer_id],
    )?;

    get_printer_by_id(conn, printer_id)
}

pub fn set_default_printer(
    conn: &Connection,
    printer_id: &str,
    now: i64,
) -> Result<Option<ManagedPrinterRecord>, PrintBackendError> {
    let target_enabled: Option<i64> = conn
        .query_row(
            "SELECT is_enabled FROM printers WHERE id = ?1",
            params![printer_id],
            |row| row.get(0),
        )
        .optional()?;

    let Some(target_enabled) = target_enabled else {
        return Ok(None);
    };

    if target_enabled == 0 {
        return Err(PrintBackendError::conflict(
            "disabled printer cannot be set as default",
        ));
    }

    conn.execute(
        "UPDATE printers SET is_default = 0, updated_at = ?1",
        params![now],
    )?;
    conn.execute(
        "UPDATE printers
         SET is_default = 1,
             updated_at = ?1
         WHERE id = ?2",
        params![now, printer_id],
    )?;

    get_printer_by_id(conn, printer_id)
}

pub fn delete_printer(
    conn: &Connection,
    printer_id: &str,
    now: i64,
) -> Result<bool, PrintBackendError> {
    let record = get_printer_by_id(conn, printer_id)?;
    let Some(record) = record else {
        return Ok(false);
    };

    let changed = conn.execute("DELETE FROM printers WHERE id = ?1", params![printer_id])?;
    if changed == 0 {
        return Ok(false);
    }

    if record.is_default {
        if let Some(next_default_id) = select_next_default_printer_id(conn)? {
            conn.execute(
                "UPDATE printers
                 SET is_default = 1,
                     updated_at = ?1
                 WHERE id = ?2",
                params![now, next_default_id],
            )?;
        }
    }

    Ok(true)
}

fn has_any_printer(conn: &Connection) -> Result<bool, PrintBackendError> {
    let count: i64 = conn.query_row("SELECT COUNT(1) FROM printers", [], |row| row.get(0))?;
    Ok(count > 0)
}

fn select_next_default_printer_id(conn: &Connection) -> Result<Option<String>, PrintBackendError> {
    conn.query_row(
        "SELECT id
         FROM printers
         WHERE is_enabled = 1
         ORDER BY created_at ASC
         LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(PrintBackendError::from)
}

fn parse_json_or_default(raw: Option<&str>) -> Value {
    raw.and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn parse_capabilities_or_default(raw: Option<&str>) -> PrinterCapabilities {
    raw.and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or_default()
}

fn map_managed_printer_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ManagedPrinterRecord> {
    Ok(ManagedPrinterRecord {
        id: row.get(0)?,
        source: row.get(1)?,
        display_name: row.get(2)?,
        printer_uri: row.get(3)?,
        normalized_uri: row.get(4)?,
        is_default: row.get::<_, i64>(5)? != 0,
        is_enabled: row.get::<_, i64>(6)? != 0,
        last_known_state: row.get(7)?,
        last_state_message: row.get(8)?,
        capabilities_json: row.get(9)?,
        attributes_json: row.get(10)?,
        last_seen_at: row.get(11)?,
        last_validated_at: row.get(12)?,
        last_refreshed_at: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}
