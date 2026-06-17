use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Instant,
};

#[path = "print_server/auth.rs"]
mod auth;
#[path = "print_server/background_tasks.rs"]
mod background_tasks;
#[path = "print_server/bootstrap.rs"]
mod bootstrap;
#[path = "print_server/config.rs"]
mod config;
#[path = "print_server/diagnostic_fs.rs"]
mod diagnostic_fs;
#[path = "print_server/diagnostics.rs"]
mod diagnostics;
#[path = "print_server/http_server.rs"]
mod http_server;
#[path = "print_server/jobs.rs"]
mod jobs;
#[path = "print_server/models.rs"]
mod models;
#[path = "print_server/open.rs"]
mod open;
#[path = "print_server/platform_paths.rs"]
mod platform_paths;
#[path = "print_server/printer_routes.rs"]
mod printer_routes;
#[path = "print_server/rendering.rs"]
mod rendering;
#[path = "print_server/runtime.rs"]
mod runtime;
#[path = "print_server/settings.rs"]
mod settings;
#[path = "print_server/shared.rs"]
mod shared;
#[path = "print_server/submission.rs"]
mod submission;
#[path = "print_server/template.rs"]
mod template;
#[path = "print_server/typst_assets.rs"]
mod typst_assets;
#[path = "print_server/utils.rs"]
mod utils;
#[path = "print_server/worker.rs"]
mod worker;

#[cfg(test)]
use self::auth::api_key_prefix_from_token;
#[cfg(test)]
use self::submission::{create_direct_job, create_job};
#[cfg(test)]
use self::submission::{CreateDirectJobRequest, CreateJobRequest, PreviewTypstRequest};
#[cfg(test)]
use self::template::{
    build_template_workspace_response, delete_template_group_record, normalize_template_payload,
};
#[cfg(test)]
use self::typst_assets::{
    collect_typst_fonts, collect_typst_packages_from_namespace, ensure_default_typst_fonts,
    locate_typst_package_root, read_typst_package_manifest, sanitize_package_segment,
    sanitize_typst_font_file_name, validate_install_typst_font_payload,
    validate_install_typst_package_payload, InstallTypstFontRequest, InstallTypstPackageRequest,
    TypstPackageOrigin,
};
#[cfg(test)]
use crate::print_backend::registry as printer_registry;

#[cfg(test)]
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, HeaderValue, Request, StatusCode},
    response::IntoResponse,
    Json,
};
#[cfg(test)]
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
#[cfg(test)]
use base64::Engine as _;
#[cfg(test)]
use rusqlite::{params, Connection};
#[cfg(test)]
use uuid::Uuid;

pub use self::config::{AgentBootError, AgentConfig, PrintServerBootError, PrintServerConfig};
use self::models::{ApiError, ApiResult, ProcessJobError};
pub use self::runtime::run_print_server;
#[cfg(test)]
use self::shared::JOB_KIND_TEMPLATE;
use self::shared::{
    sha256_hex, API_SCOPE_JOB_READ, API_SCOPE_PREVIEW_CREATE, API_SCOPE_PRINTER_READ,
    API_SCOPE_PRINT_CREATE, API_SCOPE_TEMPLATE_READ, DEFAULT_JOBS_PAGE_SIZE,
    DEFAULT_RECENT_JOBS_LIMIT, ENV_TYPST_FONTS_ROOT, ENV_TYPST_LOCAL_PACKAGES_ROOT,
    ENV_TYPST_PREVIEW_CACHE_ROOT, JOB_KIND_DIRECT_FILE, MAX_JOBS_PAGE_SIZE,
    MAX_RECENT_JOBS_LIMIT, MAX_TYPST_FONT_FILE_BYTES, MAX_TYPST_PACKAGE_ARCHIVE_BYTES,
    PREVIEW_EXPOSE_HEADERS, PREVIEW_HEADER_OUTPUT_KIND, PREVIEW_HEADER_PAGE_COUNT,
    PREVIEW_HEADER_PAGE_HEIGHT_PT, PREVIEW_HEADER_PAGE_WIDTH_PT, SUPPORTED_TYPST_FONT_EXTENSIONS,
};
use crate::print_backend::{
    ipp as ipp_backend, mock as mock_print_backend, AddPrinterRequest, AddPrinterResponse,
    CreatePrinterRecord, DeletePrinterResponse, DiscoveredPrintersResponse, PrinterDetail,
    PrinterSummary, PrinterTargetInput, PrintersListResponse, RefreshPrinterSnapshotInput,
    ValidatePrinterRequest, ValidatedPrinterTarget,
};
#[cfg(test)]
use crate::printer::SubmitJobRequest;
use crate::printer::{BackendJobState, PrintOptions, PrinterBackend};
use crate::storage::{
    increment_agent_metric, try_insert_job_event, AuthSessionRecord, AuthUserRecord,
    DatabaseTarget, JobRecord, TemplateGroupRecord, TemplateRecordRow,
    API_KEY_STATUS_ACTIVE, METRIC_LOG_CLEANUP_TOTAL, USER_ROLE_ADMIN, USER_ROLE_OPERATOR,
    USER_STATUS_ACTIVE, USER_STATUS_DISABLED,
};
#[cfg(test)]
use serde_json::Value;
use utils::{mb_to_bytes, now_unix};

#[cfg(test)]
use crate::storage::open_sqlite_connection as open_conn;
#[cfg(test)]
use crate::storage::{
    claim_next_job, count_active_admins, delete_template_record, fetch_job_by_id,
    has_active_local_auth_user, insert_auth_session, insert_template_group, insert_template_record,
    load_api_key_by_prefix_and_hash, load_auth_session, load_auth_user, load_failed_jobs_snapshot,
    load_local_auth_identity, load_local_auth_identity_by_user_id, revoke_auth_session,
    touch_auth_session, update_local_auth_password,
};
#[cfg(test)]
use crate::storage::{has_table_column, read_agent_metric};

#[derive(Clone)]
pub(crate) struct AgentState {
    database_target: Arc<DatabaseTarget>,
    db_path: Arc<PathBuf>,
    started_at: Instant,
    version: String,
    config: AgentConfig,
    cups_base_url: Arc<RwLock<String>>,
    backend: Arc<dyn PrinterBackend>,
    typst_local_packages_root: Arc<PathBuf>,
    typst_preview_cache_root: Arc<PathBuf>,
    typst_fonts_root: Arc<PathBuf>,
}

impl AgentState {
    pub(crate) fn current_cups_base_url(&self) -> String {
        self.cups_base_url
            .read()
            .expect("cups_base_url lock poisoned")
            .clone()
    }

    pub(crate) fn set_cups_base_url(&self, cups_base_url: String) {
        *self
            .cups_base_url
            .write()
            .expect("cups_base_url lock poisoned") = cups_base_url;
    }
}

#[cfg(test)]
async fn api_error_payload(err: ApiError) -> (StatusCode, Value) {
    let response = err.into_response();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read error response body");
    let payload: Value = serde_json::from_slice(&body).expect("parse error response json");
    (status, payload)
}

#[cfg(test)]
#[path = "print_server/tests.rs"]
mod tests;
