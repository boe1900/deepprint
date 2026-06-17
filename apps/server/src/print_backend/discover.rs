use std::time::Duration;

use ipp::{
    client::non_blocking::AsyncIppClient, operation::builder::IppOperationBuilder, prelude::Uri,
};
use serde_json::Value;

use super::{error::PrintBackendError, ipp::normalize_printer_uri, types::DiscoveredPrinter};

pub async fn discover_mdns_printers() -> Vec<DiscoveredPrinter> {
    Vec::new()
}

#[cfg(unix)]
pub async fn probe_cups_connection(cups_base_url: &str) -> Result<(), PrintBackendError> {
    let cups_uri: Uri = cups_base_url.parse().map_err(|err| {
        PrintBackendError::invalid_target(format!("invalid cups endpoint: {err}"))
    })?;
    let cups_builder = IppOperationBuilder::cups();
    let operation = cups_builder.get_printers();
    let response = AsyncIppClient::builder(cups_uri)
        .request_timeout(Duration::from_secs(5))
        .build()
        .send(operation)
        .await
        .map_err(|err| {
            PrintBackendError::unreachable(format!("unable to reach cups service: {err}"))
        })?;

    if !response.header().status_code().is_success() {
        return Err(PrintBackendError::backend(format!(
            "cups service returned {}",
            response.header().status_code()
        )));
    }

    Ok(())
}

#[cfg(not(unix))]
pub async fn probe_cups_connection(_cups_base_url: &str) -> Result<(), PrintBackendError> {
    Ok(())
}

#[cfg(unix)]
pub async fn discover_cups_printers(
    cups_base_url: &str,
) -> Result<Vec<DiscoveredPrinter>, PrintBackendError> {
    let cups_uri: Uri = cups_base_url.parse().map_err(|err| {
        PrintBackendError::invalid_target(format!("invalid cups endpoint: {err}"))
    })?;
    let cups_builder = IppOperationBuilder::cups();
    let operation = cups_builder.get_printers();
    let response = AsyncIppClient::builder(cups_uri)
        .request_timeout(Duration::from_secs(5))
        .build()
        .send(operation)
        .await
        .map_err(|err| {
            PrintBackendError::unreachable(format!("unable to reach cups service: {err}"))
        })?;

    if !response.header().status_code().is_success() {
        return Err(PrintBackendError::backend(format!(
            "cups service returned {}",
            response.header().status_code()
        )));
    }

    let mut printers = Vec::new();
    for group in response.attributes().groups() {
        let attrs = group.attributes();
        let candidate_uri = attrs
            .get("printer-uri-supported")
            .or_else(|| attrs.get("printer-uri"))
            .map(|attribute| ipp_value_to_json(attribute.value()))
            .and_then(|value| as_display_string(&value))
            .and_then(|value| normalize_printer_uri(&value).ok());
        let display_name = attrs
            .get("printer-info")
            .or_else(|| attrs.get("printer-name"))
            .map(|attribute| ipp_value_to_json(attribute.value()))
            .and_then(|value| as_display_string(&value));

        if let Some(candidate_uri) = candidate_uri {
            printers.push(DiscoveredPrinter {
                display_name: display_name.unwrap_or_else(|| candidate_uri.clone()),
                candidate_uri,
                source: "cups_import".to_string(),
                is_already_managed: false,
                managed_printer_id: None,
            });
        }
    }

    printers.sort_by(|left, right| left.display_name.cmp(&right.display_name));
    printers.dedup_by(|left, right| left.candidate_uri == right.candidate_uri);
    Ok(printers)
}

#[cfg(not(unix))]
pub async fn discover_cups_printers(
    _cups_base_url: &str,
) -> Result<Vec<DiscoveredPrinter>, PrintBackendError> {
    Ok(Vec::new())
}

fn as_display_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(values) => values.iter().find_map(as_display_string),
        _ => None,
    }
}

fn ipp_value_to_json(value: &ipp::prelude::IppValue) -> Value {
    match value {
        ipp::prelude::IppValue::Uri(text) => Value::String(text.to_string()),
        ipp::prelude::IppValue::Keyword(text) => Value::String(text.to_string()),
        ipp::prelude::IppValue::NameWithoutLanguage(text) => Value::String(text.to_string()),
        ipp::prelude::IppValue::TextWithoutLanguage(text) => Value::String(text.to_string()),
        ipp::prelude::IppValue::Charset(text) => Value::String(text.to_string()),
        ipp::prelude::IppValue::NaturalLanguage(text) => Value::String(text.to_string()),
        ipp::prelude::IppValue::UriScheme(text) => Value::String(text.to_string()),
        ipp::prelude::IppValue::MimeMediaType(text) => Value::String(text.to_string()),
        ipp::prelude::IppValue::MemberAttrName(text) => Value::String(text.to_string()),
        ipp::prelude::IppValue::TextWithLanguage { text, .. } => Value::String(text.to_string()),
        ipp::prelude::IppValue::NameWithLanguage { name, .. } => Value::String(name.to_string()),
        ipp::prelude::IppValue::Array(items) => {
            Value::Array(items.iter().map(ipp_value_to_json).collect())
        }
        _ => Value::Null,
    }
}
