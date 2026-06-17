use std::path::Path;

use crate::print_backend::{
    registry as printer_registry, types::DiscoveredPrinter, CreatePrinterRecord, PrintBackendError,
    PrinterDetail, PrinterSummary, RefreshPrinterSnapshotInput,
};

use super::open_connection;

pub fn load_printer_by_normalized_uri(
    db_path: &Path,
    normalized_uri: &str,
) -> Result<Option<PrinterDetail>, PrintBackendError> {
    let conn = open_connection(db_path)?;
    let record = printer_registry::get_printer_by_normalized_uri(&conn, normalized_uri)?;
    record.map(printer_registry::to_printer_detail).transpose()
}

pub fn list_printer_summaries(db_path: &Path) -> Result<Vec<PrinterSummary>, PrintBackendError> {
    let conn = open_connection(db_path)?;
    printer_registry::list_printers(&conn)
}

pub fn create_printer_record(
    db_path: &Path,
    input: &CreatePrinterRecord,
    now: i64,
) -> Result<PrinterSummary, PrintBackendError> {
    let conn = open_connection(db_path)?;
    let created = printer_registry::create_printer(&conn, input, now)?;
    printer_registry::to_printer_summary(created)
}

pub fn load_printer_detail_by_id(
    db_path: &Path,
    printer_id: &str,
) -> Result<Option<PrinterDetail>, PrintBackendError> {
    let conn = open_connection(db_path)?;
    let record = printer_registry::get_printer_by_id(&conn, printer_id)?;
    record.map(printer_registry::to_printer_detail).transpose()
}

pub fn refresh_printer_snapshot(
    db_path: &Path,
    printer_id: &str,
    input: &RefreshPrinterSnapshotInput,
    now: i64,
) -> Result<Option<PrinterDetail>, PrintBackendError> {
    let conn = open_connection(db_path)?;
    let updated = printer_registry::update_printer_refresh_snapshot(&conn, printer_id, input, now)?;
    updated.map(printer_registry::to_printer_detail).transpose()
}

pub fn enable_printer_record(
    db_path: &Path,
    printer_id: &str,
    now: i64,
) -> Result<Option<PrinterDetail>, PrintBackendError> {
    let conn = open_connection(db_path)?;
    let updated = printer_registry::enable_printer(&conn, printer_id, now)?;
    updated.map(printer_registry::to_printer_detail).transpose()
}

pub fn disable_printer_record(
    db_path: &Path,
    printer_id: &str,
    now: i64,
) -> Result<Option<PrinterDetail>, PrintBackendError> {
    let conn = open_connection(db_path)?;
    let updated = printer_registry::disable_printer(&conn, printer_id, now)?;
    updated.map(printer_registry::to_printer_detail).transpose()
}

pub fn set_default_printer_record(
    db_path: &Path,
    printer_id: &str,
    now: i64,
) -> Result<Option<PrinterDetail>, PrintBackendError> {
    let conn = open_connection(db_path)?;
    let updated = printer_registry::set_default_printer(&conn, printer_id, now)?;
    updated.map(printer_registry::to_printer_detail).transpose()
}

pub fn delete_printer_record(
    db_path: &Path,
    printer_id: &str,
    now: i64,
) -> Result<bool, PrintBackendError> {
    let conn = open_connection(db_path)?;
    printer_registry::delete_printer(&conn, printer_id, now)
}

pub fn mark_managed_discovered_printers(
    db_path: &Path,
    printers: &mut [DiscoveredPrinter],
) -> Result<(), PrintBackendError> {
    let conn = open_connection(db_path)?;
    for printer in printers {
        if let Some(existing) =
            printer_registry::get_printer_by_normalized_uri(&conn, &printer.candidate_uri)?
        {
            printer.is_already_managed = true;
            printer.managed_printer_id = Some(existing.id);
        }
    }
    Ok(())
}
