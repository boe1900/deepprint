use super::*;

#[test]
fn apply_log_retention_respects_max_total_bytes() {
    let log_dir = build_test_temp_dir("log-retention-max-bytes");
    let file1 = log_dir.join("agent.log.a");
    let file2 = log_dir.join("agent.log.b");
    let file3 = log_dir.join("agent.log.c");

    fs::write(&file1, vec![1_u8; 20]).expect("write log file1");
    std::thread::sleep(Duration::from_millis(5));
    fs::write(&file2, vec![2_u8; 20]).expect("write log file2");
    std::thread::sleep(Duration::from_millis(5));
    fs::write(&file3, vec![3_u8; 20]).expect("write log file3");

    let snapshot = apply_log_retention(&log_dir, "agent.log", 0, 45).expect("apply log retention");
    assert_eq!(snapshot.removed_files, 1);
    assert_eq!(snapshot.files_count, 2);
    assert_eq!(snapshot.disk_usage_bytes, 40);
    assert!(!file1.exists());
    assert!(file2.exists());
    assert!(file3.exists());
}

#[tokio::test]
async fn create_job_is_idempotent_for_same_request_id() {
    let db_path = build_test_db_path("idempotent");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());

    let payload = build_create_job_request("req-idempotent");
    let (status1, body1) = create_job(State(state.clone()), Json(payload))
        .await
        .expect("create job first");
    assert_eq!(status1, StatusCode::ACCEPTED);
    assert!(!body1.0.idempotent);
    let first_job_id = body1.0.job_id.clone();

    let payload_same = build_create_job_request("req-idempotent");
    let (status2, body2) = create_job(State(state), Json(payload_same))
        .await
        .expect("create job second");
    assert_eq!(status2, StatusCode::OK);
    assert!(body2.0.idempotent);
    assert_eq!(body2.0.job_id, first_job_id);
}

#[tokio::test]
async fn create_direct_job_accepts_base64_payload() {
    let db_path = build_test_db_path("create-direct-job");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let payload = build_create_direct_job_request("req-direct-1", "invoice.pdf", b"%PDF-1.7\n");

    let (status, body) = create_direct_job(State(state), Json(payload))
        .await
        .expect("create direct job");
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(!body.0.idempotent);

    let conn = open_conn(&db_path).expect("open conn");
    let job = fetch_job_by_id(&conn, &body.0.job_id)
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.job_kind, JOB_KIND_DIRECT_FILE);
    assert_eq!(job.printer_id.as_deref(), Some("test-printer"));
    assert_eq!(job.printer_name_snapshot.as_deref(), Some("Test Printer"));
    assert_eq!(job.printer_uri.as_deref(), Some("mock:printer"));
    assert_eq!(job.source_file_name.as_deref(), Some("invoice.pdf"));
    assert_eq!(job.source_content_type.as_deref(), Some("application/pdf"));
    assert_eq!(job.source_file_size_bytes, Some(9));
    let source_path = PathBuf::from(
        job.source_file_path
            .as_deref()
            .expect("source path should exist"),
    );
    assert!(source_path.exists());
}

#[tokio::test]
async fn create_direct_job_rejects_oversized_payload() {
    let db_path = build_test_db_path("create-direct-job-too-large");
    init_schema(&db_path).expect("init schema");

    let config = AgentConfig {
        mock_mode: true,
        direct_job_max_bytes: 4,
        ..AgentConfig::default()
    };
    let state = build_test_agent_state_with_config(db_path, config);
    let payload = build_create_direct_job_request("req-direct-large", "big.pdf", b"12345");

    let err = create_direct_job(State(state), Json(payload))
        .await
        .expect_err("oversized payload should fail");
    match err {
        ApiError::BadRequest(message) => {
            assert!(message.contains("exceeds limit"));
        }
        other => panic!("expected bad request error, got {other}"),
    }
}

#[tokio::test]
async fn list_printers_returns_managed_registry_entries() {
    let db_path = build_test_db_path("list-printers-managed");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path);

    let Json(response) = list_printers(State(state))
        .await
        .expect("list printers should succeed");
    assert_eq!(response.printers.len(), 1);
    assert_eq!(response.printers[0].id, "test-printer");
    assert_eq!(response.printers[0].name, "Test Printer");
    assert_eq!(response.printers[0].uri, "mock:printer");
}

#[tokio::test]
async fn validate_printer_returns_already_managed_for_seeded_mock() {
    let db_path = build_test_db_path("validate-printer-managed");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path);

    let Json(response) = validate_printer(
        State(state),
        Json(ValidatePrinterRequest {
            target: PrinterTargetInput::Uri {
                uri: "mock:printer".to_string(),
            },
        }),
    )
    .await
    .expect("validate printer should succeed");

    assert!(response.already_managed);
    assert_eq!(response.managed_printer_id.as_deref(), Some("test-printer"));
    assert_eq!(response.discovered_name, "Mock Printer");
}

#[tokio::test]
async fn create_printer_returns_existing_record_when_already_managed() {
    let db_path = build_test_db_path("create-printer-idempotent");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path);

    let (status, body) = create_printer(
        State(state),
        Json(AddPrinterRequest {
            source: crate::print_backend::registry::PrinterSource::Manual,
            printer_uri: "mock:printer".to_string(),
            display_name: "Renamed Mock".to_string(),
        }),
    )
    .await
    .expect("create printer should succeed");
    assert_eq!(status, StatusCode::OK);
    assert!(!body.0.created);
    assert_eq!(body.0.printer.id, "test-printer");
    assert_eq!(body.0.printer.name, "Test Printer");
}

#[tokio::test]
async fn create_job_rejects_unsupported_duplex_for_printer() {
    let db_path = build_test_db_path("create-job-unsupported-duplex");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");
    conn.execute(
        "UPDATE printers
             SET capabilities_json = ?1
             WHERE id = 'test-printer'",
        params![serde_json::json!({
            "document_formats": ["application/pdf"],
            "sides_supported": ["one-sided"]
        })
        .to_string()],
    )
    .expect("update printer capabilities");

    let err = create_job(
        State(state),
        Json(CreateJobRequest {
            request_id: "req-unsupported-duplex".to_string(),
            printer_id: "test-printer".to_string(),
            template_content: "#set page(width: 80mm)\nHello".to_string(),
            data: serde_json::json!({}),
            print_options: PrintOptions {
                sides: Some(crate::printer::SidesMode::TwoSidedLongEdge),
                ..PrintOptions::default()
            },
        }),
    )
    .await
    .expect_err("unsupported duplex should fail");

    let (status, payload) = api_error_payload(err).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(payload["code"], "PRINT_OPTION_UNSUPPORTED");
    assert_eq!(
        payload["message"],
        "sides=two-sided-long-edge is not supported by this printer"
    );
    assert_eq!(payload["details"]["option"], "sides");
    assert_eq!(payload["details"]["requested_value"], "two-sided-long-edge");
    assert_eq!(payload["details"]["supported_values"][0], "one-sided");
}

#[tokio::test]
async fn create_direct_job_rejects_copies_above_printer_max() {
    let db_path = build_test_db_path("create-direct-job-copies-max");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");
    conn.execute(
        "UPDATE printers
             SET capabilities_json = ?1
             WHERE id = 'test-printer'",
        params![serde_json::json!({
            "document_formats": ["application/pdf"],
            "copies": { "default": 1, "min": 1, "max": 2 }
        })
        .to_string()],
    )
    .expect("update printer capabilities");

    let err = create_direct_job(
        State(state),
        Json(CreateDirectJobRequest {
            request_id: "req-direct-copies-max".to_string(),
            printer_id: "test-printer".to_string(),
            file_name: "test.pdf".to_string(),
            file_content_base64: BASE64_STANDARD.encode(b"%PDF-1.4\n%%EOF"),
            content_type: Some("application/pdf".to_string()),
            print_options: PrintOptions {
                copies: Some(3),
                ..PrintOptions::default()
            },
        }),
    )
    .await
    .expect_err("copies above max should fail");

    let (status, payload) = api_error_payload(err).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(payload["code"], "PRINT_OPTION_INVALID_FOR_PRINTER");
    assert_eq!(payload["message"], "copies=3 exceeds printer maximum 2");
    assert_eq!(payload["details"]["option"], "copies");
    assert_eq!(payload["details"]["requested_value"], "3");
    assert_eq!(payload["details"]["reason"], "above_maximum");
    assert_eq!(payload["details"]["limit"], 2);
}

#[tokio::test]
async fn create_job_rejects_unknown_orientation_capability_with_structured_error() {
    let db_path = build_test_db_path("create-job-orientation-unknown");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");
    conn.execute(
        "UPDATE printers
             SET capabilities_json = ?1
             WHERE id = 'test-printer'",
        params![serde_json::json!({
            "document_formats": ["application/pdf"]
        })
        .to_string()],
    )
    .expect("update printer capabilities");

    let err = create_job(
        State(state),
        Json(CreateJobRequest {
            request_id: "req-orientation-unknown".to_string(),
            printer_id: "test-printer".to_string(),
            template_content: "#set page(width: 80mm)\nHello".to_string(),
            data: serde_json::json!({}),
            print_options: PrintOptions {
                orientation_requested: Some(crate::printer::OrientationRequested::Landscape),
                ..PrintOptions::default()
            },
        }),
    )
    .await
    .expect_err("unknown orientation capability should fail");

    let (status, payload) = api_error_payload(err).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(payload["code"], "PRINTER_CAPABILITY_UNKNOWN");
    assert_eq!(
        payload["message"],
        "orientationRequested support is unknown for this printer"
    );
    assert_eq!(payload["details"]["option"], "orientationRequested");
    assert_eq!(payload["details"]["requested_value"], "landscape");
}

#[tokio::test]
async fn create_job_rejects_unsupported_color_mode_with_structured_error() {
    let db_path = build_test_db_path("create-job-color-unsupported");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");
    conn.execute(
        "UPDATE printers
             SET capabilities_json = ?1
             WHERE id = 'test-printer'",
        params![serde_json::json!({
            "document_formats": ["application/pdf"],
            "color_supported": false
        })
        .to_string()],
    )
    .expect("update printer capabilities");

    let err = create_job(
        State(state),
        Json(CreateJobRequest {
            request_id: "req-color-unsupported".to_string(),
            printer_id: "test-printer".to_string(),
            template_content: "#set page(width: 80mm)\nHello".to_string(),
            data: serde_json::json!({}),
            print_options: PrintOptions {
                print_color_mode: Some(crate::printer::PrintColorMode::Color),
                ..PrintOptions::default()
            },
        }),
    )
    .await
    .expect_err("unsupported color mode should fail");

    let (status, payload) = api_error_payload(err).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(payload["code"], "PRINT_OPTION_UNSUPPORTED");
    assert_eq!(
        payload["message"],
        "printColorMode=color is not supported by this printer"
    );
    assert_eq!(payload["details"]["option"], "printColorMode");
    assert_eq!(payload["details"]["requested_value"], "color");
    assert_eq!(payload["details"]["supported_values"][0], "monochrome");
}

#[tokio::test]
async fn create_job_rejects_page_ranges_when_printer_disallows_it() {
    let db_path = build_test_db_path("create-job-page-ranges-unsupported");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");
    conn.execute(
        "UPDATE printers
             SET capabilities_json = ?1
             WHERE id = 'test-printer'",
        params![serde_json::json!({
            "document_formats": ["application/pdf"],
            "supports_page_ranges": false
        })
        .to_string()],
    )
    .expect("update printer capabilities");

    let err = create_job(
        State(state),
        Json(CreateJobRequest {
            request_id: "req-page-ranges-unsupported".to_string(),
            printer_id: "test-printer".to_string(),
            template_content: "#set page(width: 80mm)\nHello".to_string(),
            data: serde_json::json!({}),
            print_options: PrintOptions {
                page_ranges: Some("1-3".to_string()),
                ..PrintOptions::default()
            },
        }),
    )
    .await
    .expect_err("unsupported page ranges should fail");

    let (status, payload) = api_error_payload(err).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(payload["code"], "PRINT_OPTION_UNSUPPORTED");
    assert_eq!(
        payload["message"],
        "pageRanges=1-3 is not supported by this printer"
    );
    assert_eq!(payload["details"]["option"], "pageRanges");
    assert_eq!(payload["details"]["requested_value"], "1-3");
}

#[tokio::test]
async fn create_job_rejects_missing_printer_with_structured_error() {
    let db_path = build_test_db_path("create-job-printer-missing");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path);

    let err = create_job(
        State(state),
        Json(CreateJobRequest {
            request_id: "req-printer-missing".to_string(),
            printer_id: "missing-printer".to_string(),
            template_content: "#set page(width: 80mm)\nHello".to_string(),
            data: serde_json::json!({}),
            print_options: PrintOptions::default(),
        }),
    )
    .await
    .expect_err("missing printer should fail");

    let (status, payload) = api_error_payload(err).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(payload["code"], "PRINTER_NOT_FOUND");
    assert_eq!(payload["message"], "printer not found: missing-printer");
}

#[tokio::test]
async fn create_job_rejects_disabled_printer_with_structured_error() {
    let db_path = build_test_db_path("create-job-printer-disabled");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");
    conn.execute(
        "UPDATE printers
             SET is_enabled = 0
             WHERE id = 'test-printer'",
        [],
    )
    .expect("disable printer");

    let err = create_job(
        State(state),
        Json(CreateJobRequest {
            request_id: "req-printer-disabled".to_string(),
            printer_id: "test-printer".to_string(),
            template_content: "#set page(width: 80mm)\nHello".to_string(),
            data: serde_json::json!({}),
            print_options: PrintOptions::default(),
        }),
    )
    .await
    .expect_err("disabled printer should fail");

    let (status, payload) = api_error_payload(err).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(payload["code"], "PRINTER_DISABLED");
    assert_eq!(payload["message"], "printer is disabled: test-printer");
}

#[tokio::test]
async fn get_printer_detail_returns_managed_snapshot() {
    let db_path = build_test_db_path("get-printer-detail");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path);

    let Json(response) = get_printer_detail(State(state), AxumPath("test-printer".to_string()))
        .await
        .expect("get printer detail should succeed");
    assert_eq!(response.id, "test-printer");
    assert_eq!(response.name, "Test Printer");
    assert_eq!(response.uri, "mock:printer");
    assert_eq!(response.normalized_uri, "mock:printer");
}

#[tokio::test]
async fn refresh_printer_updates_snapshot_fields() {
    let db_path = build_test_db_path("refresh-printer");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());

    let Json(response) = refresh_printer(State(state), AxumPath("test-printer".to_string()))
        .await
        .expect("refresh printer should succeed");
    assert_eq!(response.id, "test-printer");
    assert_eq!(response.name, "Test Printer");
    assert_eq!(response.state.as_deref(), Some("idle"));
    assert!(response.last_refreshed_at.is_some());

    let conn = open_conn(&db_path).expect("open conn");
    let record = printer_registry::get_printer_by_id(&conn, "test-printer")
        .expect("load printer by id")
        .expect("printer exists");
    assert_eq!(record.last_known_state.as_deref(), Some("idle"));
    assert!(record.last_refreshed_at.is_some());
    assert!(record
        .attributes_json
        .as_deref()
        .is_some_and(|value| value.contains("Mock Printer")));
}

#[tokio::test]
async fn refresh_printer_returns_not_found_for_missing_printer() {
    let db_path = build_test_db_path("refresh-printer-missing");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path);

    let err = refresh_printer(State(state), AxumPath("missing-printer".to_string()))
        .await
        .expect_err("refresh missing printer should fail");
    match err {
        ApiError::NotFound(message) => {
            assert!(message.contains("printer not found"));
        }
        other => panic!("expected not found error, got {other}"),
    }
}

#[tokio::test]
async fn disable_printer_marks_printer_disabled() {
    let db_path = build_test_db_path("disable-printer");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");

    seed_additional_test_printer(&conn, "printer-disable", "Printer Disable", false, true);

    let Json(response) = disable_printer(State(state), AxumPath("printer-disable".to_string()))
        .await
        .expect("disable printer should succeed");
    assert!(!response.enabled);

    let record = printer_registry::get_printer_by_id(&conn, "printer-disable")
        .expect("get printer by id")
        .expect("printer exists");
    assert!(!record.is_enabled);
}

#[tokio::test]
async fn enable_printer_marks_printer_enabled() {
    let db_path = build_test_db_path("enable-printer");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");

    seed_additional_test_printer(&conn, "printer-enable", "Printer Enable", false, false);

    let Json(response) = enable_printer(State(state), AxumPath("printer-enable".to_string()))
        .await
        .expect("enable printer should succeed");
    assert!(response.enabled);

    let record = printer_registry::get_printer_by_id(&conn, "printer-enable")
        .expect("get printer by id")
        .expect("printer exists");
    assert!(record.is_enabled);
}

#[tokio::test]
async fn set_default_printer_switches_default_flag() {
    let db_path = build_test_db_path("set-default-printer");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");

    seed_additional_test_printer(&conn, "printer-default", "Printer Default", false, true);

    let Json(response) = set_default_printer(State(state), AxumPath("printer-default".to_string()))
        .await
        .expect("set default printer should succeed");
    assert!(response.is_default);

    let old_default = printer_registry::get_printer_by_id(&conn, "test-printer")
        .expect("get old default")
        .expect("old default exists");
    let new_default = printer_registry::get_printer_by_id(&conn, "printer-default")
        .expect("get new default")
        .expect("new default exists");
    assert!(!old_default.is_default);
    assert!(new_default.is_default);
}

#[tokio::test]
async fn disable_default_printer_returns_conflict() {
    let db_path = build_test_db_path("disable-default-printer");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path);

    let err = disable_printer(State(state), AxumPath("test-printer".to_string()))
        .await
        .expect_err("disable default printer should fail");
    let (status, payload) = api_error_payload(err).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(payload["code"], "PRINTER_CONFLICT");
    assert_eq!(payload["message"], "default printer cannot be disabled");
}

#[tokio::test]
async fn delete_printer_rejects_inflight_jobs() {
    let db_path = build_test_db_path("delete-printer-inflight");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");

    seed_additional_test_printer(
        &conn,
        "printer-delete-blocked",
        "Delete Blocked",
        false,
        true,
    );
    insert_test_job(
        &conn,
        "job-delete-blocked",
        "req-delete-blocked",
        "queued",
        None,
        None,
    );
    conn.execute(
        "UPDATE jobs SET printer_id = ?1 WHERE id = ?2",
        params!["printer-delete-blocked", "job-delete-blocked"],
    )
    .expect("attach inflight job");

    let err = delete_printer(State(state), AxumPath("printer-delete-blocked".to_string()))
        .await
        .expect_err("delete printer with inflight job should fail");
    match err {
        ApiError::Conflict(message) => assert!(message.contains("inflight")),
        other => panic!("expected conflict error, got {other}"),
    }
}

#[tokio::test]
async fn delete_printer_allows_terminal_jobs_and_rehomes_default() {
    let db_path = build_test_db_path("delete-printer-terminal");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");

    seed_additional_test_printer(&conn, "printer-delete-ok", "Delete OK", false, true);
    seed_additional_test_printer(
        &conn,
        "printer-delete-default",
        "Delete Default",
        true,
        true,
    );
    conn.execute(
        "UPDATE printers SET is_default = 0 WHERE id = 'test-printer'",
        [],
    )
    .expect("clear old default");
    insert_test_job(
        &conn,
        "job-delete-terminal",
        "req-delete-terminal",
        "succeeded",
        None,
        None,
    );
    conn.execute(
        "UPDATE jobs SET printer_id = ?1 WHERE id = ?2",
        params!["printer-delete-default", "job-delete-terminal"],
    )
    .expect("attach terminal job");

    let Json(response) =
        delete_printer(State(state), AxumPath("printer-delete-default".to_string()))
            .await
            .expect("delete printer should succeed");
    assert!(response.deleted);

    let deleted = printer_registry::get_printer_by_id(&conn, "printer-delete-default")
        .expect("query deleted printer");
    assert!(deleted.is_none());
    let fallback_default = printer_registry::get_printer_by_id(&conn, "test-printer")
        .expect("query fallback default")
        .expect("fallback default exists");
    assert!(fallback_default.is_default);
}
