use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrintBackendError {
    #[error("invalid printer target: {0}")]
    InvalidTarget(String),
    #[error("printer target is unreachable: {0}")]
    Unreachable(String),
    #[error("printer target is unsupported: {0}")]
    Unsupported(String),
    #[error("printer target conflict: {0}")]
    Conflict(String),
    #[error("{message}")]
    PrintOptionUnsupported {
        option: String,
        requested: Option<String>,
        supported: Vec<String>,
        message: String,
    },
    #[error("{message}")]
    PrintOptionInvalidForPrinter {
        option: String,
        requested: String,
        reason: &'static str,
        limit: Option<u16>,
        message: String,
    },
    #[error("{message}")]
    PrinterCapabilityUnknown {
        option: String,
        requested: Option<String>,
        message: String,
    },
    #[error("backend error: {0}")]
    Backend(String),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl PrintBackendError {
    pub fn invalid_target(message: impl Into<String>) -> Self {
        Self::InvalidTarget(message.into())
    }

    pub fn unreachable(message: impl Into<String>) -> Self {
        Self::Unreachable(message.into())
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::Unsupported(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn print_option_unsupported(
        option: impl Into<String>,
        requested: Option<String>,
        supported: Vec<String>,
    ) -> Self {
        let option = option.into();
        let message = match requested.as_deref() {
            Some(requested) => format!("{option}={requested} is not supported by this printer"),
            None => format!("{option} is not supported by this printer"),
        };
        Self::PrintOptionUnsupported {
            option,
            requested,
            supported,
            message,
        }
    }

    pub fn print_option_invalid_for_printer(
        option: impl Into<String>,
        requested: impl Into<String>,
        reason: &'static str,
        limit: Option<u16>,
    ) -> Self {
        let option = option.into();
        let requested = requested.into();
        let message = match (reason, limit) {
            ("below_minimum", Some(limit)) => {
                format!("{option}={requested} is below printer minimum {limit}")
            }
            ("above_maximum", Some(limit)) => {
                format!("{option}={requested} exceeds printer maximum {limit}")
            }
            _ => format!("{option}={requested} is invalid for this printer"),
        };
        Self::PrintOptionInvalidForPrinter {
            option,
            requested,
            reason,
            limit,
            message,
        }
    }

    pub fn printer_capability_unknown(
        option: impl Into<String>,
        requested: Option<String>,
    ) -> Self {
        let option = option.into();
        let message = format!("{option} support is unknown for this printer");
        Self::PrinterCapabilityUnknown {
            option,
            requested,
            message,
        }
    }

    pub fn backend(message: impl Into<String>) -> Self {
        Self::Backend(message.into())
    }

    pub fn api_code(&self) -> Option<&'static str> {
        match self {
            Self::InvalidTarget(_) => Some("INVALID_PRINTER_TARGET"),
            Self::Unreachable(_) => Some("PRINTER_UNREACHABLE"),
            Self::Unsupported(_) => Some("UNSUPPORTED_PRINTER_TARGET"),
            Self::Conflict(_) => Some("PRINTER_CONFLICT"),
            Self::PrintOptionUnsupported { .. } => Some("PRINT_OPTION_UNSUPPORTED"),
            Self::PrintOptionInvalidForPrinter { .. } => Some("PRINT_OPTION_INVALID_FOR_PRINTER"),
            Self::PrinterCapabilityUnknown { .. } => Some("PRINTER_CAPABILITY_UNKNOWN"),
            Self::Backend(_) => Some("PRINT_BACKEND_ERROR"),
            Self::Db(_) | Self::Serde(_) => None,
        }
    }

    pub fn api_message(&self) -> Option<&str> {
        match self {
            Self::InvalidTarget(message)
            | Self::Unreachable(message)
            | Self::Unsupported(message)
            | Self::Conflict(message)
            | Self::Backend(message) => Some(message.as_str()),
            Self::PrintOptionUnsupported { message, .. }
            | Self::PrintOptionInvalidForPrinter { message, .. }
            | Self::PrinterCapabilityUnknown { message, .. } => Some(message.as_str()),
            Self::Db(_) | Self::Serde(_) => None,
        }
    }

    pub fn api_details(&self) -> Option<Value> {
        match self {
            Self::PrintOptionUnsupported {
                option,
                requested,
                supported,
                ..
            } => {
                let mut details = serde_json::Map::from_iter([(
                    "option".to_string(),
                    Value::String(option.clone()),
                )]);
                if let Some(requested) = requested {
                    details.insert(
                        "requested_value".to_string(),
                        Value::String(requested.clone()),
                    );
                }
                if !supported.is_empty() {
                    details.insert(
                        "supported_values".to_string(),
                        Value::Array(
                            supported
                                .iter()
                                .cloned()
                                .map(Value::String)
                                .collect::<Vec<_>>(),
                        ),
                    );
                }
                Some(Value::Object(details))
            }
            Self::PrintOptionInvalidForPrinter {
                option,
                requested,
                reason,
                limit,
                ..
            } => {
                let mut details = serde_json::Map::from_iter([
                    ("option".to_string(), Value::String(option.clone())),
                    (
                        "requested_value".to_string(),
                        Value::String(requested.clone()),
                    ),
                    ("reason".to_string(), Value::String((*reason).to_string())),
                ]);
                if let Some(limit) = limit {
                    details.insert("limit".to_string(), json!(limit));
                }
                Some(Value::Object(details))
            }
            Self::PrinterCapabilityUnknown {
                option, requested, ..
            } => {
                let mut details = serde_json::Map::from_iter([(
                    "option".to_string(),
                    Value::String(option.clone()),
                )]);
                if let Some(requested) = requested {
                    details.insert(
                        "requested_value".to_string(),
                        Value::String(requested.clone()),
                    );
                }
                Some(Value::Object(details))
            }
            _ => None,
        }
    }
}
