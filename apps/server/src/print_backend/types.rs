use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CopiesCapability {
    pub default: Option<u16>,
    pub min: Option<u16>,
    pub max: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PrinterCapabilities {
    pub document_formats: Vec<String>,
    pub media_supported: Vec<String>,
    pub media_default: Option<String>,
    pub media_types_supported: Vec<String>,
    pub sides_supported: Vec<String>,
    pub sides_default: Option<String>,
    pub copies: Option<CopiesCapability>,
    pub color_modes_supported: Vec<String>,
    pub color_supported: Option<bool>,
    pub orientations_supported: Vec<String>,
    pub scalings_supported: Vec<String>,
    pub supports_page_ranges: Option<bool>,
    pub job_creation_attributes_supported: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum PrinterTargetInput {
    Uri { uri: String },
    Host { host: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidatePrinterRequest {
    #[serde(flatten)]
    pub target: PrinterTargetInput,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidatedPrinterTarget {
    pub normalized_uri: String,
    pub printer_uri: String,
    pub discovered_name: String,
    pub state: Option<String>,
    pub state_message: Option<String>,
    pub capabilities: PrinterCapabilities,
    pub attributes: Value,
    pub already_managed: bool,
    pub managed_printer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidatePrinter {
    pub display_name: String,
    pub candidate_uri: String,
    pub source: String,
    pub is_already_managed: bool,
    pub managed_printer_id: Option<String>,
}

pub type DiscoveredPrinter = CandidatePrinter;

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredPrintersResponse {
    pub printers: Vec<DiscoveredPrinter>,
    pub cups_base_url: Option<String>,
    pub reachable: Option<bool>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrintersListResponse<T> {
    pub printers: Vec<T>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeletePrinterResponse {
    pub deleted: bool,
}
