use serde_json::Value;

use crate::printer::{PrintColorMode, PrintOptions};

use super::{
    error::PrintBackendError,
    types::{CopiesCapability, PrinterCapabilities},
};

pub fn extract_capabilities(attributes: &Value) -> PrinterCapabilities {
    let job_creation_attributes_supported =
        get_string_list(attributes, "job-creation-attributes-supported");

    PrinterCapabilities {
        document_formats: get_string_list(attributes, "document-format-supported"),
        media_supported: get_string_list(attributes, "media-supported"),
        media_default: get_string(attributes, "media-default"),
        media_types_supported: get_string_list(attributes, "media-type-supported"),
        sides_supported: get_string_list(attributes, "sides-supported"),
        sides_default: get_string(attributes, "sides-default"),
        copies: extract_copies_capability(attributes),
        color_modes_supported: get_string_list(attributes, "print-color-mode-supported"),
        color_supported: attributes.get("color-supported").and_then(Value::as_bool),
        orientations_supported: normalize_orientation_supported(
            get_i32_list(attributes, "orientation-requested-supported"),
            &job_creation_attributes_supported,
        ),
        scalings_supported: normalize_scaling_supported(
            get_string_list(attributes, "print-scaling-supported"),
            &job_creation_attributes_supported,
        ),
        supports_page_ranges: normalize_attr_support(
            attributes,
            "page-ranges-supported",
            "page-ranges",
            &job_creation_attributes_supported,
        ),
        job_creation_attributes_supported,
    }
}

pub fn validate_print_options_against_capabilities(
    capabilities: &PrinterCapabilities,
    options: &PrintOptions,
) -> Result<(), PrintBackendError> {
    if let Some(copies) = options.copies {
        let Some(copies_cap) = &capabilities.copies else {
            return Err(PrintBackendError::printer_capability_unknown(
                "copies",
                Some(copies.to_string()),
            ));
        };

        if let Some(min) = copies_cap.min {
            if copies < min {
                return Err(PrintBackendError::print_option_invalid_for_printer(
                    "copies",
                    copies.to_string(),
                    "below_minimum",
                    Some(min),
                ));
            }
        }
        if let Some(max) = copies_cap.max {
            if copies > max {
                return Err(PrintBackendError::print_option_invalid_for_printer(
                    "copies",
                    copies.to_string(),
                    "above_maximum",
                    Some(max),
                ));
            }
        }
    }

    if let Some(sides) = options.sides {
        require_supported_keyword(
            "sides",
            sides.as_ipp_keyword(),
            &capabilities.sides_supported,
        )?;
    }

    if let Some(color_mode) = options.print_color_mode {
        if !capabilities.color_modes_supported.is_empty() {
            require_supported_keyword(
                "printColorMode",
                color_mode.as_ipp_keyword(),
                &capabilities.color_modes_supported,
            )?;
        } else if matches!(color_mode, PrintColorMode::Color)
            && matches!(capabilities.color_supported, Some(false))
        {
            return Err(PrintBackendError::print_option_unsupported(
                "printColorMode",
                Some("color".to_string()),
                vec!["monochrome".to_string()],
            ));
        } else {
            return Err(PrintBackendError::printer_capability_unknown(
                "printColorMode",
                Some(color_mode.as_ipp_keyword().to_string()),
            ));
        }
    }

    if let Some(media) = options
        .media
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        require_supported_keyword("media", media, &capabilities.media_supported)?;
    }

    if let Some(media_type) = options
        .media_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        require_supported_keyword("mediaType", media_type, &capabilities.media_types_supported)?;
    }

    if let Some(orientation) = options.orientation_requested {
        require_supported_keyword(
            "orientationRequested",
            orientation.as_capability_keyword(),
            &capabilities.orientations_supported,
        )?;
    }

    if let Some(scaling) = options.print_scaling {
        require_supported_keyword(
            "printScaling",
            scaling.as_ipp_keyword(),
            &capabilities.scalings_supported,
        )?;
    }

    if options
        .page_ranges
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        match capabilities.supports_page_ranges {
            Some(true) => {}
            Some(false) => {
                return Err(PrintBackendError::print_option_unsupported(
                    "pageRanges",
                    options
                        .page_ranges
                        .as_ref()
                        .map(|value| value.trim().to_string()),
                    Vec::new(),
                ));
            }
            None => {
                return Err(PrintBackendError::printer_capability_unknown(
                    "pageRanges",
                    options
                        .page_ranges
                        .as_ref()
                        .map(|value| value.trim().to_string()),
                ));
            }
        }
    }

    Ok(())
}

fn require_supported_keyword(
    option_name: &str,
    keyword: &str,
    supported: &[String],
) -> Result<(), PrintBackendError> {
    if supported.is_empty() {
        return Err(PrintBackendError::printer_capability_unknown(
            option_name,
            Some(keyword.to_string()),
        ));
    }

    if supported
        .iter()
        .any(|value| value.eq_ignore_ascii_case(keyword))
    {
        return Ok(());
    }

    Err(PrintBackendError::print_option_unsupported(
        option_name,
        Some(keyword.to_string()),
        supported.to_vec(),
    ))
}

fn extract_copies_capability(attributes: &Value) -> Option<CopiesCapability> {
    let default = attributes
        .get("copies-default")
        .and_then(Value::as_i64)
        .and_then(|value| u16::try_from(value).ok());
    let (min, max) = attributes
        .get("copies-supported")
        .and_then(Value::as_array)
        .and_then(|items| {
            let first = items.first()?;
            Some(if first.is_object() {
                (
                    first
                        .get("min")?
                        .as_u64()
                        .and_then(|value| u16::try_from(value).ok()),
                    first
                        .get("max")?
                        .as_u64()
                        .and_then(|value| u16::try_from(value).ok()),
                )
            } else {
                (
                    items
                        .first()?
                        .as_u64()
                        .and_then(|value| u16::try_from(value).ok()),
                    items
                        .get(1)?
                        .as_u64()
                        .and_then(|value| u16::try_from(value).ok()),
                )
            })
        })
        .unwrap_or((None, None));

    if default.is_none() && min.is_none() && max.is_none() {
        None
    } else {
        Some(CopiesCapability { default, min, max })
    }
}

fn normalize_orientation_supported(
    supported: Vec<i32>,
    job_creation_attributes_supported: &[String],
) -> Vec<String> {
    if !supported.is_empty() {
        let mut output = Vec::new();
        for value in supported {
            match value {
                3 => output.push("portrait".to_string()),
                4 => output.push("landscape".to_string()),
                _ => {}
            }
        }
        if !output.iter().any(|value| value == "portrait") {
            output.push("portrait".to_string());
        }
        return dedup_strings(output);
    }

    if job_creation_attributes_supported
        .iter()
        .any(|value| value.eq_ignore_ascii_case("orientation-requested"))
    {
        return vec!["portrait".to_string(), "landscape".to_string()];
    }

    Vec::new()
}

fn normalize_scaling_supported(
    supported: Vec<String>,
    job_creation_attributes_supported: &[String],
) -> Vec<String> {
    if !supported.is_empty() {
        return dedup_strings(supported);
    }

    if job_creation_attributes_supported
        .iter()
        .any(|value| value.eq_ignore_ascii_case("print-scaling"))
    {
        return vec![
            "auto".to_string(),
            "auto-fit".to_string(),
            "fit".to_string(),
            "fill".to_string(),
            "none".to_string(),
        ];
    }

    Vec::new()
}

fn normalize_attr_support(
    attributes: &Value,
    explicit_key: &str,
    attr_name: &str,
    job_creation_attributes_supported: &[String],
) -> Option<bool> {
    if let Some(value) = attributes.get(explicit_key).and_then(Value::as_bool) {
        return Some(value);
    }

    if job_creation_attributes_supported
        .iter()
        .any(|value| value.eq_ignore_ascii_case(attr_name))
    {
        return Some(true);
    }

    None
}

fn get_string(attributes: &Value, key: &str) -> Option<String> {
    match attributes.get(key) {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Array(values)) => values.iter().find_map(|value| match value {
            Value::String(text) => Some(text.clone()),
            _ => None,
        }),
        _ => None,
    }
}

fn get_string_list(attributes: &Value, key: &str) -> Vec<String> {
    match attributes.get(key) {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Array(values)) => dedup_strings(
            values
                .iter()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .collect(),
        ),
        _ => Vec::new(),
    }
}

fn get_i32_list(attributes: &Value, key: &str) -> Vec<i32> {
    match attributes.get(key) {
        Some(Value::Number(value)) => value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .into_iter()
            .collect(),
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(|value| value.as_i64().and_then(|value| i32::try_from(value).ok()))
            .collect(),
        _ => Vec::new(),
    }
}

fn dedup_strings(values: Vec<String>) -> Vec<String> {
    let mut output = Vec::new();
    for value in values {
        if !output
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&value))
        {
            output.push(value);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::printer::{OrientationRequested, PrintColorMode, SidesMode};

    #[test]
    fn extract_capabilities_builds_normalized_profile() {
        let attributes = serde_json::json!({
            "document-format-supported": ["application/pdf"],
            "media-supported": ["iso_a4_210x297mm", "na_letter_8.5x11in"],
            "media-default": "iso_a4_210x297mm",
            "media-type-supported": ["stationery", "photographic-glossy"],
            "sides-supported": ["one-sided", "two-sided-long-edge"],
            "sides-default": "one-sided",
            "copies-supported": [{"min": 1, "max": 3}],
            "copies-default": 1,
            "print-color-mode-supported": ["color", "monochrome"],
            "color-supported": true,
            "orientation-requested-supported": [3, 4],
            "print-scaling-supported": ["auto", "fit"],
            "job-creation-attributes-supported": ["page-ranges"]
        });

        let capabilities = extract_capabilities(&attributes);
        assert_eq!(
            capabilities.media_default.as_deref(),
            Some("iso_a4_210x297mm")
        );
        assert_eq!(
            capabilities.copies.as_ref().and_then(|value| value.max),
            Some(3)
        );
        assert!(capabilities.supports_page_ranges.is_some_and(|value| value));
    }

    #[test]
    fn validate_print_options_against_capabilities_rejects_unsupported_option() {
        let capabilities = PrinterCapabilities {
            sides_supported: vec!["one-sided".to_string()],
            ..PrinterCapabilities::default()
        };
        let err = validate_print_options_against_capabilities(
            &capabilities,
            &PrintOptions {
                sides: Some(SidesMode::TwoSidedLongEdge),
                ..PrintOptions::default()
            },
        )
        .expect_err("unsupported duplex should fail");
        assert_eq!(
            err.api_code(),
            Some("PRINT_OPTION_UNSUPPORTED"),
            "unsupported duplex should expose structured api code"
        );
    }

    #[test]
    fn validate_print_options_against_capabilities_rejects_out_of_range_copies() {
        let capabilities = PrinterCapabilities {
            copies: Some(CopiesCapability {
                default: Some(1),
                min: Some(1),
                max: Some(2),
            }),
            ..PrinterCapabilities::default()
        };
        let err = validate_print_options_against_capabilities(
            &capabilities,
            &PrintOptions {
                copies: Some(3),
                ..PrintOptions::default()
            },
        )
        .expect_err("copies above max should fail");
        assert_eq!(err.api_code(), Some("PRINT_OPTION_INVALID_FOR_PRINTER"));
    }

    #[test]
    fn validate_print_options_against_capabilities_rejects_unknown_copies() {
        let err = validate_print_options_against_capabilities(
            &PrinterCapabilities::default(),
            &PrintOptions {
                copies: Some(2),
                ..PrintOptions::default()
            },
        )
        .expect_err("unknown copies capability should fail");
        assert_eq!(err.api_code(), Some("PRINTER_CAPABILITY_UNKNOWN"));
    }

    #[test]
    fn validate_print_options_against_capabilities_rejects_unsupported_color_mode() {
        let capabilities = PrinterCapabilities {
            color_supported: Some(false),
            ..PrinterCapabilities::default()
        };
        let err = validate_print_options_against_capabilities(
            &capabilities,
            &PrintOptions {
                print_color_mode: Some(PrintColorMode::Color),
                ..PrintOptions::default()
            },
        )
        .expect_err("unsupported color mode should fail");
        assert_eq!(err.api_code(), Some("PRINT_OPTION_UNSUPPORTED"));
    }

    #[test]
    fn validate_print_options_against_capabilities_rejects_unknown_color_mode() {
        let err = validate_print_options_against_capabilities(
            &PrinterCapabilities::default(),
            &PrintOptions {
                print_color_mode: Some(PrintColorMode::Monochrome),
                ..PrintOptions::default()
            },
        )
        .expect_err("unknown color mode capability should fail");
        assert_eq!(err.api_code(), Some("PRINTER_CAPABILITY_UNKNOWN"));
    }

    #[test]
    fn validate_print_options_against_capabilities_rejects_unknown_orientation_capability() {
        let capabilities = PrinterCapabilities::default();
        let err = validate_print_options_against_capabilities(
            &capabilities,
            &PrintOptions {
                orientation_requested: Some(OrientationRequested::Landscape),
                ..PrintOptions::default()
            },
        )
        .expect_err("unknown orientation capability should fail");
        assert_eq!(err.api_code(), Some("PRINTER_CAPABILITY_UNKNOWN"));
    }

    #[test]
    fn validate_print_options_against_capabilities_rejects_page_ranges_when_disallowed() {
        let capabilities = PrinterCapabilities {
            supports_page_ranges: Some(false),
            ..PrinterCapabilities::default()
        };
        let err = validate_print_options_against_capabilities(
            &capabilities,
            &PrintOptions {
                page_ranges: Some("1-3".to_string()),
                ..PrintOptions::default()
            },
        )
        .expect_err("page ranges should be rejected");
        assert_eq!(err.api_code(), Some("PRINT_OPTION_UNSUPPORTED"));
    }

    #[test]
    fn validate_print_options_against_capabilities_rejects_page_ranges_when_unknown() {
        let capabilities = PrinterCapabilities::default();
        let err = validate_print_options_against_capabilities(
            &capabilities,
            &PrintOptions {
                page_ranges: Some("1-3".to_string()),
                ..PrintOptions::default()
            },
        )
        .expect_err("unknown page ranges capability should fail");
        assert_eq!(err.api_code(), Some("PRINTER_CAPABILITY_UNKNOWN"));
    }

    #[test]
    fn validate_print_options_against_capabilities_accepts_supported_media_combo() {
        let capabilities = PrinterCapabilities {
            media_supported: vec!["iso_a4_210x297mm".to_string()],
            media_types_supported: vec!["stationery".to_string()],
            ..PrinterCapabilities::default()
        };
        validate_print_options_against_capabilities(
            &capabilities,
            &PrintOptions {
                media: Some("iso_a4_210x297mm".to_string()),
                media_type: Some("stationery".to_string()),
                ..PrintOptions::default()
            },
        )
        .expect("supported media combo should pass");
    }
}
