use std::time::Duration;

use ipp::{
    client::non_blocking::AsyncIppClient,
    model::{DelimiterTag, PrinterState, StatusCode},
    operation::builder::IppOperationBuilder,
    prelude::{IppValue, Uri},
};
use serde_json::{Map, Value};

use super::{
    error::PrintBackendError,
    mapper::extract_capabilities,
    registry::PrinterDetail,
    types::{PrinterCapabilities, PrinterTargetInput, ValidatedPrinterTarget},
};

const REQUESTED_PRINTER_ATTRIBUTES: &[&str] = &[
    "printer-name",
    "printer-info",
    "printer-state",
    "printer-state-message",
    "printer-state-reasons",
    "printer-is-accepting-jobs",
    "printer-make-and-model",
    "document-format-supported",
    "media-supported",
    "media-default",
    "media-type-supported",
    "sides-supported",
    "sides-default",
    "copies-supported",
    "copies-default",
    "print-color-mode-supported",
    "color-supported",
    "orientation-requested-supported",
    "print-scaling-supported",
    "page-ranges-supported",
    "job-creation-attributes-supported",
];

pub async fn validate_printer_target(
    target: &PrinterTargetInput,
) -> Result<ValidatedPrinterTarget, PrintBackendError> {
    let candidates = candidate_uris(target)?;
    let mut last_error = None;

    for candidate in candidates {
        match validate_uri(candidate.as_str()).await {
            Ok(validated) => return Ok(validated),
            Err(err) => last_error = Some(err),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        PrintBackendError::unreachable("no reachable IPP endpoint could be validated")
    }))
}

pub fn normalize_printer_uri(input: &str) -> Result<String, PrintBackendError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(PrintBackendError::invalid_target(
            "printer uri must not be empty",
        ));
    }

    let normalized_input = if has_explicit_scheme(trimmed) {
        trimmed.to_string()
    } else {
        format!("ipp://{trimmed}")
    };

    let parsed: Uri = normalized_input
        .parse()
        .map_err(|err| PrintBackendError::invalid_target(format!("invalid printer uri: {err}")))?;
    let scheme = parsed
        .scheme_str()
        .ok_or_else(|| PrintBackendError::invalid_target("printer uri scheme is required"))?;
    if !matches!(scheme, "ipp" | "ipps" | "http" | "https") {
        return Err(PrintBackendError::invalid_target(format!(
            "unsupported printer uri scheme: {scheme}"
        )));
    }

    let authority = parsed.authority().ok_or_else(|| {
        PrintBackendError::invalid_target("printer uri must include a hostname or IP address")
    })?;
    let host = authority.host().trim().to_lowercase();
    if host.is_empty() {
        return Err(PrintBackendError::invalid_target(
            "printer uri host must not be empty",
        ));
    }

    let path = normalize_path(parsed.path());
    let normalized = match authority.port_u16() {
        Some(port) => format!("{scheme}://{host}:{port}{path}"),
        None => format!("{scheme}://{host}{path}"),
    };
    Ok(normalized)
}

async fn validate_uri(uri: &str) -> Result<ValidatedPrinterTarget, PrintBackendError> {
    let snapshot = fetch_printer_snapshot(uri).await?;
    Ok(ValidatedPrinterTarget {
        normalized_uri: normalize_printer_uri(uri)?,
        printer_uri: uri.to_string(),
        discovered_name: snapshot
            .discovered_name
            .clone()
            .unwrap_or_else(|| uri.to_string()),
        state: snapshot.state,
        state_message: snapshot.state_message,
        capabilities: snapshot.capabilities,
        attributes: snapshot.attributes,
        already_managed: false,
        managed_printer_id: None,
    })
}

pub async fn get_printer_detail(printer_uri: &str) -> Result<PrinterDetail, PrintBackendError> {
    let snapshot = fetch_printer_snapshot(printer_uri).await?;
    Ok(PrinterDetail {
        id: String::new(),
        name: snapshot
            .discovered_name
            .unwrap_or_else(|| printer_uri.to_string()),
        uri: printer_uri.to_string(),
        normalized_uri: normalize_printer_uri(printer_uri)?,
        source: "ipp".to_string(),
        is_default: false,
        enabled: true,
        state: snapshot.state,
        state_message: snapshot.state_message,
        capabilities: snapshot.capabilities,
        attributes: snapshot.attributes,
        last_seen_at: None,
        last_validated_at: None,
        last_refreshed_at: None,
        created_at: 0,
        updated_at: 0,
    })
}

struct IppPrinterSnapshot {
    discovered_name: Option<String>,
    state: Option<String>,
    state_message: Option<String>,
    capabilities: PrinterCapabilities,
    attributes: Value,
}

async fn fetch_printer_snapshot(uri: &str) -> Result<IppPrinterSnapshot, PrintBackendError> {
    let parsed: Uri = uri
        .parse()
        .map_err(|err| PrintBackendError::invalid_target(format!("invalid printer uri: {err}")))?;
    let operation = IppOperationBuilder::get_printer_attributes(parsed.clone())
        .attributes(REQUESTED_PRINTER_ATTRIBUTES)
        .build()
        .map_err(|err| PrintBackendError::invalid_target(format!("invalid ipp request: {err}")))?;

    let client = AsyncIppClient::builder(parsed)
        .request_timeout(Duration::from_secs(8))
        .build();
    let response = client
        .send(operation)
        .await
        .map_err(|err| PrintBackendError::unreachable(err.to_string()))?;

    if !response.header().status_code().is_success() {
        let status = response.header().status_code();
        return match status {
            StatusCode::ClientErrorBadRequest
            | StatusCode::ClientErrorAttributesOrValuesNotSupported
            | StatusCode::ClientErrorUriSchemeNotSupported => Err(PrintBackendError::unsupported(
                format!("ipp status returned {status:?}"),
            )),
            _ => Err(PrintBackendError::unreachable(format!(
                "ipp request failed with status {status:?}"
            ))),
        };
    }

    let attributes = response
        .attributes()
        .groups_of(DelimiterTag::PrinterAttributes)
        .next()
        .ok_or_else(|| {
            PrintBackendError::unsupported("printer did not return printer-attributes".to_string())
        })?;

    let attributes_json = attributes_to_json(attributes);
    let state = attributes
        .attributes()
        .get("printer-state")
        .and_then(|attr| attr.value().as_enum())
        .and_then(|raw| match *raw {
            3 => Some(PrinterState::Idle),
            4 => Some(PrinterState::Processing),
            5 => Some(PrinterState::Stopped),
            _ => None,
        })
        .map(printer_state_to_string);
    let state_message = attributes
        .attributes()
        .get("printer-state-message")
        .and_then(|attr| attr.value().as_text_without_language())
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty());

    let discovered_name = discover_name(&attributes_json);
    let capabilities = extract_capabilities(&attributes_json);

    Ok(IppPrinterSnapshot {
        discovered_name,
        state,
        state_message,
        capabilities,
        attributes: attributes_json,
    })
}

fn candidate_uris(target: &PrinterTargetInput) -> Result<Vec<String>, PrintBackendError> {
    match target {
        PrinterTargetInput::Uri { uri } => Ok(vec![normalize_printer_uri(uri)?]),
        PrinterTargetInput::Host { host } => {
            let host = host.trim();
            if host.is_empty() {
                return Err(PrintBackendError::invalid_target("host must not be empty"));
            }
            Ok(vec![
                format!("ipps://{host}/ipp/print"),
                format!("ipps://{host}:443/ipp/print"),
                format!("ipp://{host}/ipp/print"),
                format!("ipp://{host}:631/ipp/print"),
                format!("http://{host}:631/ipp/print"),
            ])
        }
    }
}

fn has_explicit_scheme(input: &str) -> bool {
    input.contains("://")
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return "/ipp/print".to_string();
    }

    let mut normalized = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    };

    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }

    normalized
}

fn discover_name(attributes: &Value) -> Option<String> {
    ["printer-name", "printer-info", "printer-make-and-model"]
        .into_iter()
        .find_map(|key| attributes.get(key).and_then(as_display_string))
        .filter(|value| !value.trim().is_empty())
}

fn as_display_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(values) => values.iter().find_map(as_display_string),
        _ => None,
    }
}

fn printer_state_to_string(state: PrinterState) -> String {
    match state {
        PrinterState::Idle => "idle",
        PrinterState::Processing => "processing",
        PrinterState::Stopped => "stopped",
    }
    .to_string()
}

fn attributes_to_json(group: &ipp::attribute::IppAttributeGroup) -> Value {
    let mut object = Map::new();
    for (name, attribute) in group.attributes() {
        object.insert(name.to_string(), ipp_value_to_json(attribute.value()));
    }
    Value::Object(object)
}

fn ipp_value_to_json(value: &IppValue) -> Value {
    match value {
        IppValue::Integer(number) | IppValue::Enum(number) => Value::from(*number),
        IppValue::Boolean(flag) => Value::from(*flag),
        IppValue::Charset(text) => Value::String(text.to_string()),
        IppValue::NaturalLanguage(text) => Value::String(text.to_string()),
        IppValue::Uri(text) => Value::String(text.to_string()),
        IppValue::UriScheme(text) => Value::String(text.to_string()),
        IppValue::Keyword(text) => Value::String(text.to_string()),
        IppValue::MimeMediaType(text) => Value::String(text.to_string()),
        IppValue::NameWithoutLanguage(text) => Value::String(text.to_string()),
        IppValue::TextWithoutLanguage(text) => Value::String(text.to_string()),
        IppValue::TextWithLanguage { language, text } => serde_json::json!({
            "language": language.to_string(),
            "text": text.to_string(),
        }),
        IppValue::NameWithLanguage { language, name } => serde_json::json!({
            "language": language.to_string(),
            "name": name.to_string(),
        }),
        IppValue::RangeOfInteger { min, max } => serde_json::json!({
            "min": min,
            "max": max,
        }),
        IppValue::Array(items) => Value::Array(items.iter().map(ipp_value_to_json).collect()),
        IppValue::Collection(entries) => {
            let mut object = Map::new();
            for (key, nested) in entries {
                object.insert(key.to_string(), ipp_value_to_json(nested));
            }
            Value::Object(object)
        }
        IppValue::DateTime {
            year,
            month,
            day,
            hour,
            minutes,
            seconds,
            deci_seconds,
            utc_dir,
            utc_hours,
            utc_mins,
        } => serde_json::json!({
            "year": year,
            "month": month,
            "day": day,
            "hour": hour,
            "minutes": minutes,
            "seconds": seconds,
            "deci_seconds": deci_seconds,
            "utc_dir": utc_dir.to_string(),
            "utc_hours": utc_hours,
            "utc_mins": utc_mins,
        }),
        IppValue::Resolution {
            cross_feed,
            feed,
            units,
        } => serde_json::json!({
            "cross_feed": cross_feed,
            "feed": feed,
            "units": units,
        }),
        IppValue::NoValue => Value::Null,
        IppValue::OctetString(text) => Value::String(text.to_string()),
        IppValue::MemberAttrName(text) => Value::String(text.to_string()),
        IppValue::Other { tag, data } => serde_json::json!({
            "tag": tag,
            "data_hex": bytes_to_hex(data),
        }),
    }
}

fn bytes_to_hex(data: &[u8]) -> String {
    let mut output = String::with_capacity(data.len() * 2);
    for byte in data {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_printer_uri_defaults_host_and_path() {
        let normalized = normalize_printer_uri("Printer.LOCAL").expect("normalize uri");
        assert_eq!(normalized, "ipp://printer.local/ipp/print");
    }

    #[test]
    fn normalize_printer_uri_trims_trailing_slash_and_preserves_port() {
        let normalized =
            normalize_printer_uri("ipps://Example.com:443/custom/path/").expect("normalize uri");
        assert_eq!(normalized, "ipps://example.com:443/custom/path");
    }
}
