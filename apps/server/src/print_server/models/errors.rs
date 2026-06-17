use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::Value;
use thiserror::Error;
use tracing::error;

#[derive(Debug, Error)]
pub(crate) enum ApiError {
    #[error("{message}")]
    Structured {
        status: StatusCode,
        code: &'static str,
        message: String,
        details: Option<Value>,
    },
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    ServiceUnavailable(String),
    #[error("{0}")]
    Internal(String),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
}

impl ApiError {
    pub(crate) fn structured(
        status: StatusCode,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self::Structured {
            status,
            code,
            message: message.into(),
            details: None,
        }
    }

    pub(crate) fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::structured(StatusCode::BAD_REQUEST, code, message)
    }

    pub(crate) fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::structured(StatusCode::NOT_FOUND, code, message)
    }

    pub(crate) fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::structured(StatusCode::CONFLICT, code, message)
    }

    pub(crate) fn service_unavailable(code: &'static str, message: impl Into<String>) -> Self {
        Self::structured(StatusCode::SERVICE_UNAVAILABLE, code, message)
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self::structured(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", message)
    }

    pub(crate) fn with_details(mut self, details: Value) -> Self {
        match &mut self {
            Self::Structured { details: slot, .. } => {
                *slot = Some(details);
            }
            Self::Db(_) => {}
            Self::BadRequest(_)
            | Self::Unauthorized(_)
            | Self::NotFound(_)
            | Self::Conflict(_)
            | Self::ServiceUnavailable(_)
            | Self::Internal(_) => {}
        }
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Structured {
                status,
                code,
                message,
                details,
            } => error_json_response(status, code, message, details),
            ApiError::BadRequest(msg) => {
                error_json_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", msg, None)
            }
            ApiError::Unauthorized(msg) => {
                error_json_response(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg, None)
            }
            ApiError::NotFound(msg) => {
                error_json_response(StatusCode::NOT_FOUND, "NOT_FOUND", msg, None)
            }
            ApiError::Conflict(msg) => {
                error_json_response(StatusCode::CONFLICT, "CONFLICT", msg, None)
            }
            ApiError::ServiceUnavailable(msg) => error_json_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_UNAVAILABLE",
                msg,
                None,
            ),
            ApiError::Internal(msg) => error_json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg,
                None,
            ),
            ApiError::Db(err) => {
                error!("database error: {err}");
                error_json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "internal error".to_string(),
                    None,
                )
            }
        }
    }
}

pub(crate) type ApiResult<T> = Result<T, ApiError>;

fn error_json_response(
    status: StatusCode,
    code: &'static str,
    message: String,
    details: Option<Value>,
) -> Response {
    let mut payload = serde_json::Map::from_iter([
        ("code".to_string(), Value::String(code.to_string())),
        ("message".to_string(), Value::String(message)),
    ]);
    if let Some(details) = details {
        payload.insert("details".to_string(), details);
    }
    (status, Json(Value::Object(payload))).into_response()
}
