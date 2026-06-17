use super::*;

#[test]
fn claim_next_job_moves_to_rendering_and_increments_attempt() {
    let db_path = build_test_db_path("claim-next");
    init_schema(&db_path).expect("init schema");

    let conn = open_conn(&db_path).expect("open conn");
    conn.execute(
        "INSERT INTO jobs (
               id, request_id, template_content, data_json, print_options_json,
               status, attempt_count, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, 'queued', 0, ?6, ?6)",
        params![
            "job-claim-next",
            "req-claim-next",
            "#set page(width: 80mm)",
            "{\"orderNo\":\"A1\"}",
            "{}",
            now_unix()
        ],
    )
    .expect("insert queued job");

    let claimed = claim_next_job(&db_path)
        .expect("claim next job")
        .expect("job should be claimed");
    assert_eq!(claimed, "job-claim-next");

    let job = fetch_job_by_id(&conn, "job-claim-next")
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.status, "rendering");
    assert_eq!(job.attempt_count, 1);
}

#[test]
fn claim_next_job_skips_same_printer_when_inflight_exists() {
    let db_path = build_test_db_path("claim-next-same-printer-serialized");
    init_schema(&db_path).expect("init schema");

    let conn = open_conn(&db_path).expect("open conn");
    seed_test_printer(&db_path);
    conn.execute(
            "INSERT INTO jobs (
               id, request_id, job_kind, printer_id, printer_name_snapshot, printer_uri,
               template_content, data_json, print_options_json, status, attempt_count, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'printing', 1, ?10, ?10)",
            params![
                "job-printing-existing",
                "req-printing-existing",
                "template",
                "test-printer",
                "Test Printer",
                "ipp://printer.local/ipp/print",
                "#set page(width: 80mm)",
                "{\"orderNo\":\"A1\"}",
                "{}",
                now_unix(),
            ],
        )
        .expect("insert printing job");
    conn.execute(
            "INSERT INTO jobs (
               id, request_id, job_kind, printer_id, printer_name_snapshot, printer_uri,
               template_content, data_json, print_options_json, status, attempt_count, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'queued', 0, ?10, ?10)",
            params![
                "job-queued-same-printer",
                "req-queued-same-printer",
                "template",
                "test-printer",
                "Test Printer",
                "ipp://printer.local/ipp/print",
                "#set page(width: 80mm)",
                "{\"orderNo\":\"A2\"}",
                "{}",
                now_unix(),
            ],
        )
        .expect("insert queued same-printer job");

    let claimed = claim_next_job(&db_path).expect("claim next job");
    assert!(claimed.is_none());
}

#[test]
fn recover_inflight_jobs_only_requeues_rendering_in_normal_mode() {
    let db_path = build_test_db_path("recover-inflight");
    init_schema(&db_path).expect("init schema");

    let conn = open_conn(&db_path).expect("open conn");
    insert_test_job(
        &conn,
        "job-recover-rendering",
        "req-recover-rendering",
        "rendering",
        Some("OLD_RENDER_ERR"),
        Some("rendering old error"),
    );
    insert_test_job(
        &conn,
        "job-recover-printing",
        "req-recover-printing",
        "printing",
        Some("OLD_PRINT_ERR"),
        Some("printing old error"),
    );
    insert_test_job(
        &conn,
        "job-recover-queued",
        "req-recover-queued",
        "queued",
        Some("KEEP_ERR"),
        Some("keep this error"),
    );

    recover_inflight_jobs(&db_path, false).expect("recover inflight jobs");

    let rendering = fetch_job_by_id(&conn, "job-recover-rendering")
        .expect("query rendering job")
        .expect("rendering job exists");
    assert_eq!(rendering.status, "queued");
    assert_eq!(rendering.last_error_code, None);
    assert_eq!(rendering.last_error_message, None);

    let printing = fetch_job_by_id(&conn, "job-recover-printing")
        .expect("query printing job")
        .expect("printing job exists");
    assert_eq!(printing.status, "printing");
    assert_eq!(printing.last_error_code.as_deref(), Some("OLD_PRINT_ERR"));
    assert_eq!(
        printing.last_error_message.as_deref(),
        Some("printing old error")
    );

    let queued = fetch_job_by_id(&conn, "job-recover-queued")
        .expect("query queued job")
        .expect("queued job exists");
    assert_eq!(queued.status, "queued");
    assert_eq!(queued.last_error_code.as_deref(), Some("KEEP_ERR"));
}

#[test]
fn recover_inflight_jobs_requeues_printing_in_mock_mode() {
    let db_path = build_test_db_path("recover-inflight-mock-printing");
    init_schema(&db_path).expect("init schema");

    let conn = open_conn(&db_path).expect("open conn");
    insert_test_job(
        &conn,
        "job-recover-printing",
        "req-recover-printing",
        "printing",
        Some("OLD_PRINT_ERR"),
        Some("printing old error"),
    );

    recover_inflight_jobs(&db_path, true).expect("recover inflight jobs");

    let printing = fetch_job_by_id(&conn, "job-recover-printing")
        .expect("query printing job")
        .expect("printing job exists");
    assert_eq!(printing.status, "queued");
    assert_eq!(printing.last_error_code, None);
    assert_eq!(printing.last_error_message, None);
    assert_eq!(printing.backend_job_ref_json, None);
    assert_eq!(printing.backend_state, None);
    assert_eq!(printing.needs_attention_reason, None);
    assert_eq!(printing.unknown_since_at, None);
}

#[tokio::test]
async fn cancel_printing_job_without_backend_job_ref_json_returns_conflict() {
    let db_path = build_test_db_path("cancel-printing-missing-backend-job-ref");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());

    let payload = build_create_job_request("req-cancel-printing-missing-backend-job-ref");
    let (_, created) = create_job(State(state.clone()), Json(payload))
        .await
        .expect("create job");
    let job_id = created.0.job_id;

    let conn = open_conn(&db_path).expect("open conn");
    conn.execute(
        "UPDATE jobs
             SET status = 'printing',
                 backend_job_ref_json = NULL,
                 updated_at = ?1
             WHERE id = ?2",
        params![now_unix(), job_id],
    )
    .expect("move job to printing without backend job ref");

    let err = match cancel_job(State(state), AxumPath(job_id)).await {
        Ok(_) => panic!("cancel printing without backend job ref should fail"),
        Err(err) => err,
    };
    match err {
        ApiError::Conflict(message) => {
            assert!(message.contains("backend_job_ref_json is missing"));
        }
        other => panic!("expected conflict error, got {other}"),
    }
}

#[tokio::test]
async fn cancel_needs_attention_job_changes_status_to_canceled() {
    let db_path = build_test_db_path("cancel-needs-attention");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());

    let payload = build_create_job_request("req-cancel-needs-attention");
    let (_, created) = create_job(State(state.clone()), Json(payload))
        .await
        .expect("create job");
    let job_id = created.0.job_id;

    let conn = open_conn(&db_path).expect("open conn");
    conn.execute(
        "UPDATE jobs
             SET status = 'needs_attention',
                 last_error_code = 'SUBMISSION_RESULT_UNKNOWN',
                 last_error_message = 'manual attention required',
                 needs_attention_reason = 'SUBMISSION_RESULT_UNKNOWN',
                 updated_at = ?1
             WHERE id = ?2",
        params![now_unix(), job_id],
    )
    .expect("move job to needs_attention");

    let Json(cancel_resp) = cancel_job(State(state), AxumPath(job_id.clone()))
        .await
        .expect("cancel needs_attention job");
    assert_eq!(cancel_resp.status, "canceled");

    let job = fetch_job_by_id(&conn, &job_id)
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.status, "canceled");
    assert_eq!(job.last_error_code.as_deref(), Some("CANCELED_BY_API"));
}

#[tokio::test]
async fn monitor_submitting_jobs_reconciles_unique_backend_job_ref() {
    let db_path = build_test_db_path("submitting-reconcile-success");
    init_schema(&db_path).expect("init schema");
    let backend = Arc::new(TestReconcileBackend {
        reconcile_result: std::sync::Mutex::new(Some(Ok(Some(
            "{\"printer_uri\":\"ipp://printer.local/ipp/print\",\"job_id\":42}".to_string(),
        )))),
        query_result: std::sync::Mutex::new(None),
    });
    let state = build_test_agent_state_with_backend(db_path.clone(), backend);

    let conn = open_conn(&db_path).expect("open conn");
    insert_test_job(
        &conn,
        "job-submitting-reconcile",
        "req-submitting-reconcile",
        "submitting",
        None,
        None,
    );
    conn.execute(
        "UPDATE jobs
             SET printer_uri = 'ipp://printer.local/ipp/print',
                 submit_started_at = ?1,
                 updated_at = ?1
             WHERE id = ?2",
        params![now_unix(), "job-submitting-reconcile"],
    )
    .expect("seed submitting job");

    monitor_submitting_jobs(state.as_ref())
        .await
        .expect("monitor submitting jobs");

    let job = fetch_job_by_id(&conn, "job-submitting-reconcile")
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.status, "printing");
    assert_eq!(
        job.backend_job_ref_json.as_deref(),
        Some("{\"printer_uri\":\"ipp://printer.local/ipp/print\",\"job_id\":42}")
    );
    assert_eq!(job.backend_state.as_deref(), Some("accepted"));
    assert!(job.submitted_at.is_some());
    assert_eq!(job.needs_attention_reason, None);
}

#[tokio::test]
async fn monitor_submitting_jobs_times_out_to_needs_attention() {
    let db_path = build_test_db_path("submitting-reconcile-timeout");
    init_schema(&db_path).expect("init schema");
    let config = AgentConfig {
        mock_mode: true,
        submission_recovery_timeout_sec: 10,
        ..AgentConfig::default()
    };
    let state = build_test_agent_state_with_config(db_path.clone(), config);

    let conn = open_conn(&db_path).expect("open conn");
    insert_test_job(
        &conn,
        "job-submitting-timeout",
        "req-submitting-timeout",
        "submitting",
        None,
        None,
    );
    conn.execute(
        "UPDATE jobs
             SET printer_uri = 'mock:printer',
                 submit_started_at = ?1,
                 updated_at = ?2
             WHERE id = ?3",
        params![now_unix() - 60, now_unix() - 60, "job-submitting-timeout"],
    )
    .expect("seed timed out submitting job");

    monitor_submitting_jobs(state.as_ref())
        .await
        .expect("monitor submitting jobs");

    let job = fetch_job_by_id(&conn, "job-submitting-timeout")
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.status, "needs_attention");
    assert_eq!(
        job.last_error_code.as_deref(),
        Some("SUBMISSION_RECOVERY_TIMEOUT")
    );
    assert_eq!(
        job.needs_attention_reason.as_deref(),
        Some("submission_recovery_timeout")
    );
}

#[tokio::test]
async fn monitor_printing_jobs_moves_completed_failed_and_canceled_to_terminal() {
    let db_path = build_test_db_path("printing-terminal-transitions");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");

    insert_test_job(
        &conn,
        "job-printing-completed",
        "req-printing-completed",
        "printing",
        None,
        None,
    );
    conn.execute(
        "UPDATE jobs SET backend_job_ref_json = ?1 WHERE id = ?2",
        params!["ref-completed", "job-printing-completed"],
    )
    .expect("seed completed ref");
    let completed_state = build_test_agent_state_with_backend(
        db_path.clone(),
        Arc::new(TestReconcileBackend {
            reconcile_result: std::sync::Mutex::new(None),
            query_result: std::sync::Mutex::new(Some(Ok(BackendJobState::Completed))),
        }),
    );
    monitor_printing_jobs(completed_state.as_ref())
        .await
        .expect("monitor completed printing job");
    let completed = fetch_job_by_id(&conn, "job-printing-completed")
        .expect("query completed job")
        .expect("completed job exists");
    assert_eq!(completed.status, "succeeded");

    insert_test_job(
        &conn,
        "job-printing-failed",
        "req-printing-failed",
        "printing",
        None,
        None,
    );
    conn.execute(
        "UPDATE jobs SET backend_job_ref_json = ?1 WHERE id = ?2",
        params!["ref-failed", "job-printing-failed"],
    )
    .expect("seed failed ref");
    let failed_state = build_test_agent_state_with_backend(
        db_path.clone(),
        Arc::new(TestReconcileBackend {
            reconcile_result: std::sync::Mutex::new(None),
            query_result: std::sync::Mutex::new(Some(Ok(BackendJobState::Failed))),
        }),
    );
    monitor_printing_jobs(failed_state.as_ref())
        .await
        .expect("monitor failed printing job");
    let failed = fetch_job_by_id(&conn, "job-printing-failed")
        .expect("query failed job")
        .expect("failed job exists");
    assert_eq!(failed.status, "failed");
    assert_eq!(
        failed.last_error_code.as_deref(),
        Some("BACKEND_JOB_FAILED")
    );

    insert_test_job(
        &conn,
        "job-printing-canceled",
        "req-printing-canceled",
        "printing",
        None,
        None,
    );
    conn.execute(
        "UPDATE jobs SET backend_job_ref_json = ?1 WHERE id = ?2",
        params!["ref-canceled", "job-printing-canceled"],
    )
    .expect("seed canceled ref");
    let canceled_state = build_test_agent_state_with_backend(
        db_path.clone(),
        Arc::new(TestReconcileBackend {
            reconcile_result: std::sync::Mutex::new(None),
            query_result: std::sync::Mutex::new(Some(Ok(BackendJobState::Canceled))),
        }),
    );
    monitor_printing_jobs(canceled_state.as_ref())
        .await
        .expect("monitor canceled printing job");
    let canceled = fetch_job_by_id(&conn, "job-printing-canceled")
        .expect("query canceled job")
        .expect("canceled job exists");
    assert_eq!(canceled.status, "canceled");
    assert_eq!(
        canceled.last_error_code.as_deref(),
        Some("BACKEND_JOB_CANCELED")
    );
}

#[tokio::test]
async fn monitor_printing_jobs_retryable_backend_error_does_not_requeue() {
    let db_path = build_test_db_path("printing-no-requeue-on-timeout");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");

    insert_test_job(
        &conn,
        "job-printing-timeout",
        "req-printing-timeout",
        "printing",
        None,
        None,
    );
    conn.execute(
        "UPDATE jobs
             SET backend_job_ref_json = ?1,
                 unknown_since_at = ?2,
                 updated_at = ?2
             WHERE id = ?3",
        params!["ref-timeout", now_unix(), "job-printing-timeout"],
    )
    .expect("seed printing timeout ref");

    let state = build_test_agent_state_with_backend_and_config(
        db_path.clone(),
        Arc::new(TestReconcileBackend {
            reconcile_result: std::sync::Mutex::new(None),
            query_result: std::sync::Mutex::new(Some(Err(crate::printer::BackendError::new(
                "BACKEND_STATUS_TIMEOUT",
                "temporary backend timeout",
                true,
            )))),
        }),
        AgentConfig {
            mock_mode: true,
            backend_unknown_to_attention_sec: 300,
            ..AgentConfig::default()
        },
    );

    monitor_printing_jobs(state.as_ref())
        .await
        .expect("monitor printing jobs");

    let job = fetch_job_by_id(&conn, "job-printing-timeout")
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.status, "printing");
    assert_ne!(job.status, "queued");
    assert_eq!(job.backend_state.as_deref(), Some("unknown"));
}

#[test]
fn transition_job_status_returns_false_on_stale_from_status() {
    let db_path = build_test_db_path("stale-transition");
    init_schema(&db_path).expect("init schema");

    let conn = open_conn(&db_path).expect("open conn");
    insert_test_job(
        &conn,
        "job-stale-transition",
        "req-stale-transition",
        "queued",
        None,
        None,
    );

    let transitioned = transition_job_status(
        &db_path,
        "job-stale-transition",
        "printing",
        "succeeded",
        "should not transition",
        None,
        None,
    )
    .expect("transition call");
    assert!(!transitioned);

    let job = fetch_job_by_id(&conn, "job-stale-transition")
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.status, "queued");
}

#[test]
fn handle_job_failure_retryable_schedules_retry() {
    let db_path = build_test_db_path("retry-scheduled");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");

    insert_test_job(
        &conn,
        "job-retry-scheduled",
        "req-retry-scheduled",
        "rendering",
        None,
        None,
    );
    conn.execute(
        "UPDATE jobs SET attempt_count = 1, status = 'rendering' WHERE id = ?1",
        params!["job-retry-scheduled"],
    )
    .expect("set attempt_count");

    let err = ProcessJobError::retryable("BACKEND_STATUS_TIMEOUT", "temporary backend timeout");
    handle_job_failure(&db_path, "job-retry-scheduled", &err, 3, 2, 60)
        .expect("handle retryable failure");

    let (status, next_retry_at): (String, Option<i64>) = conn
        .query_row(
            "SELECT status, next_retry_at FROM jobs WHERE id = ?1",
            params!["job-retry-scheduled"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("query retry state");
    assert_eq!(status, "queued");
    assert!(next_retry_at.unwrap_or(0) > now_unix());

    let retry_metric =
        read_agent_metric(&conn, METRIC_RETRY_SCHEDULED_TOTAL).expect("read retry metric");
    assert_eq!(retry_metric, 1);

    let dead_letter_count: i64 = conn
        .query_row("SELECT COUNT(1) FROM dead_letter", [], |row| row.get(0))
        .expect("query dead letter count");
    assert_eq!(dead_letter_count, 0);
}

#[test]
fn handle_job_failure_retryable_at_limit_moves_dead_letter() {
    let db_path = build_test_db_path("retry-dead-letter");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");

    insert_test_job(
        &conn,
        "job-retry-dead-letter",
        "req-retry-dead-letter",
        "printing",
        None,
        None,
    );
    conn.execute(
        "UPDATE jobs SET attempt_count = 3, status = 'printing' WHERE id = ?1",
        params!["job-retry-dead-letter"],
    )
    .expect("set attempt_count");

    let err = ProcessJobError::retryable("BACKEND_STATUS_TIMEOUT", "timed out repeatedly");
    handle_job_failure(&db_path, "job-retry-dead-letter", &err, 3, 2, 60)
        .expect("handle dead-letter failure");

    let job = fetch_job_by_id(&conn, "job-retry-dead-letter")
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.status, "needs_attention");
    assert_eq!(
        job.last_error_code.as_deref(),
        Some("BACKEND_STATUS_TIMEOUT")
    );
    assert_eq!(
        job.needs_attention_reason.as_deref(),
        Some("BACKEND_STATUS_TIMEOUT")
    );

    let dead_letter_count: i64 = conn
        .query_row("SELECT COUNT(1) FROM dead_letter", [], |row| row.get(0))
        .expect("query dead letter count");
    assert_eq!(dead_letter_count, 0);

    let dead_letter_metric =
        read_agent_metric(&conn, METRIC_DEAD_LETTER_TOTAL).expect("read dead letter metric");
    assert_eq!(dead_letter_metric, 0);
}

#[test]
fn handle_job_failure_for_submitting_moves_to_needs_attention() {
    let db_path = build_test_db_path("submitting-needs-attention");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");

    insert_test_job(
        &conn,
        "job-submitting-attention",
        "req-submitting-attention",
        "submitting",
        None,
        None,
    );

    let err = ProcessJobError::retryable("IPP_SUBMIT_TIMEOUT", "submit result is uncertain");
    handle_job_failure(&db_path, "job-submitting-attention", &err, 3, 2, 60)
        .expect("handle submitting failure");

    let job = fetch_job_by_id(&conn, "job-submitting-attention")
        .expect("query job")
        .expect("job exists");
    assert_eq!(job.status, "needs_attention");
    assert_eq!(job.last_error_code.as_deref(), Some("IPP_SUBMIT_TIMEOUT"));
    assert_eq!(
        job.needs_attention_reason.as_deref(),
        Some("IPP_SUBMIT_TIMEOUT")
    );
}

#[test]
fn handle_job_failure_for_submitting_without_backend_ref_schedules_retry() {
    let db_path = build_test_db_path("submitting-retry-before-backend-ref");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");

    insert_test_job(
        &conn,
        "job-submitting-retry",
        "req-submitting-retry",
        "submitting",
        None,
        None,
    );
    conn.execute(
            "UPDATE jobs SET attempt_count = 1, status = 'submitting', backend_job_ref_json = NULL WHERE id = ?1",
            params!["job-submitting-retry"],
        )
        .expect("set submitting retry state");

    let err = ProcessJobError::retryable(
        "IPP_SUBMIT_STATUS_FAILED",
        "ipp submit failed with status ServerErrorBusy",
    );
    handle_job_failure(&db_path, "job-submitting-retry", &err, 3, 2, 60)
        .expect("handle submitting retryable failure");

    let (status, next_retry_at, needs_attention_reason): (String, Option<i64>, Option<String>) =
        conn.query_row(
            "SELECT status, next_retry_at, needs_attention_reason FROM jobs WHERE id = ?1",
            params!["job-submitting-retry"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("query retry state");
    assert_eq!(status, "queued");
    assert!(next_retry_at.unwrap_or(0) > now_unix());
    assert_eq!(needs_attention_reason, None);
}
