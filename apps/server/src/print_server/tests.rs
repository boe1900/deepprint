use super::auth::{
    delete_auth_user_for_path, hash_password, insert_api_key_record, insert_local_auth_user,
    normalize_auth_provider_key, require_api_key_scope, revoke_api_key_record, verify_password,
};
use super::background_tasks::{monitor_printing_jobs, monitor_submitting_jobs};
use super::http_server::build_test_http_app;
use super::jobs::{cancel_job, list_jobs, list_recent_jobs, ListJobsQuery, ListRecentJobsQuery};
use super::models::{CreateApiKeyRequest, CreateUserRequest, ProcessJobError};
use super::open::{
    open_create_direct_job, open_create_job, open_get_job, open_get_job_by_request_id,
    open_get_printer_detail, open_list_printers, open_list_templates, open_me, open_preview_typst,
    OpenPreviewRequest, OpenPrintRequest,
};
use super::printer_routes::{
    create_printer, delete_printer, disable_printer, enable_printer, get_printer_detail,
    list_printers, refresh_printer, set_default_printer, validate_printer,
};
use super::shared::{handle_job_failure, sha256_hex};
use super::*;
use crate::print_server::diagnostic_fs::{apply_log_retention, cleanup_old_diagnostic_bundles};
use crate::print_server::utils::{normalize_pagination, validate_preview_typst_payload};
use crate::storage::{
    init_schema, load_queue_metrics_snapshot, recover_inflight_jobs, transition_job_status,
    update_template_record, METRIC_DEAD_LETTER_TOTAL, METRIC_RETRY_SCHEDULED_TOTAL,
};
use axum::{
    extract::{Path as AxumPath, Query},
    routing::post,
    Router,
};
use std::{
    fs,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use tower::ServiceExt;

#[path = "tests/job_lifecycle.rs"]
mod job_lifecycle;
#[path = "tests/storage_and_listing.rs"]
mod storage_and_listing;
#[path = "tests/submission_and_printers.rs"]
mod submission_and_printers;
#[path = "tests/template_and_auth.rs"]
mod template_and_auth;
#[path = "tests/typst_and_utils.rs"]
mod typst_and_utils;

fn init_test_cleanup_schema(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE render_cache (
               cache_key TEXT PRIMARY KEY,
               artifact_path TEXT NOT NULL,
               artifact_size_bytes INTEGER NOT NULL,
               created_at INTEGER NOT NULL,
               updated_at INTEGER NOT NULL
             );
             CREATE TABLE jobs (
               status TEXT,
               render_artifact_path TEXT
             );
             CREATE TABLE agent_metrics (
               key TEXT PRIMARY KEY,
               value INTEGER NOT NULL
             );",
    )
    .expect("create cleanup test schema");
}

fn build_test_agent_state(db_path: PathBuf) -> Arc<AgentState> {
    let config = AgentConfig {
        mock_mode: true,
        ..AgentConfig::default()
    };
    build_test_agent_state_with_config(db_path, config)
}

#[derive(Debug)]
struct TestReconcileBackend {
    reconcile_result:
        std::sync::Mutex<Option<Result<Option<String>, crate::printer::BackendError>>>,
    query_result: std::sync::Mutex<
        Option<Result<crate::printer::BackendJobState, crate::printer::BackendError>>,
    >,
}

impl PrinterBackend for TestReconcileBackend {
    fn backend_name(&self) -> &'static str {
        "test"
    }

    fn list_printers(
        &self,
    ) -> Result<Vec<crate::printer::PrinterInfo>, crate::printer::BackendError> {
        Ok(vec![])
    }

    fn submit_job(
        &self,
        _req: &SubmitJobRequest,
    ) -> Result<crate::printer::SubmitJobResult, crate::printer::BackendError> {
        Err(crate::printer::BackendError::new(
            "TEST_SUBMIT_UNEXPECTED",
            "submit_job should not be called in reconcile tests",
            false,
        ))
    }

    fn reconcile_submission(
        &self,
        _printer_uri: &str,
        _job_name: &str,
        _submit_started_at: Option<i64>,
    ) -> Result<Option<String>, crate::printer::BackendError> {
        self.reconcile_result
            .lock()
            .expect("lock reconcile_result")
            .take()
            .unwrap_or(Ok(None))
    }

    fn query_job_status(
        &self,
        _backend_job_ref_json: &str,
    ) -> Result<crate::printer::BackendJobState, crate::printer::BackendError> {
        self.query_result
            .lock()
            .expect("lock query_result")
            .take()
            .unwrap_or(Ok(crate::printer::BackendJobState::Unknown))
    }
}

fn build_test_agent_state_with_config(db_path: PathBuf, config: AgentConfig) -> Arc<AgentState> {
    build_test_agent_state_with_backend_and_config(
        db_path,
        crate::printer::create_backend(true),
        config,
    )
}

fn build_test_agent_state_with_backend(
    db_path: PathBuf,
    backend: Arc<dyn PrinterBackend>,
) -> Arc<AgentState> {
    let config = AgentConfig {
        mock_mode: true,
        ..AgentConfig::default()
    };
    build_test_agent_state_with_backend_and_config(db_path, backend, config)
}

fn build_test_agent_state_with_backend_and_config(
    db_path: PathBuf,
    backend: Arc<dyn PrinterBackend>,
    config: AgentConfig,
) -> Arc<AgentState> {
    seed_test_printer(&db_path);
    let local_root = build_test_temp_dir("typst-local-root");
    let preview_root = build_test_temp_dir("typst-preview-root");
    let fonts_root = build_test_temp_dir("typst-fonts-root");
    Arc::new(AgentState {
        database_target: Arc::new(DatabaseTarget::Sqlite {
            path: db_path.clone(),
        }),
        db_path: Arc::new(db_path),
        started_at: Instant::now(),
        version: "test".to_string(),
        config,
        cups_base_url: Arc::new(RwLock::new("http://127.0.0.1:631/".to_string())),
        backend,
        typst_local_packages_root: Arc::new(local_root),
        typst_preview_cache_root: Arc::new(preview_root),
        typst_fonts_root: Arc::new(fonts_root),
    })
}

fn build_api_key_headers_for_scopes(conn: &Connection, scopes: &[&str]) -> HeaderMap {
    let user = insert_local_auth_user(
        conn,
        &CreateUserRequest {
            username: format!("admin-{}", Uuid::new_v4().simple()),
            password: "Admin123!".to_string(),
            email: None,
            display_name: Some("Open API Test Admin".to_string()),
            role: Some(USER_ROLE_ADMIN.to_string()),
        },
        now_unix(),
    )
    .expect("create api key owner");

    let (_api_key, token) = insert_api_key_record(
        conn,
        &CreateApiKeyRequest {
            name: "Open API Test Key".to_string(),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
            expires_at: Some(now_unix() + 3600),
        },
        &user.id,
    )
    .expect("create api key");

    bearer_headers(&token)
}

fn bearer_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).expect("valid bearer header"),
    );
    headers
}

fn build_open_direct_multipart_body(boundary: &str, request_id: &str) -> String {
    format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"request_id\"\r\n\r\n\
         {request_id}\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"printer_id\"\r\n\r\n\
         test-printer\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"print_options\"\r\n\r\n\
         {{\"copies\":1}}\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"external.pdf\"\r\n\
         Content-Type: application/pdf\r\n\r\n\
         %PDF-1.7\n\r\n\
         --{boundary}--\r\n"
    )
}

fn seed_test_printer(db_path: &Path) {
    let conn = open_conn(db_path).expect("open conn for test printer seed");
    let capabilities = serde_json::json!({
        "document_formats": ["application/pdf"],
        "media_supported": ["iso_a4_210x297mm", "na_letter_8.5x11in"],
        "media_default": "iso_a4_210x297mm",
        "media_types_supported": ["stationery", "photographic", "photographic-glossy"],
        "sides_supported": ["one-sided", "two-sided-long-edge"],
        "sides_default": "one-sided",
        "copies": { "default": 1, "min": 1, "max": 10 },
        "color_modes_supported": ["color", "monochrome"],
        "color_supported": true,
        "orientations_supported": ["portrait", "landscape"],
        "scalings_supported": ["auto", "auto-fit", "fit", "fill", "none"],
        "supports_page_ranges": true,
        "job_creation_attributes_supported": [
            "copies",
            "media",
            "media-type",
            "sides",
            "print-color-mode",
            "orientation-requested",
            "print-scaling",
            "page-ranges"
        ]
    });
    conn.execute(
        "INSERT OR IGNORE INTO printers (
                id,
                source,
                display_name,
                printer_uri,
                normalized_uri,
                is_default,
                is_enabled,
                last_known_state,
                last_state_message,
                capabilities_json,
                attributes_json,
                last_seen_at,
                last_validated_at,
                last_refreshed_at,
                created_at,
                updated_at
             ) VALUES (
                'test-printer',
                'manual',
                'Test Printer',
                'mock:printer',
                'mock:printer',
                1,
                1,
                'idle',
                NULL,
                ?2,
                '{}',
                ?1,
                ?1,
                ?1,
                ?1,
                ?1
             )",
        params![now_unix(), capabilities.to_string()],
    )
    .expect("seed test printer");
}

fn seed_additional_test_printer(
    conn: &Connection,
    id: &str,
    display_name: &str,
    is_default: bool,
    is_enabled: bool,
) {
    conn.execute(
        "INSERT INTO printers (
                id,
                source,
                display_name,
                printer_uri,
                normalized_uri,
                is_default,
                is_enabled,
                last_known_state,
                last_state_message,
                capabilities_json,
                attributes_json,
                last_seen_at,
                last_validated_at,
                last_refreshed_at,
                created_at,
                updated_at
             ) VALUES (
                ?1,
                'manual',
                ?2,
                ?3,
                ?3,
                ?4,
                ?5,
                'idle',
                NULL,
                '{}',
                '{}',
                ?6,
                ?6,
                ?6,
                ?6,
                ?6
             )",
        params![
            id,
            display_name,
            format!("mock:{id}"),
            if is_default { 1 } else { 0 },
            if is_enabled { 1 } else { 0 },
            now_unix(),
        ],
    )
    .expect("seed additional test printer");
}

fn build_create_job_request(request_id: &str) -> CreateJobRequest {
    CreateJobRequest {
        request_id: request_id.to_string(),
        printer_id: "test-printer".to_string(),
        template_content: "#set page(width: 80mm, height: auto)\nHello #data.orderNo".to_string(),
        data: serde_json::json!({ "orderNo": "A1001" }),
        print_options: PrintOptions::default(),
    }
}

fn insert_open_api_test_template(conn: &Connection) -> String {
    let group = insert_template_group(conn, "Open API Tests").expect("insert open api group");
    let template = insert_template_record(
        conn,
        normalize_template_payload(
            &group.id,
            "Open API Template",
            "Used by open api tests",
            "open-api.pdf",
            "#set page(width: 80mm, height: auto)\nHello #data.orderNo",
            "{ \"orderNo\": \"A1001\" }",
        )
        .expect("normalize open api template"),
    )
    .expect("insert open api template");
    template.id
}

fn build_create_direct_job_request(
    request_id: &str,
    file_name: &str,
    bytes: &[u8],
) -> CreateDirectJobRequest {
    CreateDirectJobRequest {
        request_id: request_id.to_string(),
        printer_id: "test-printer".to_string(),
        file_name: file_name.to_string(),
        file_content_base64: BASE64_STANDARD.encode(bytes),
        content_type: Some("application/pdf".to_string()),
        print_options: PrintOptions::default(),
    }
}

fn insert_test_job(
    conn: &Connection,
    id: &str,
    request_id: &str,
    status: &str,
    last_error_code: Option<&str>,
    last_error_message: Option<&str>,
) {
    let now = now_unix();
    conn.execute(
        "INSERT INTO jobs (
               id, request_id, template_content, data_json, print_options_json,
               status, attempt_count, last_error_code, last_error_message, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?8, ?9, ?9)",
        params![
            id,
            request_id,
            "#set page(width: 80mm)",
            "{\"orderNo\":\"A1\"}",
            "{}",
            status,
            last_error_code,
            last_error_message,
            now,
        ],
    )
    .expect("insert test job");
}

fn insert_test_job_with_printer(
    conn: &Connection,
    id: &str,
    request_id: &str,
    status: &str,
    printer_id: Option<&str>,
) {
    let now = now_unix();
    let printer_name_snapshot = printer_id.map(|value| format!("printer-{value}"));
    let printer_uri = printer_id.map(|value| format!("mock:{value}"));
    conn.execute(
        "INSERT INTO jobs (
               id,
               request_id,
               job_kind,
               printer_id,
               printer_name_snapshot,
               printer_uri,
               template_content,
               data_json,
               print_options_json,
               status,
               attempt_count,
               created_at,
               updated_at
             ) VALUES (?1, ?2, 'template', ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10, ?10)",
        params![
            id,
            request_id,
            printer_id,
            printer_name_snapshot,
            printer_uri,
            "#set page(width: 80mm)",
            "{\"orderNo\":\"A1\"}",
            "{}",
            status,
            now,
        ],
    )
    .expect("insert test job with printer");
}

fn build_test_db_path(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("deepprint-agent-db-{label}-{}", Uuid::new_v4()));
    fs::create_dir_all(&dir).expect("create temp test db dir");
    dir.join("agent.db")
}

fn build_test_temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("deepprint-agent-test-{label}-{}", Uuid::new_v4()));
    fs::create_dir_all(&dir).expect("create temp test dir");
    dir
}
