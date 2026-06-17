use super::*;

async fn assert_required_scope(err: ApiError, required_scope: &str) {
    let (status, payload) = api_error_payload(err).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(payload["code"], "API_KEY_SCOPE_REQUIRED");
    assert_eq!(
        payload["message"],
        format!("api key scope required: {required_scope}")
    );
}

async fn assert_missing_api_key(err: ApiError) {
    let (status, payload) = api_error_payload(err).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(payload["code"], "UNAUTHORIZED");
}

async fn read_response_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    serde_json::from_slice(&body).expect("parse response json")
}

async fn read_response_bytes(response: axum::response::Response) -> Vec<u8> {
    to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body")
        .to_vec()
}

fn request_with_json(method: &str, uri: &str, headers: &HeaderMap, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(
            AUTHORIZATION,
            headers.get(AUTHORIZATION).expect("authorization header"),
        )
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build json request")
}

fn request_without_body(method: &str, uri: &str, headers: &HeaderMap) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(
            AUTHORIZATION,
            headers.get(AUTHORIZATION).expect("authorization header"),
        )
        .body(Body::empty())
        .expect("build request")
}

#[test]
fn validate_preview_typst_payload_rejects_empty_template() {
    let payload = PreviewTypstRequest {
        template_content: "   ".to_string(),
        data: serde_json::json!({ "orderNo": "A1001" }),
        print_options: PrintOptions::default(),
    };
    let err =
        validate_preview_typst_payload(&payload).expect_err("empty template should be rejected");
    match err {
        ApiError::BadRequest(message) => assert!(message.contains("template_content")),
        other => panic!("expected bad request, got {other}"),
    }
}

#[test]
fn validate_preview_typst_payload_rejects_invalid_copies() {
    let payload = PreviewTypstRequest {
        template_content: "#set page(width: 80mm)\nHello #data.orderNo".to_string(),
        data: serde_json::json!({ "orderNo": "A1001" }),
        print_options: PrintOptions {
            copies: Some(101),
            ..PrintOptions::default()
        },
    };
    let err =
        validate_preview_typst_payload(&payload).expect_err("invalid copies should be rejected");
    match err {
        ApiError::BadRequest(message) => assert!(message.contains("copies")),
        other => panic!("expected bad request, got {other}"),
    }
}

#[test]
fn init_schema_seeds_template_workspace_once() {
    let db_path = build_test_db_path("template-workspace-seed");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");

    let groups: i64 = conn
        .query_row("SELECT COUNT(1) FROM template_groups", [], |row| row.get(0))
        .expect("count groups");
    let templates: i64 = conn
        .query_row("SELECT COUNT(1) FROM templates", [], |row| row.get(0))
        .expect("count templates");
    assert!(groups >= 4);
    assert!(templates >= 4);

    init_schema(&db_path).expect("re-init schema");
    let groups_after: i64 = conn
        .query_row("SELECT COUNT(1) FROM template_groups", [], |row| row.get(0))
        .expect("count groups after");
    assert_eq!(groups, groups_after);
}

#[test]
fn template_group_and_template_crud_round_trip() {
    let db_path = build_test_db_path("template-workspace-crud");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");

    let group = insert_template_group(&conn, "测试分组").expect("insert group");
    assert_eq!(group.name, "测试分组");

    let template = insert_template_record(
        &conn,
        normalize_template_payload(
            &group.id,
            "测试模板",
            "用于 CRUD 校验",
            "test-output.pdf",
            "#set page(margin: 10mm)\n= 测试模板",
            "{\n  \"name\": \"DeepPrint\"\n}",
        )
        .expect("normalize template payload"),
    )
    .expect("insert template");
    assert_eq!(template.group_id, group.id);
    assert_eq!(template.name, "测试模板");

    let updated = update_template_record(
        &conn,
        &template.id,
        normalize_template_payload(
            &group.id,
            "测试模板已更新",
            "说明已更新",
            "updated.pdf",
            "#set page(margin: 12mm)\n= 更新模板",
            "{\n  \"value\": 1\n}",
        )
        .expect("normalize updated template payload"),
    )
    .expect("update template result")
    .expect("updated template");
    assert_eq!(updated.name, "测试模板已更新");
    assert_eq!(updated.output_name, "updated.pdf");

    let workspace = build_template_workspace_response(&db_path).expect("build workspace");
    let test_group = workspace
        .groups
        .into_iter()
        .find(|item| item.id == group.id)
        .expect("find test group");
    assert_eq!(test_group.templates.len(), 1);
    assert_eq!(test_group.templates[0].id, template.id);

    let deleted = delete_template_record(&conn, &template.id).expect("delete template");
    assert!(deleted);
    let group_deleted = delete_template_group_record(&db_path, &group.id).expect("delete group");
    assert!(group_deleted);
}

#[test]
fn auth_storage_round_trip_supports_session_and_password_rotation() {
    let db_path = build_test_db_path("auth-storage-round-trip");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");

    assert!(!has_active_local_auth_user(&conn).expect("no local auth user initially"));

    let user = insert_local_auth_user(
        &conn,
        &CreateUserRequest {
            username: "admin".to_string(),
            password: "Admin123!".to_string(),
            email: Some("admin@example.com".to_string()),
            display_name: Some("Admin".to_string()),
            role: Some(USER_ROLE_ADMIN.to_string()),
        },
        now_unix(),
    )
    .expect("insert local auth user");
    assert_eq!(count_active_admins(&conn).expect("count active admins"), 1);
    assert!(has_active_local_auth_user(&conn).expect("local auth user exists"));

    let provider_key =
        normalize_auth_provider_key(&user.username).expect("normalize auth provider key");
    let identity = load_local_auth_identity(&conn, &provider_key)
        .expect("load local auth identity")
        .expect("identity exists");
    assert_eq!(identity.user.id, user.id);
    assert!(verify_password("Admin123!", &identity.password_hash).expect("verify initial password"));

    let session_token_hash = sha256_hex(b"session-token");
    let now = now_unix();
    let expires_at = now + 3600;
    insert_auth_session(
        &conn,
        &user.id,
        &session_token_hash,
        now,
        expires_at,
        Some("127.0.0.1".to_string()),
        Some("test-agent".to_string()),
    )
    .expect("insert auth session");

    let session = load_auth_session(&conn, &session_token_hash, now)
        .expect("load auth session")
        .expect("session exists");
    assert_eq!(session.user.id, user.id);
    assert_eq!(session.expires_at, expires_at);

    let updated_password_hash = hash_password("Admin123!Changed").expect("hash new password");
    update_local_auth_password(
        &conn,
        &user.id,
        &updated_password_hash,
        now + 10,
        &session_token_hash,
    )
    .expect("update local auth password");
    touch_auth_session(&conn, &session_token_hash, now + 10).expect("touch auth session");

    let updated_identity = load_local_auth_identity_by_user_id(&conn, &user.id)
        .expect("load updated identity")
        .expect("updated identity exists");
    assert!(
        verify_password("Admin123!Changed", &updated_identity.password_hash)
            .expect("verify updated password")
    );

    let updated_user = load_auth_user(&conn, &user.id)
        .expect("load updated auth user")
        .expect("updated user exists");
    assert!(!updated_user.must_change_password);

    revoke_auth_session(&conn, &session_token_hash, now + 20).expect("revoke auth session");
    assert!(load_auth_session(&conn, &session_token_hash, now + 20)
        .expect("load revoked session")
        .is_none());
}

#[test]
fn delete_auth_user_removes_login_identity_and_sessions() {
    let db_path = build_test_db_path("auth-delete-user");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let now = now_unix();

    let admin = insert_local_auth_user(
        &conn,
        &CreateUserRequest {
            username: "admin".to_string(),
            password: "Admin123!".to_string(),
            email: Some("admin@example.com".to_string()),
            display_name: Some("Admin".to_string()),
            role: Some(USER_ROLE_ADMIN.to_string()),
        },
        now,
    )
    .expect("create admin");
    let operator = insert_local_auth_user(
        &conn,
        &CreateUserRequest {
            username: "operator".to_string(),
            password: "Operator123!".to_string(),
            email: Some("operator@example.com".to_string()),
            display_name: Some("Operator".to_string()),
            role: None,
        },
        now,
    )
    .expect("create operator");

    let provider_key =
        normalize_auth_provider_key(&operator.username).expect("normalize provider key");
    let session_token_hash = sha256_hex(b"operator-session-token");
    insert_auth_session(
        &conn,
        &operator.id,
        &session_token_hash,
        now,
        now + 3600,
        None,
        None,
    )
    .expect("insert operator session");

    let deleted =
        delete_auth_user_for_path(&db_path, &operator.id, &admin.id).expect("delete operator");
    assert_eq!(deleted.id, operator.id);
    assert!(load_auth_user(&conn, &operator.id)
        .expect("load deleted user")
        .is_none());
    assert!(load_local_auth_identity(&conn, &provider_key)
        .expect("load deleted identity")
        .is_none());
    assert!(load_auth_session(&conn, &session_token_hash, now)
        .expect("load deleted session")
        .is_none());
}

#[test]
fn delete_auth_user_rejects_self_and_last_active_admin() {
    let db_path = build_test_db_path("auth-delete-user-guards");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let now = now_unix();

    let admin = insert_local_auth_user(
        &conn,
        &CreateUserRequest {
            username: "admin".to_string(),
            password: "Admin123!".to_string(),
            email: None,
            display_name: Some("Admin".to_string()),
            role: Some(USER_ROLE_ADMIN.to_string()),
        },
        now,
    )
    .expect("create admin");
    let operator = insert_local_auth_user(
        &conn,
        &CreateUserRequest {
            username: "operator".to_string(),
            password: "Operator123!".to_string(),
            email: None,
            display_name: Some("Operator".to_string()),
            role: None,
        },
        now,
    )
    .expect("create operator");

    let self_delete_err = delete_auth_user_for_path(&db_path, &admin.id, &admin.id)
        .expect_err("self delete should fail");
    match self_delete_err {
        ApiError::BadRequest(message) => assert!(message.contains("cannot delete")),
        other => panic!("expected bad request, got {other}"),
    }

    let last_admin_err = delete_auth_user_for_path(&db_path, &admin.id, &operator.id)
        .expect_err("last active admin delete should fail");
    match last_admin_err {
        ApiError::Conflict(message) => assert!(message.contains("last active admin")),
        other => panic!("expected conflict, got {other}"),
    }
}

#[test]
fn api_key_storage_round_trip_supports_create_touch_and_revoke() {
    let db_path = build_test_db_path("api-key-storage-round-trip");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let user = insert_local_auth_user(
        &conn,
        &CreateUserRequest {
            username: "admin".to_string(),
            password: "Admin123!".to_string(),
            email: Some("admin@example.com".to_string()),
            display_name: Some("Admin".to_string()),
            role: Some(USER_ROLE_ADMIN.to_string()),
        },
        now_unix(),
    )
    .expect("create admin user");

    let (api_key, token) = insert_api_key_record(
        &conn,
        &CreateApiKeyRequest {
            name: "Preview Integration".to_string(),
            scopes: vec![
                API_SCOPE_TEMPLATE_READ.to_string(),
                API_SCOPE_PREVIEW_CREATE.to_string(),
            ],
            expires_at: Some(now_unix() + 3600),
        },
        &user.id,
    )
    .expect("create api key");
    assert_eq!(api_key.status, API_KEY_STATUS_ACTIVE);

    let prefix = api_key_prefix_from_token(&token).expect("derive api key prefix");
    let loaded = load_api_key_by_prefix_and_hash(&conn, &prefix, &sha256_hex(token.as_bytes()))
        .expect("load api key by prefix and hash")
        .expect("api key exists");
    assert_eq!(loaded.id, api_key.id);

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).expect("valid bearer header"),
    );
    let authorized = require_api_key_scope(&conn, &headers, API_SCOPE_TEMPLATE_READ)
        .expect("require api key scope");
    assert_eq!(authorized.id, api_key.id);

    let touched = crate::storage::load_api_key_by_id(&conn, &api_key.id)
        .expect("load touched api key")
        .expect("touched api key exists");
    assert!(touched.last_used_at.is_some());

    let revoked = revoke_api_key_record(&conn, &api_key.id, now_unix()).expect("revoke api key");
    assert_eq!(revoked.status, crate::storage::API_KEY_STATUS_REVOKED);

    let err = require_api_key_scope(&conn, &headers, API_SCOPE_TEMPLATE_READ)
        .expect_err("revoked api key should be rejected");
    match err {
        ApiError::Unauthorized(message) => assert!(message.contains("revoked")),
        other => panic!("expected unauthorized error, got {other}"),
    }
}

#[test]
fn api_key_scopes_reject_unused_credential_manage_scope() {
    let db_path = build_test_db_path("api-key-scope-credential-manage");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let user = insert_local_auth_user(
        &conn,
        &CreateUserRequest {
            username: "admin".to_string(),
            password: "Admin123!".to_string(),
            email: None,
            display_name: Some("Admin".to_string()),
            role: Some(USER_ROLE_ADMIN.to_string()),
        },
        now_unix(),
    )
    .expect("create admin user");

    let err = insert_api_key_record(
        &conn,
        &CreateApiKeyRequest {
            name: "Unsupported Scope".to_string(),
            scopes: vec!["credential:manage".to_string()],
            expires_at: None,
        },
        &user.id,
    )
    .expect_err("credential:manage should not be accepted by open api scopes");

    match err {
        ApiError::BadRequest(message) => {
            assert!(message.contains("unsupported api key scope: credential:manage"));
        }
        other => panic!("expected bad request, got {other}"),
    }
}

#[tokio::test]
async fn open_me_returns_current_api_key_metadata() {
    let db_path = build_test_db_path("open-me");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let state = build_test_agent_state(db_path);
    let headers = build_api_key_headers_for_scopes(&conn, &[API_SCOPE_PRINT_CREATE]);

    let Json(response) = open_me(State(state), headers)
        .await
        .expect("open me should succeed");

    assert_eq!(response.api_key.name, "Open API Test Key");
    assert_eq!(response.api_key.status, API_KEY_STATUS_ACTIVE);
    assert_eq!(
        response.api_key.scopes,
        vec![API_SCOPE_PRINT_CREATE.to_string()]
    );
    assert!(!response.api_key.key_prefix.is_empty());
    assert!(response.api_key.expires_at.is_some());
}

#[tokio::test]
async fn open_me_rejects_missing_api_key() {
    let db_path = build_test_db_path("open-me-missing-key");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path);

    let err = open_me(State(state), HeaderMap::new())
        .await
        .expect_err("open me should require an api key");

    assert_missing_api_key(err).await;
}

#[tokio::test]
async fn open_printer_endpoints_require_printer_read_scope() {
    let db_path = build_test_db_path("open-printers-scope");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let state = build_test_agent_state(db_path);
    let headers = build_api_key_headers_for_scopes(&conn, &[API_SCOPE_PRINT_CREATE]);

    let err = open_list_printers(State(state), headers)
        .await
        .expect_err("printer read scope should be required");
    let (status, payload) = api_error_payload(err).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(payload["code"], "API_KEY_SCOPE_REQUIRED");
    assert_eq!(payload["message"], "api key scope required: printer:read");
}

#[tokio::test]
async fn open_printer_endpoints_return_managed_printers_and_capabilities() {
    let db_path = build_test_db_path("open-printers");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let state = build_test_agent_state(db_path);
    let headers = build_api_key_headers_for_scopes(&conn, &[API_SCOPE_PRINTER_READ]);

    let Json(list_response) = open_list_printers(State(state.clone()), headers.clone())
        .await
        .expect("open list printers should succeed");
    assert_eq!(list_response.printers.len(), 1);
    assert_eq!(list_response.printers[0].id, "test-printer");
    assert_eq!(list_response.printers[0].name, "Test Printer");

    let Json(detail) =
        open_get_printer_detail(State(state), headers, AxumPath("test-printer".to_string()))
            .await
            .expect("open printer detail should succeed");
    assert_eq!(detail.id, "test-printer");
    assert_eq!(
        detail.capabilities.media_default.as_deref(),
        Some("iso_a4_210x297mm")
    );
    assert!(detail
        .capabilities
        .job_creation_attributes_supported
        .iter()
        .any(|item| item == "print-color-mode"));
}

#[tokio::test]
async fn open_list_templates_requires_template_read_and_returns_templates() {
    let db_path = build_test_db_path("open-list-templates");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let state = build_test_agent_state(db_path);
    let headers = build_api_key_headers_for_scopes(&conn, &[API_SCOPE_TEMPLATE_READ]);

    let Json(response) = open_list_templates(State(state), headers)
        .await
        .expect("open list templates should succeed");

    let template_count = response
        .groups
        .iter()
        .map(|group| group.templates.len())
        .sum::<usize>();
    assert!(template_count > 0);
}

#[tokio::test]
async fn open_preview_typst_requires_preview_scope_and_returns_pdf() {
    let db_path = build_test_db_path("open-preview");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let template_id = insert_open_api_test_template(&conn);
    let state = build_test_agent_state(db_path);
    let headers = build_api_key_headers_for_scopes(&conn, &[API_SCOPE_PREVIEW_CREATE]);

    let response = open_preview_typst(
        State(state),
        headers,
        Json(OpenPreviewRequest {
            template_id,
            data: serde_json::json!({ "orderNo": "A1001" }),
            print_options: PrintOptions::default(),
        }),
    )
    .await
    .expect("open preview should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/pdf")
    );
}

#[tokio::test]
async fn open_create_job_requires_print_scope_and_creates_template_job() {
    let db_path = build_test_db_path("open-create-template-job");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let template_id = insert_open_api_test_template(&conn);
    let state = build_test_agent_state(db_path.clone());
    let headers = build_api_key_headers_for_scopes(&conn, &[API_SCOPE_PRINT_CREATE]);

    let (status, body) = open_create_job(
        State(state),
        headers,
        Json(OpenPrintRequest {
            request_id: "req-open-template-print".to_string(),
            template_id,
            printer_id: "test-printer".to_string(),
            data: serde_json::json!({ "orderNo": "A1001" }),
            print_options: PrintOptions::default(),
        }),
    )
    .await
    .expect("open create job should succeed");

    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(!body.0.idempotent);

    let job = fetch_job_by_id(&conn, &body.0.job_id)
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.job_kind, JOB_KIND_TEMPLATE);
    assert_eq!(job.request_id, "req-open-template-print");
    assert_eq!(job.printer_id.as_deref(), Some("test-printer"));
}

#[tokio::test]
async fn open_get_job_requires_job_read_and_returns_job_by_id() {
    let db_path = build_test_db_path("open-job-by-id");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let state = build_test_agent_state(db_path);
    let headers = build_api_key_headers_for_scopes(&conn, &[API_SCOPE_JOB_READ]);
    let (_, created) = create_job(
        State(state.clone()),
        Json(build_create_job_request("req-open-job-id")),
    )
    .await
    .expect("create job");

    let Json(job) = open_get_job(State(state), headers, AxumPath(created.0.job_id.clone()))
        .await
        .expect("open get job by id should succeed");

    assert_eq!(job.job_id, created.0.job_id);
}

#[tokio::test]
async fn open_get_job_by_request_id_returns_existing_job() {
    let db_path = build_test_db_path("open-job-by-request-id");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let state = build_test_agent_state(db_path);
    let headers = build_api_key_headers_for_scopes(&conn, &[API_SCOPE_JOB_READ]);

    let (_, created) = create_job(
        State(state.clone()),
        Json(build_create_job_request("req-open-job-lookup")),
    )
    .await
    .expect("create job");

    let Json(job) = open_get_job_by_request_id(
        State(state),
        headers,
        AxumPath("req-open-job-lookup".to_string()),
    )
    .await
    .expect("open get job by request id should succeed");

    assert_eq!(job.job_id, created.0.job_id);
}

#[tokio::test]
async fn open_create_direct_job_accepts_file_payload() {
    let db_path = build_test_db_path("open-direct-print");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let state = build_test_agent_state(db_path.clone());
    let headers = build_api_key_headers_for_scopes(&conn, &[API_SCOPE_PRINT_CREATE]);

    let boundary = "deepprint-test-boundary";
    let body = build_open_direct_multipart_body(boundary, "req-open-direct");
    let app = Router::new()
        .route("/v1/open/print/direct", post(open_create_direct_job))
        .with_state(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/open/print/direct")
                .header(
                    AUTHORIZATION,
                    headers.get(AUTHORIZATION).expect("authorization header"),
                )
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("build multipart request"),
        )
        .await
        .expect("open direct print should succeed");
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let response_body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read create response");
    let payload: Value = serde_json::from_slice(&response_body).expect("parse create response");
    assert_eq!(payload["idempotent"], false);
    let job_id = payload["job_id"].as_str().expect("job_id string");

    let job = fetch_job_by_id(&conn, job_id)
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.job_kind, JOB_KIND_DIRECT_FILE);
    assert_eq!(job.request_id, "req-open-direct");
    assert_eq!(job.source_file_name.as_deref(), Some("external.pdf"));
    assert_eq!(job.source_content_type.as_deref(), Some("application/pdf"));
}

#[tokio::test]
async fn open_create_direct_job_rejects_missing_print_scope() {
    let db_path = build_test_db_path("open-direct-print-scope");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let state = build_test_agent_state(db_path);
    let headers = build_api_key_headers_for_scopes(&conn, &[API_SCOPE_JOB_READ]);

    let boundary = "deepprint-test-boundary";
    let app = Router::new()
        .route("/v1/open/print/direct", post(open_create_direct_job))
        .with_state(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/open/print/direct")
                .header(
                    AUTHORIZATION,
                    headers.get(AUTHORIZATION).expect("authorization header"),
                )
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(build_open_direct_multipart_body(
                    boundary,
                    "req-open-direct-scope",
                )))
                .expect("build multipart request"),
        )
        .await
        .expect("open direct print request should complete");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response_body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read forbidden response");
    let payload: Value = serde_json::from_slice(&response_body).expect("parse forbidden response");
    assert_eq!(payload["code"], "API_KEY_SCOPE_REQUIRED");
    assert_eq!(payload["message"], "api key scope required: print:create");
}

#[tokio::test]
async fn open_http_endpoints_complete_functional_flow() {
    let db_path = build_test_db_path("open-http-functional-flow");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let template_id = insert_open_api_test_template(&conn);
    let state = build_test_agent_state(db_path.clone());
    let headers = build_api_key_headers_for_scopes(
        &conn,
        &[
            API_SCOPE_TEMPLATE_READ,
            API_SCOPE_PRINTER_READ,
            API_SCOPE_PREVIEW_CREATE,
            API_SCOPE_PRINT_CREATE,
            API_SCOPE_JOB_READ,
        ],
    );
    let app = build_test_http_app(state);

    let me_response = app
        .clone()
        .oneshot(request_without_body("GET", "/v1/open/me", &headers))
        .await
        .expect("open me http request");
    assert_eq!(me_response.status(), StatusCode::OK);
    let me_payload = read_response_json(me_response).await;
    assert_eq!(me_payload["api_key"]["name"], "Open API Test Key");
    assert_eq!(
        me_payload["api_key"]["scopes"]
            .as_array()
            .expect("scopes array")
            .len(),
        5
    );

    let templates_response = app
        .clone()
        .oneshot(request_without_body("GET", "/v1/open/templates", &headers))
        .await
        .expect("open templates http request");
    assert_eq!(templates_response.status(), StatusCode::OK);
    let templates_payload = read_response_json(templates_response).await;
    let template_exists = templates_payload["groups"]
        .as_array()
        .expect("groups array")
        .iter()
        .flat_map(|group| group["templates"].as_array().into_iter().flatten())
        .any(|template| template["id"] == template_id);
    assert!(template_exists);

    let printers_response = app
        .clone()
        .oneshot(request_without_body("GET", "/v1/open/printers", &headers))
        .await
        .expect("open printers http request");
    assert_eq!(printers_response.status(), StatusCode::OK);
    let printers_payload = read_response_json(printers_response).await;
    assert_eq!(printers_payload["printers"][0]["id"], "test-printer");
    assert_eq!(printers_payload["printers"][0]["name"], "Test Printer");

    let printer_detail_response = app
        .clone()
        .oneshot(request_without_body(
            "GET",
            "/v1/open/printers/test-printer",
            &headers,
        ))
        .await
        .expect("open printer detail http request");
    assert_eq!(printer_detail_response.status(), StatusCode::OK);
    let printer_detail = read_response_json(printer_detail_response).await;
    assert_eq!(printer_detail["id"], "test-printer");
    assert_eq!(
        printer_detail["capabilities"]["media_default"],
        "iso_a4_210x297mm"
    );
    assert!(
        printer_detail["capabilities"]["job_creation_attributes_supported"]
            .as_array()
            .expect("job creation attrs")
            .iter()
            .any(|value| value == "print-color-mode")
    );

    let preview_response = app
        .clone()
        .oneshot(request_with_json(
            "POST",
            "/v1/open/preview",
            &headers,
            serde_json::json!({
                "template_id": template_id,
                "data": { "orderNo": "HTTP-1001" },
                "print_options": {
                    "copies": 1,
                    "media": "iso_a4_210x297mm",
                    "sides": "one-sided",
                    "printColorMode": "monochrome"
                }
            }),
        ))
        .await
        .expect("open preview http request");
    assert_eq!(preview_response.status(), StatusCode::OK);
    assert_eq!(
        preview_response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/pdf")
    );
    let preview_bytes = read_response_bytes(preview_response).await;
    assert!(preview_bytes.starts_with(b"%PDF-"));

    let print_response = app
        .clone()
        .oneshot(request_with_json(
            "POST",
            "/v1/open/print",
            &headers,
            serde_json::json!({
                "request_id": "req-open-http-template",
                "template_id": template_id,
                "printer_id": "test-printer",
                "data": { "orderNo": "HTTP-1001" },
                "print_options": {
                    "copies": 1,
                    "media": "iso_a4_210x297mm",
                    "sides": "one-sided",
                    "printColorMode": "monochrome"
                }
            }),
        ))
        .await
        .expect("open print http request");
    assert_eq!(print_response.status(), StatusCode::ACCEPTED);
    let print_payload = read_response_json(print_response).await;
    assert_eq!(print_payload["status"], "queued");
    assert_eq!(print_payload["idempotent"], false);
    let job_id = print_payload["job_id"]
        .as_str()
        .expect("job_id string")
        .to_string();

    let job_by_id_response = app
        .clone()
        .oneshot(request_without_body(
            "GET",
            &format!("/v1/open/jobs/{job_id}"),
            &headers,
        ))
        .await
        .expect("open get job by id http request");
    assert_eq!(job_by_id_response.status(), StatusCode::OK);
    let job_by_id_payload = read_response_json(job_by_id_response).await;
    assert_eq!(job_by_id_payload["job_id"], job_id);
    assert_eq!(job_by_id_payload["request_id"], "req-open-http-template");
    assert_eq!(job_by_id_payload["job_kind"], JOB_KIND_TEMPLATE);
    assert_eq!(
        job_by_id_payload["print_options"]["printColorMode"],
        "monochrome"
    );

    let job_by_request_response = app
        .clone()
        .oneshot(request_without_body(
            "GET",
            "/v1/open/jobs/by-request-id/req-open-http-template",
            &headers,
        ))
        .await
        .expect("open get job by request id http request");
    assert_eq!(job_by_request_response.status(), StatusCode::OK);
    let job_by_request_payload = read_response_json(job_by_request_response).await;
    assert_eq!(job_by_request_payload["job_id"], job_id);
    assert_eq!(
        job_by_request_payload["request_id"],
        "req-open-http-template"
    );

    let boundary = "deepprint-http-flow-boundary";
    let direct_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/open/print/direct")
                .header(
                    AUTHORIZATION,
                    headers.get(AUTHORIZATION).expect("authorization header"),
                )
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(build_open_direct_multipart_body(
                    boundary,
                    "req-open-http-direct",
                )))
                .expect("build multipart request"),
        )
        .await
        .expect("open direct print http request");
    assert_eq!(direct_response.status(), StatusCode::ACCEPTED);
    let direct_payload = read_response_json(direct_response).await;
    assert_eq!(direct_payload["status"], "queued");
    assert_eq!(direct_payload["idempotent"], false);
    let direct_job_id = direct_payload["job_id"].as_str().expect("direct job id");

    let template_job = fetch_job_by_id(&conn, &job_id)
        .expect("query template job")
        .expect("template job exists");
    assert_eq!(template_job.job_kind, JOB_KIND_TEMPLATE);
    assert_eq!(template_job.request_id, "req-open-http-template");
    assert_eq!(template_job.printer_id.as_deref(), Some("test-printer"));

    let direct_job = fetch_job_by_id(&conn, direct_job_id)
        .expect("query direct job")
        .expect("direct job exists");
    assert_eq!(direct_job.job_kind, JOB_KIND_DIRECT_FILE);
    assert_eq!(direct_job.request_id, "req-open-http-direct");
    assert_eq!(direct_job.source_file_name.as_deref(), Some("external.pdf"));
    assert_eq!(
        direct_job.source_content_type.as_deref(),
        Some("application/pdf")
    );
}

#[tokio::test]
async fn open_endpoints_reject_missing_required_scopes() {
    let db_path = build_test_db_path("open-scope-matrix");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");
    let template_id = insert_open_api_test_template(&conn);
    let state = build_test_agent_state(db_path);
    let wrong_headers = build_api_key_headers_for_scopes(&conn, &[API_SCOPE_JOB_READ]);
    let non_job_headers = build_api_key_headers_for_scopes(&conn, &[API_SCOPE_PRINT_CREATE]);

    assert_required_scope(
        open_list_templates(State(state.clone()), wrong_headers.clone())
            .await
            .expect_err("templates should require template:read"),
        API_SCOPE_TEMPLATE_READ,
    )
    .await;

    assert_required_scope(
        open_list_printers(State(state.clone()), wrong_headers.clone())
            .await
            .expect_err("printers should require printer:read"),
        API_SCOPE_PRINTER_READ,
    )
    .await;

    assert_required_scope(
        open_get_printer_detail(
            State(state.clone()),
            wrong_headers.clone(),
            AxumPath("test-printer".to_string()),
        )
        .await
        .expect_err("printer detail should require printer:read"),
        API_SCOPE_PRINTER_READ,
    )
    .await;

    assert_required_scope(
        open_preview_typst(
            State(state.clone()),
            wrong_headers.clone(),
            Json(OpenPreviewRequest {
                template_id: template_id.clone(),
                data: serde_json::json!({ "orderNo": "A1001" }),
                print_options: PrintOptions::default(),
            }),
        )
        .await
        .expect_err("preview should require preview:create"),
        API_SCOPE_PREVIEW_CREATE,
    )
    .await;

    assert_required_scope(
        open_create_job(
            State(state.clone()),
            wrong_headers.clone(),
            Json(OpenPrintRequest {
                request_id: "req-open-scope-matrix".to_string(),
                template_id,
                printer_id: "test-printer".to_string(),
                data: serde_json::json!({ "orderNo": "A1001" }),
                print_options: PrintOptions::default(),
            }),
        )
        .await
        .expect_err("print should require print:create"),
        API_SCOPE_PRINT_CREATE,
    )
    .await;

    assert_required_scope(
        open_get_job(
            State(state.clone()),
            non_job_headers.clone(),
            AxumPath("job-missing-scope".to_string()),
        )
        .await
        .expect_err("job lookup should require job:read"),
        API_SCOPE_JOB_READ,
    )
    .await;

    assert_required_scope(
        open_get_job_by_request_id(
            State(state),
            non_job_headers,
            AxumPath("req-missing-scope".to_string()),
        )
        .await
        .expect_err("job lookup by request id should require job:read"),
        API_SCOPE_JOB_READ,
    )
    .await;
}


#[tokio::test]
async fn cancel_queued_job_changes_status_to_canceled() {
    let db_path = build_test_db_path("cancel-queued");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());

    let payload = build_create_job_request("req-cancel-queued");
    let (_, created) = create_job(State(state.clone()), Json(payload))
        .await
        .expect("create job");
    let job_id = created.0.job_id;

    let Json(cancel_resp) = cancel_job(State(state), AxumPath(job_id.clone()))
        .await
        .expect("cancel queued job");
    assert_eq!(cancel_resp.status, "canceled");

    let conn = open_conn(&db_path).expect("open conn");
    let job = fetch_job_by_id(&conn, &job_id)
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.status, "canceled");
}

#[tokio::test]
async fn cancel_printing_job_updates_error_fields() {
    let db_path = build_test_db_path("cancel-printing");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());

    let payload = build_create_job_request("req-cancel-printing");
    let (_, created) = create_job(State(state.clone()), Json(payload))
        .await
        .expect("create job");
    let job_id = created.0.job_id;

    let conn = open_conn(&db_path).expect("open conn");
    conn.execute(
        "UPDATE jobs
             SET status = 'printing',
                 backend_job_ref_json = ?1,
                 updated_at = ?2
             WHERE id = ?3",
        params!["mock-backend-job-for-cancel", now_unix(), job_id],
    )
    .expect("move job to printing");

    let Json(cancel_resp) = cancel_job(State(state), AxumPath(job_id.clone()))
        .await
        .expect("cancel printing job");
    assert_eq!(cancel_resp.status, "canceled");

    let job = fetch_job_by_id(&conn, &job_id)
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.status, "canceled");
    assert_eq!(job.backend_state.as_deref(), Some("canceled"));
    assert_eq!(
        job.backend_state_message.as_deref(),
        Some("backend cancel accepted by api")
    );
    assert_eq!(job.last_error_code.as_deref(), Some("CANCELED_BY_API"));
}
