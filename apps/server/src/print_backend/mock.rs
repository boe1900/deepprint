use super::{
    registry::PrinterDetail,
    types::{CopiesCapability, PrinterCapabilities, PrinterTargetInput, ValidatedPrinterTarget},
};

pub async fn validate_mock_target(target: &PrinterTargetInput) -> Option<ValidatedPrinterTarget> {
    let printer_uri = match target {
        PrinterTargetInput::Uri { uri } if uri.trim().eq_ignore_ascii_case("mock:printer") => {
            "mock:printer".to_string()
        }
        PrinterTargetInput::Host { host } if host.trim().eq_ignore_ascii_case("mock") => {
            "mock:printer".to_string()
        }
        _ => return None,
    };

    Some(ValidatedPrinterTarget {
        normalized_uri: printer_uri.clone(),
        printer_uri,
        discovered_name: "Mock Printer".to_string(),
        state: Some("idle".to_string()),
        state_message: None,
        capabilities: mock_capabilities(),
        attributes: serde_json::json!({
            "printer-name": "Mock Printer",
            "printer-state": "idle"
        }),
        already_managed: false,
        managed_printer_id: None,
    })
}

pub async fn get_mock_printer_detail(printer_uri: &str) -> Option<PrinterDetail> {
    if !printer_uri.trim().eq_ignore_ascii_case("mock:printer") {
        return None;
    }

    Some(PrinterDetail {
        id: String::new(),
        name: "Mock Printer".to_string(),
        uri: "mock:printer".to_string(),
        normalized_uri: "mock:printer".to_string(),
        source: "mock".to_string(),
        is_default: false,
        enabled: true,
        state: Some("idle".to_string()),
        state_message: None,
        capabilities: mock_capabilities(),
        attributes: serde_json::json!({
            "printer-name": "Mock Printer",
            "printer-state": "idle"
        }),
        last_seen_at: None,
        last_validated_at: None,
        last_refreshed_at: None,
        created_at: 0,
        updated_at: 0,
    })
}

fn mock_capabilities() -> PrinterCapabilities {
    PrinterCapabilities {
        document_formats: vec!["application/pdf".to_string()],
        media_supported: vec![
            "iso_a4_210x297mm".to_string(),
            "na_letter_8.5x11in".to_string(),
        ],
        media_default: Some("iso_a4_210x297mm".to_string()),
        media_types_supported: vec![
            "stationery".to_string(),
            "photographic".to_string(),
            "photographic-glossy".to_string(),
        ],
        sides_supported: vec!["one-sided".to_string(), "two-sided-long-edge".to_string()],
        sides_default: Some("one-sided".to_string()),
        copies: Some(CopiesCapability {
            default: Some(1),
            min: Some(1),
            max: Some(10),
        }),
        color_modes_supported: vec!["color".to_string(), "monochrome".to_string()],
        color_supported: Some(true),
        orientations_supported: vec!["portrait".to_string(), "landscape".to_string()],
        scalings_supported: vec![
            "auto".to_string(),
            "auto-fit".to_string(),
            "fit".to_string(),
            "fill".to_string(),
            "none".to_string(),
        ],
        supports_page_ranges: Some(true),
        job_creation_attributes_supported: vec![
            "copies".to_string(),
            "media".to_string(),
            "media-type".to_string(),
            "sides".to_string(),
            "print-color-mode".to_string(),
            "orientation-requested".to_string(),
            "print-scaling".to_string(),
            "page-ranges".to_string(),
        ],
    }
}
