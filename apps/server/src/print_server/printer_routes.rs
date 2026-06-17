#[path = "printer_routes/create.rs"]
mod create;
#[path = "printer_routes/discovery.rs"]
mod discovery;
#[path = "printer_routes/mutations.rs"]
mod mutations;
#[path = "printer_routes/queries.rs"]
mod queries;
#[path = "printer_routes/validation.rs"]
mod validation;

pub(super) use create::create_printer;
pub(super) use discovery::{discover_cups_printers, discover_mdns_printers};
pub(super) use mutations::{delete_printer, disable_printer, enable_printer, set_default_printer};
pub(super) use queries::{get_printer_detail, list_printers};
pub(super) use validation::{refresh_printer, validate_printer};
