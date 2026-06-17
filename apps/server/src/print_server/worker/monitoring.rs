#[path = "monitoring/printing.rs"]
mod printing;
#[path = "monitoring/submitting.rs"]
mod submitting;
#[path = "monitoring/terminal.rs"]
mod terminal;

pub(super) use printing::monitor_printing_jobs;
pub(super) use submitting::monitor_submitting_jobs;
