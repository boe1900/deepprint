use std::{
    collections::HashMap,
    fs::File,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, SystemTime},
};

use ipp::{
    attribute::IppAttribute,
    attribute::IppAttributes,
    client::non_blocking::AsyncIppClient,
    model::{JobState, StatusCode},
    operation::builder::IppOperationBuilder,
    payload::IppPayload,
    prelude::{IppValue, Uri},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct PrintOptions {
    pub copies: Option<u16>,
    pub sides: Option<SidesMode>,
    #[serde(rename = "printColorMode")]
    pub print_color_mode: Option<PrintColorMode>,
    pub media: Option<String>,
    #[serde(rename = "mediaType")]
    pub media_type: Option<String>,
    #[serde(rename = "orientationRequested")]
    pub orientation_requested: Option<OrientationRequested>,
    #[serde(rename = "printScaling")]
    pub print_scaling: Option<PrintScaling>,
    #[serde(rename = "pageRanges")]
    pub page_ranges: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SidesMode {
    OneSided,
    TwoSidedLongEdge,
    TwoSidedShortEdge,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PrintColorMode {
    Color,
    Monochrome,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OrientationRequested {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PrintScaling {
    Auto,
    AutoFit,
    Fit,
    Fill,
    None,
}

impl SidesMode {
    pub(crate) fn as_ipp_keyword(self) -> &'static str {
        match self {
            Self::OneSided => "one-sided",
            Self::TwoSidedLongEdge => "two-sided-long-edge",
            Self::TwoSidedShortEdge => "two-sided-short-edge",
        }
    }
}

impl PrintColorMode {
    pub(crate) fn as_ipp_keyword(self) -> &'static str {
        match self {
            Self::Color => "color",
            Self::Monochrome => "monochrome",
        }
    }
}

impl OrientationRequested {
    pub(crate) fn as_ipp_enum(self) -> i32 {
        match self {
            Self::Portrait => 3,
            Self::Landscape => 4,
        }
    }

    pub(crate) fn as_capability_keyword(self) -> &'static str {
        match self {
            Self::Portrait => "portrait",
            Self::Landscape => "landscape",
        }
    }
}

impl PrintScaling {
    pub(crate) fn as_ipp_keyword(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::AutoFit => "auto-fit",
            Self::Fit => "fit",
            Self::Fill => "fill",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PrinterInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub status: String,
    pub backend: String,
}

#[derive(Debug, Clone)]
pub struct SubmitJobRequest {
    pub local_file: PathBuf,
    pub printer_uri: String,
    pub job_name: String,
    pub document_format: Option<String>,
    pub options: PrintOptions,
}

#[derive(Debug, Clone)]
pub struct SubmitJobResult {
    pub backend: String,
    pub backend_job_ref_json: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendJobState {
    Pending,
    Processing,
    Completed,
    Failed,
    Canceled,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct BackendError {
    code: &'static str,
    message: String,
    retryable: bool,
}

impl BackendError {
    pub fn new(code: &'static str, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
        }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn retryable(&self) -> bool {
        self.retryable
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for BackendError {}

pub trait PrinterBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn list_printers(&self) -> Result<Vec<PrinterInfo>, BackendError>;
    fn submit_job(&self, req: &SubmitJobRequest) -> Result<SubmitJobResult, BackendError>;
    fn reconcile_submission(
        &self,
        _printer_uri: &str,
        _job_name: &str,
        _submit_started_at: Option<i64>,
    ) -> Result<Option<String>, BackendError> {
        Ok(None)
    }
    fn query_job_status(
        &self,
        _backend_job_ref_json: &str,
    ) -> Result<BackendJobState, BackendError> {
        Err(BackendError::new(
            "BACKEND_METHOD_UNSUPPORTED",
            "query_job_status is not supported by current backend",
            false,
        ))
    }
    fn cancel_job(&self, _backend_job_ref_json: &str) -> Result<(), BackendError> {
        Err(BackendError::new(
            "BACKEND_METHOD_UNSUPPORTED",
            "cancel_job is not supported by current backend",
            false,
        ))
    }
}

pub fn create_backend(mock_mode: bool) -> Arc<dyn PrinterBackend> {
    if mock_mode {
        return Arc::new(MockPrinterBackend);
    }

    Arc::new(IppPrinterBackend)
}

struct MockPrinterBackend;

#[derive(Debug, Clone)]
struct MockJobState {
    submitted_at: SystemTime,
    canceled: bool,
}

static MOCK_JOBS: LazyLock<Mutex<HashMap<String, MockJobState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

impl PrinterBackend for MockPrinterBackend {
    fn backend_name(&self) -> &'static str {
        "mock"
    }

    fn list_printers(&self) -> Result<Vec<PrinterInfo>, BackendError> {
        Ok(vec![PrinterInfo {
            id: "mock-printer".to_string(),
            name: "Mock Printer".to_string(),
            is_default: true,
            status: "online".to_string(),
            backend: self.backend_name().to_string(),
        }])
    }

    fn submit_job(&self, req: &SubmitJobRequest) -> Result<SubmitJobResult, BackendError> {
        if !req.local_file.exists() {
            return Err(BackendError::new(
                "PRINT_FILE_NOT_FOUND",
                format!("print file not found: {}", req.local_file.display()),
                false,
            ));
        }

        let backend_job_ref_json = format!("mock-{}", Uuid::new_v4());
        let mut map = MOCK_JOBS.lock().map_err(|_| {
            BackendError::new(
                "MOCK_STATE_LOCK_FAILED",
                "unable to lock mock job store",
                true,
            )
        })?;
        map.insert(
            backend_job_ref_json.clone(),
            MockJobState {
                submitted_at: SystemTime::now(),
                canceled: false,
            },
        );

        Ok(SubmitJobResult {
            backend: self.backend_name().to_string(),
            backend_job_ref_json: Some(backend_job_ref_json),
        })
    }

    fn reconcile_submission(
        &self,
        _printer_uri: &str,
        _job_name: &str,
        _submit_started_at: Option<i64>,
    ) -> Result<Option<String>, BackendError> {
        Ok(None)
    }

    fn query_job_status(
        &self,
        backend_job_ref_json: &str,
    ) -> Result<BackendJobState, BackendError> {
        let mut map = MOCK_JOBS.lock().map_err(|_| {
            BackendError::new(
                "MOCK_STATE_LOCK_FAILED",
                "unable to lock mock job store",
                true,
            )
        })?;

        let Some(state) = map.get(backend_job_ref_json).cloned() else {
            return Ok(BackendJobState::Unknown);
        };

        if state.canceled {
            map.remove(backend_job_ref_json);
            return Ok(BackendJobState::Canceled);
        }

        let elapsed = state
            .submitted_at
            .elapsed()
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_millis();

        if elapsed < 1800 {
            return Ok(BackendJobState::Processing);
        }

        map.remove(backend_job_ref_json);
        Ok(BackendJobState::Completed)
    }

    fn cancel_job(&self, backend_job_ref_json: &str) -> Result<(), BackendError> {
        let mut map = MOCK_JOBS.lock().map_err(|_| {
            BackendError::new(
                "MOCK_STATE_LOCK_FAILED",
                "unable to lock mock job store",
                true,
            )
        })?;
        if let Some(state) = map.get_mut(backend_job_ref_json) {
            state.canceled = true;
            return Ok(());
        }

        map.insert(
            backend_job_ref_json.to_string(),
            MockJobState {
                submitted_at: SystemTime::now(),
                canceled: true,
            },
        );
        Ok(())
    }
}

struct IppPrinterBackend;

impl PrinterBackend for IppPrinterBackend {
    fn backend_name(&self) -> &'static str {
        "ipp"
    }

    fn list_printers(&self) -> Result<Vec<PrinterInfo>, BackendError> {
        Ok(vec![])
    }

    fn submit_job(&self, req: &SubmitJobRequest) -> Result<SubmitJobResult, BackendError> {
        if !req.local_file.exists() {
            return Err(BackendError::new(
                "PRINT_FILE_NOT_FOUND",
                format!("print file not found: {}", req.local_file.display()),
                false,
            ));
        }

        let printer_uri =
            req.printer_uri.trim().parse::<Uri>().map_err(|err| {
                BackendError::new("IPP_PRINTER_URI_INVALID", err.to_string(), false)
            })?;
        let payload = IppPayload::new(File::open(&req.local_file).map_err(|err| {
            BackendError::new(
                "PRINT_FILE_OPEN_FAILED",
                format!("failed to open print file: {err}"),
                false,
            )
        })?);
        let operation = IppOperationBuilder::print_job(printer_uri.clone(), payload)
            .job_title(req.job_name.as_str())
            .user_name("deepprint")
            .document_format(
                req.document_format
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or("application/pdf"),
            )
            .attributes(build_ipp_job_attributes(&req.options)?)
            .build()
            .map_err(|err| BackendError::new("IPP_REQUEST_BUILD_FAILED", err.to_string(), false))?;

        let response = tokio::runtime::Handle::current()
            .block_on(async {
                AsyncIppClient::builder(printer_uri)
                    .request_timeout(Duration::from_secs(20))
                    .build()
                    .send(operation)
                    .await
            })
            .map_err(|err| classify_ipp_client_error("IPP_SUBMIT_FAILED", err.to_string()))?;

        if !response.header().status_code().is_success() {
            return Err(BackendError::new(
                "IPP_SUBMIT_STATUS_FAILED",
                format!(
                    "ipp submit failed with status {:?}",
                    response.header().status_code()
                ),
                is_ipp_retryable_status(response.header().status_code()),
            ));
        }

        let backend_job_ref_json = extract_job_id(response.attributes())
            .map(|job_id| encode_ipp_backend_job_ref(req.printer_uri.as_str(), job_id))
            .ok_or_else(|| {
                BackendError::new(
                    "IPP_JOB_ID_MISSING",
                    "ipp submit succeeded but job-id is missing",
                    false,
                )
            })?;

        Ok(SubmitJobResult {
            backend: self.backend_name().to_string(),
            backend_job_ref_json: Some(backend_job_ref_json),
        })
    }

    fn reconcile_submission(
        &self,
        printer_uri: &str,
        job_name: &str,
        _submit_started_at: Option<i64>,
    ) -> Result<Option<String>, BackendError> {
        let uri = printer_uri
            .trim()
            .parse::<Uri>()
            .map_err(|err| BackendError::new("IPP_PRINTER_URI_INVALID", err.to_string(), false))?;
        let operation = IppOperationBuilder::get_jobs(uri.clone())
            .user_name("deepprint")
            .build()
            .map_err(|err| BackendError::new("IPP_REQUEST_BUILD_FAILED", err.to_string(), false))?;

        let response = tokio::runtime::Handle::current()
            .block_on(async {
                AsyncIppClient::builder(uri)
                    .request_timeout(Duration::from_secs(10))
                    .build()
                    .send(operation)
                    .await
            })
            .map_err(|err| classify_ipp_client_error("IPP_RECONCILE_FAILED", err.to_string()))?;

        if !response.header().status_code().is_success() {
            return Err(BackendError::new(
                "IPP_RECONCILE_STATUS_FAILED",
                format!(
                    "ipp reconcile failed with status {:?}",
                    response.header().status_code()
                ),
                is_ipp_retryable_status(response.header().status_code()),
            ));
        }

        let matches = find_ipp_job_ids_by_name(response.attributes(), job_name);
        match matches.as_slice() {
            [] => Ok(None),
            [job_id] => Ok(Some(encode_ipp_backend_job_ref(printer_uri, *job_id))),
            _ => Err(BackendError::new(
                "IPP_RECONCILE_AMBIGUOUS",
                format!(
                    "multiple ipp jobs matched job_name={job_name} on printer_uri={printer_uri}"
                ),
                false,
            )),
        }
    }

    fn query_job_status(
        &self,
        backend_job_ref_json: &str,
    ) -> Result<BackendJobState, BackendError> {
        let (printer_uri, job_id) = decode_ipp_backend_job_ref(backend_job_ref_json)?;
        let uri = printer_uri
            .parse::<Uri>()
            .map_err(|err| BackendError::new("IPP_PRINTER_URI_INVALID", err.to_string(), false))?;
        let operation = IppOperationBuilder::get_job_attributes(uri.clone(), job_id)
            .build()
            .map_err(|err| BackendError::new("IPP_REQUEST_BUILD_FAILED", err.to_string(), false))?;

        let response = tokio::runtime::Handle::current()
            .block_on(async {
                AsyncIppClient::builder(uri)
                    .request_timeout(Duration::from_secs(10))
                    .build()
                    .send(operation)
                    .await
            })
            .map_err(|err| classify_ipp_client_error("IPP_QUERY_FAILED", err.to_string()))?;

        if !response.header().status_code().is_success() {
            return Err(BackendError::new(
                "IPP_QUERY_STATUS_FAILED",
                format!(
                    "ipp query failed with status {:?}",
                    response.header().status_code()
                ),
                is_ipp_retryable_status(response.header().status_code()),
            ));
        }

        Ok(map_ipp_job_state(
            extract_job_state(response.attributes()).unwrap_or(JobState::Pending),
        ))
    }

    fn cancel_job(&self, backend_job_ref_json: &str) -> Result<(), BackendError> {
        let (printer_uri, job_id) = decode_ipp_backend_job_ref(backend_job_ref_json)?;
        let uri = printer_uri
            .parse::<Uri>()
            .map_err(|err| BackendError::new("IPP_PRINTER_URI_INVALID", err.to_string(), false))?;
        let operation = IppOperationBuilder::cancel_job(uri.clone(), job_id)
            .build()
            .map_err(|err| BackendError::new("IPP_REQUEST_BUILD_FAILED", err.to_string(), false))?;

        let response = tokio::runtime::Handle::current()
            .block_on(async {
                AsyncIppClient::builder(uri)
                    .request_timeout(Duration::from_secs(10))
                    .build()
                    .send(operation)
                    .await
            })
            .map_err(|err| classify_ipp_client_error("IPP_CANCEL_FAILED", err.to_string()))?;

        if !response.header().status_code().is_success() {
            return Err(BackendError::new(
                "IPP_CANCEL_STATUS_FAILED",
                format!(
                    "ipp cancel failed with status {:?}",
                    response.header().status_code()
                ),
                is_ipp_retryable_status(response.header().status_code()),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IppBackendJobRef {
    printer_uri: String,
    job_id: i32,
}

fn build_ipp_job_attributes(options: &PrintOptions) -> Result<Vec<IppAttribute>, BackendError> {
    let mut attributes = Vec::new();

    if let Some(copies) = options.copies.filter(|value| *value > 0) {
        attributes.push(
            IppAttribute::with_name(IppAttribute::COPIES, IppValue::Integer(i32::from(copies)))
                .map_err(|err| {
                    BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false)
                })?,
        );
    }

    if let Some(media) = options
        .media
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        attributes.push(
            IppAttribute::with_name(
                "media",
                IppValue::Keyword(media.try_into().map_err(
                    |err: ipp::parser::IppParseError| {
                        BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false)
                    },
                )?),
            )
            .map_err(|err| BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false))?,
        );
    }

    if let Some(sides) = options.sides {
        attributes.push(
            IppAttribute::with_name(
                IppAttribute::SIDES,
                IppValue::Keyword(sides.as_ipp_keyword().try_into().map_err(
                    |err: ipp::parser::IppParseError| {
                        BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false)
                    },
                )?),
            )
            .map_err(|err| BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false))?,
        );
    }

    if let Some(color_mode) = options.print_color_mode {
        attributes.push(
            IppAttribute::with_name(
                "print-color-mode",
                IppValue::Keyword(color_mode.as_ipp_keyword().try_into().map_err(
                    |err: ipp::parser::IppParseError| {
                        BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false)
                    },
                )?),
            )
            .map_err(|err| BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false))?,
        );
    }

    if let Some(media_type) = options
        .media_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        attributes.push(
            IppAttribute::with_name(
                "media-type",
                IppValue::Keyword(media_type.try_into().map_err(
                    |err: ipp::parser::IppParseError| {
                        BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false)
                    },
                )?),
            )
            .map_err(|err| BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false))?,
        );
    }

    if let Some(orientation) = options.orientation_requested {
        attributes.push(
            IppAttribute::with_name(
                "orientation-requested",
                IppValue::Enum(orientation.as_ipp_enum()),
            )
            .map_err(|err| BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false))?,
        );
    }

    if let Some(scaling) = options.print_scaling {
        attributes.push(
            IppAttribute::with_name(
                "print-scaling",
                IppValue::Keyword(scaling.as_ipp_keyword().try_into().map_err(
                    |err: ipp::parser::IppParseError| {
                        BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false)
                    },
                )?),
            )
            .map_err(|err| BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false))?,
        );
    }

    if let Some(page_ranges) = options
        .page_ranges
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let ranges = parse_page_ranges(page_ranges)?;
        let values = ranges
            .into_iter()
            .map(|(min, max)| IppValue::RangeOfInteger { min, max })
            .collect::<Vec<_>>();
        attributes.push(
            IppAttribute::with_name("page-ranges", IppValue::Array(values)).map_err(|err| {
                BackendError::new("IPP_ATTRIBUTE_INVALID", err.to_string(), false)
            })?,
        );
    }

    Ok(attributes)
}

fn parse_page_ranges(input: &str) -> Result<Vec<(i32, i32)>, BackendError> {
    let mut ranges = Vec::new();

    for segment in input.split(|ch: char| ch == ',' || ch.is_ascii_whitespace()) {
        let part = segment.trim();
        if part.is_empty() {
            continue;
        }

        let (min, max) = if let Some((start, end)) = part.split_once('-') {
            let min = parse_positive_page_number(start)?;
            let max = parse_positive_page_number(end)?;
            if max < min {
                return Err(BackendError::new(
                    "IPP_PAGE_RANGES_INVALID",
                    format!("invalid page range: {part}"),
                    false,
                ));
            }
            (min, max)
        } else {
            let page = parse_positive_page_number(part)?;
            (page, page)
        };

        ranges.push((min, max));
    }

    if ranges.is_empty() {
        return Err(BackendError::new(
            "IPP_PAGE_RANGES_INVALID",
            "page_ranges must contain at least one page".to_string(),
            false,
        ));
    }

    Ok(ranges)
}

fn parse_positive_page_number(input: &str) -> Result<i32, BackendError> {
    let value = input.trim().parse::<i32>().map_err(|_| {
        BackendError::new(
            "IPP_PAGE_RANGES_INVALID",
            format!("invalid page number: {input}"),
            false,
        )
    })?;
    if value <= 0 {
        return Err(BackendError::new(
            "IPP_PAGE_RANGES_INVALID",
            format!("page number must be positive: {input}"),
            false,
        ));
    }
    Ok(value)
}

fn classify_ipp_client_error(code: &'static str, message: String) -> BackendError {
    let lower = message.to_ascii_lowercase();
    let retryable = lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("connection refused")
        || lower.contains("connection reset")
        || lower.contains("temporar")
        || lower.contains("unavailable")
        || lower.contains("busy")
        || lower.contains("network");
    BackendError::new(code, message, retryable)
}

fn is_ipp_retryable_status(status: StatusCode) -> bool {
    (status as u16) >= 0x0500
}

fn extract_job_id(attributes: &IppAttributes) -> Option<i32> {
    attributes
        .groups()
        .iter()
        .find_map(|group| group.attributes().get("job-id"))
        .and_then(|attribute| {
            attribute
                .value()
                .as_integer()
                .copied()
                .or_else(|| attribute.value().as_enum().copied())
        })
}

fn extract_job_state(attributes: &IppAttributes) -> Option<JobState> {
    attributes
        .groups()
        .iter()
        .find_map(|group| group.attributes().get("job-state"))
        .and_then(|attribute| {
            attribute
                .value()
                .as_enum()
                .copied()
                .or_else(|| attribute.value().as_integer().copied())
        })
        .and_then(|raw| match raw {
            3 => Some(JobState::Pending),
            4 => Some(JobState::PendingHeld),
            5 => Some(JobState::Processing),
            6 => Some(JobState::ProcessingStopped),
            7 => Some(JobState::Canceled),
            8 => Some(JobState::Aborted),
            9 => Some(JobState::Completed),
            _ => None,
        })
}

fn find_ipp_job_ids_by_name(attributes: &IppAttributes, expected_job_name: &str) -> Vec<i32> {
    let mut matches = Vec::new();

    for group in attributes.groups() {
        let attrs = group.attributes();
        let Some(job_name) = attrs
            .get(IppAttribute::JOB_NAME)
            .and_then(|attribute| extract_ipp_text_value(attribute.value()))
        else {
            continue;
        };

        if job_name != expected_job_name {
            continue;
        }

        let Some(job_id) = attrs
            .get(IppAttribute::JOB_ID)
            .and_then(|attribute| extract_ipp_integer_value(attribute.value()))
        else {
            continue;
        };

        matches.push(job_id);
    }

    matches.sort_unstable();
    matches.dedup();
    matches
}

fn extract_ipp_text_value(value: &IppValue) -> Option<&str> {
    match value {
        IppValue::TextWithoutLanguage(text) | IppValue::OctetString(text) => Some(text.as_ref()),
        IppValue::NameWithoutLanguage(name) => Some(name.as_ref()),
        IppValue::TextWithLanguage { text, .. } => Some(text.as_ref()),
        IppValue::NameWithLanguage { name, .. } => Some(name.as_ref()),
        IppValue::Keyword(keyword) => Some(keyword.as_ref()),
        IppValue::Uri(uri) => Some(uri.as_ref()),
        _ => None,
    }
}

fn extract_ipp_integer_value(value: &IppValue) -> Option<i32> {
    match value {
        IppValue::Integer(value) | IppValue::Enum(value) => Some(*value),
        _ => None,
    }
}

fn map_ipp_job_state(state: JobState) -> BackendJobState {
    match state {
        JobState::Pending | JobState::PendingHeld => BackendJobState::Pending,
        JobState::Processing | JobState::ProcessingStopped => BackendJobState::Processing,
        JobState::Completed => BackendJobState::Completed,
        JobState::Canceled => BackendJobState::Canceled,
        JobState::Aborted => BackendJobState::Failed,
    }
}

fn encode_ipp_backend_job_ref(printer_uri: &str, job_id: i32) -> String {
    serde_json::to_string(&IppBackendJobRef {
        printer_uri: printer_uri.to_string(),
        job_id,
    })
    .expect("ipp backend job ref should serialize")
}

fn decode_ipp_backend_job_ref(encoded: &str) -> Result<(String, i32), BackendError> {
    serde_json::from_str::<IppBackendJobRef>(encoded)
        .map(|value| (value.printer_uri, value.job_id))
        .map_err(|err| BackendError::new("INVALID_BACKEND_JOB_REF_JSON", err.to_string(), false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_ipp_backend_job_ref_roundtrip() {
        let encoded = encode_ipp_backend_job_ref("ipp://printer.local/ipp/print", 42);
        let decoded = decode_ipp_backend_job_ref(&encoded).expect("should decode");
        assert_eq!(decoded.0, "ipp://printer.local/ipp/print");
        assert_eq!(decoded.1, 42);
    }

    #[test]
    fn decode_ipp_backend_job_ref_rejects_invalid_json() {
        let err = decode_ipp_backend_job_ref("not-json").expect_err("invalid json should fail");
        assert_eq!(err.code(), "INVALID_BACKEND_JOB_REF_JSON");
    }

    #[test]
    fn build_ipp_job_attributes_maps_supported_print_options() {
        let attributes = build_ipp_job_attributes(&PrintOptions {
            copies: Some(2),
            sides: Some(SidesMode::TwoSidedShortEdge),
            print_color_mode: Some(PrintColorMode::Monochrome),
            media: Some("iso_a4_210x297mm".to_string()),
            media_type: Some("photographic-glossy".to_string()),
            orientation_requested: Some(OrientationRequested::Landscape),
            print_scaling: Some(PrintScaling::Fit),
            page_ranges: Some("1-3 5".to_string()),
        })
        .expect("build ipp attributes");

        assert!(attributes
            .iter()
            .any(|attr| attr.name().as_ref() == "copies"));
        assert!(attributes
            .iter()
            .any(|attr| attr.name().as_ref() == "sides"));
        assert!(attributes
            .iter()
            .any(|attr| attr.name().as_ref() == "print-color-mode"));
        assert!(attributes
            .iter()
            .any(|attr| attr.name().as_ref() == "media"));
        assert!(attributes
            .iter()
            .any(|attr| attr.name().as_ref() == "media-type"));
        assert!(attributes
            .iter()
            .any(|attr| attr.name().as_ref() == "orientation-requested"));
        assert!(attributes
            .iter()
            .any(|attr| attr.name().as_ref() == "print-scaling"));
        assert!(attributes
            .iter()
            .any(|attr| attr.name().as_ref() == "page-ranges"));
    }

    #[test]
    fn build_ipp_job_attributes_sends_portrait_orientation_explicitly() {
        let attributes = build_ipp_job_attributes(&PrintOptions {
            orientation_requested: Some(OrientationRequested::Portrait),
            ..PrintOptions::default()
        })
        .expect("build ipp attributes");

        assert!(attributes
            .iter()
            .any(|attr| attr.name().as_ref() == "orientation-requested"));
    }

    #[test]
    fn parse_page_ranges_accepts_ranges_and_single_pages() {
        let ranges = parse_page_ranges("1-3, 5 7-9").expect("parse page ranges");
        assert_eq!(ranges, vec![(1, 3), (5, 5), (7, 9)]);
    }

    #[test]
    fn parse_page_ranges_rejects_invalid_input() {
        let err = parse_page_ranges("3-1").expect_err("descending page range should fail");
        assert_eq!(err.code(), "IPP_PAGE_RANGES_INVALID");
    }

    #[test]
    fn find_ipp_job_ids_by_name_returns_unique_matches() {
        let mut attributes = IppAttributes::new();
        attributes.add(
            ipp::model::DelimiterTag::JobAttributes,
            IppAttribute::with_name(
                IppAttribute::JOB_NAME,
                IppValue::NameWithoutLanguage("deepprint:job-1".try_into().expect("job name")),
            )
            .expect("build job-name"),
        );
        attributes.add(
            ipp::model::DelimiterTag::JobAttributes,
            IppAttribute::with_name(IppAttribute::JOB_ID, IppValue::Integer(101))
                .expect("build job-id"),
        );
        attributes.add(
            ipp::model::DelimiterTag::JobAttributes,
            IppAttribute::with_name(
                IppAttribute::JOB_NAME,
                IppValue::NameWithoutLanguage("deepprint:job-2".try_into().expect("job name")),
            )
            .expect("build job-name 2"),
        );
        attributes.add(
            ipp::model::DelimiterTag::JobAttributes,
            IppAttribute::with_name(IppAttribute::JOB_ID, IppValue::Integer(202))
                .expect("build job-id 2"),
        );

        let matches = find_ipp_job_ids_by_name(&attributes, "deepprint:job-2");
        assert_eq!(matches, vec![202]);
    }
}
