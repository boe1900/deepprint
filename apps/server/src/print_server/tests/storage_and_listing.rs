use super::*;

#[test]
fn cleanup_disk_watermark_evicts_lru_until_low_watermark() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    init_test_cleanup_schema(&conn);

    let temp_root = build_test_temp_dir("watermark");
    let file_old = temp_root.join("old.pdf");
    let file_new = temp_root.join("new.pdf");
    fs::write(&file_old, vec![1_u8; 20]).expect("write old file");
    fs::write(&file_new, vec![2_u8; 20]).expect("write new file");

    conn.execute(
            "INSERT INTO render_cache (cache_key, artifact_path, artifact_size_bytes, created_at, updated_at)
             VALUES (?1, ?2, ?3, 1, 1)",
            params!["old-key", file_old.to_string_lossy().to_string(), 20_i64],
        )
        .expect("insert old cache");
    conn.execute(
            "INSERT INTO render_cache (cache_key, artifact_path, artifact_size_bytes, created_at, updated_at)
             VALUES (?1, ?2, ?3, 2, 2)",
            params!["new-key", file_new.to_string_lossy().to_string(), 20_i64],
        )
        .expect("insert new cache");

    let snapshot = crate::storage::cleanup_render_cache_by_disk_watermark_conn(&conn, 30, 20)
        .expect("cleanup by watermark");
    assert_eq!(snapshot.stale_removed, 0);
    assert_eq!(snapshot.watermark_evicted, 1);
    assert_eq!(snapshot.disk_usage_bytes, 20);

    let count: i64 = conn
        .query_row("SELECT COUNT(1) FROM render_cache", [], |row| row.get(0))
        .expect("count cache rows");
    assert_eq!(count, 1);

    let remaining_key: String = conn
        .query_row("SELECT cache_key FROM render_cache", [], |row| row.get(0))
        .expect("remaining key");
    assert_eq!(remaining_key, "new-key");

    assert!(!file_old.exists());
    assert!(file_new.exists());
}

#[test]
fn cleanup_disk_watermark_removes_stale_entries_even_without_overflow() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    init_test_cleanup_schema(&conn);

    let missing_path = build_test_temp_dir("stale")
        .join("missing.pdf")
        .to_string_lossy()
        .to_string();
    conn.execute(
            "INSERT INTO render_cache (cache_key, artifact_path, artifact_size_bytes, created_at, updated_at)
             VALUES (?1, ?2, ?3, 1, 1)",
            params!["stale-key", missing_path, 123_i64],
        )
        .expect("insert stale cache");

    let snapshot = crate::storage::cleanup_render_cache_by_disk_watermark_conn(&conn, 1_000, 800)
        .expect("cleanup stale cache");
    assert_eq!(snapshot.stale_removed, 1);
    assert_eq!(snapshot.watermark_evicted, 0);
    assert_eq!(snapshot.disk_usage_bytes, 0);

    let count: i64 = conn
        .query_row("SELECT COUNT(1) FROM render_cache", [], |row| row.get(0))
        .expect("count cache rows");
    assert_eq!(count, 0);
}

#[test]
fn init_schema_migrates_legacy_jobs_table_before_retry_index() {
    let db_path = build_test_db_path("legacy-schema-next-retry");
    let conn = Connection::open(&db_path).expect("open sqlite db");
    conn.execute_batch(
        "CREATE TABLE jobs (
               id TEXT PRIMARY KEY,
               request_id TEXT NOT NULL UNIQUE,
               template_content TEXT NOT NULL,
               data_json TEXT NOT NULL,
               print_options_json TEXT NOT NULL,
               status TEXT NOT NULL,
               attempt_count INTEGER NOT NULL DEFAULT 0,
               last_error_code TEXT,
               last_error_message TEXT,
               created_at INTEGER NOT NULL,
               updated_at INTEGER NOT NULL
             );",
    )
    .expect("create legacy jobs table");
    drop(conn);

    init_schema(&db_path).expect("init schema for legacy db");
    let conn = open_conn(&db_path).expect("open migrated db");

    assert!(
        has_table_column(&conn, "jobs", "next_retry_at").expect("query jobs columns"),
        "next_retry_at should exist after migration"
    );

    let index_count: i64 = conn
        .query_row(
            "SELECT COUNT(1)
                 FROM sqlite_master
                 WHERE type = 'index'
                   AND name = 'idx_jobs_status_next_retry_created_at'",
            [],
            |row| row.get(0),
        )
        .expect("check retry index");
    assert_eq!(index_count, 1);
}

#[test]
fn load_queue_metrics_snapshot_reports_counts_and_rates() {
    let db_path = build_test_db_path("queue-metrics");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");

    insert_test_job(&conn, "job-q-1", "req-q-1", "queued", None, None);
    insert_test_job(&conn, "job-r-1", "req-r-1", "rendering", None, None);
    insert_test_job(&conn, "job-p-1", "req-p-1", "printing", None, None);
    insert_test_job(&conn, "job-s-1", "req-s-1", "succeeded", None, None);
    insert_test_job(&conn, "job-s-2", "req-s-2", "succeeded", None, None);
    insert_test_job(
        &conn,
        "job-f-1",
        "req-f-1",
        "failed",
        Some("ERR"),
        Some("failed"),
    );
    insert_test_job(
        &conn,
        "job-c-1",
        "req-c-1",
        "canceled",
        Some("CANCELED"),
        Some("canceled"),
    );

    conn.execute(
        "UPDATE jobs SET created_at = 10, updated_at = 20 WHERE id = 'job-s-1'",
        [],
    )
    .expect("update succeeded job 1 duration");
    conn.execute(
        "UPDATE jobs SET created_at = 30, updated_at = 60 WHERE id = 'job-s-2'",
        [],
    )
    .expect("update succeeded job 2 duration");

    let snapshot = load_queue_metrics_snapshot(&db_path).expect("load queue metrics");
    assert_eq!(snapshot.queued_count, 1);
    assert_eq!(snapshot.rendering_count, 1);
    assert_eq!(snapshot.submitting_count, 0);
    assert_eq!(snapshot.printing_count, 1);
    assert_eq!(snapshot.needs_attention_count, 0);
    assert_eq!(snapshot.succeeded_count, 2);
    assert_eq!(snapshot.failed_count, 1);
    assert_eq!(snapshot.canceled_count, 1);
    assert_eq!(snapshot.terminal_total, 4);
    assert!((snapshot.success_rate - 0.5).abs() < f64::EPSILON);
    assert!((snapshot.failure_rate - 0.25).abs() < f64::EPSILON);
    assert!((snapshot.avg_succeeded_duration_sec - 20.0).abs() < f64::EPSILON);
}

#[test]
fn load_failed_jobs_snapshot_limits_rows() {
    let db_path = build_test_db_path("failed-jobs-snapshot");
    init_schema(&db_path).expect("init schema");
    let conn = open_conn(&db_path).expect("open conn");

    insert_test_job(
        &conn,
        "job-failed-1",
        "req-failed-1",
        "failed",
        Some("F1"),
        Some("failed one"),
    );
    insert_test_job(
        &conn,
        "job-failed-2",
        "req-failed-2",
        "failed",
        Some("F2"),
        Some("failed two"),
    );
    insert_test_job(
        &conn,
        "job-canceled-1",
        "req-canceled-1",
        "canceled",
        Some("C1"),
        Some("canceled one"),
    );
    insert_test_job(
        &conn,
        "job-succeeded-1",
        "req-succeeded-1",
        "succeeded",
        None,
        None,
    );

    conn.execute(
        "UPDATE jobs SET updated_at = 100 WHERE id = 'job-failed-1'",
        [],
    )
    .expect("update timestamp 1");
    conn.execute(
        "UPDATE jobs SET updated_at = 200 WHERE id = 'job-failed-2'",
        [],
    )
    .expect("update timestamp 2");
    conn.execute(
        "UPDATE jobs SET updated_at = 300 WHERE id = 'job-canceled-1'",
        [],
    )
    .expect("update timestamp 3");

    let snapshot = load_failed_jobs_snapshot(&conn, 2).expect("load failed jobs snapshot");
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0].job_id, "job-canceled-1");
    assert_eq!(snapshot[1].job_id, "job-failed-2");
}

#[tokio::test]
async fn list_jobs_defaults_to_needs_attention_ordered_by_updated_at_desc() {
    let db_path = build_test_db_path("list-jobs-default-needs-attention");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");

    insert_test_job_with_printer(
        &conn,
        "job-attn-old",
        "req-attn-old",
        "needs_attention",
        Some("test-printer"),
    );
    insert_test_job_with_printer(
        &conn,
        "job-printing",
        "req-printing",
        "printing",
        Some("test-printer"),
    );
    insert_test_job_with_printer(
        &conn,
        "job-attn-new",
        "req-attn-new",
        "needs_attention",
        Some("test-printer"),
    );
    conn.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![100, "job-attn-old"],
    )
    .expect("update old attention");
    conn.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![200, "job-printing"],
    )
    .expect("update printing");
    conn.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![300, "job-attn-new"],
    )
    .expect("update new attention");

    let Json(response) = list_jobs(State(state), Query(ListJobsQuery::default()))
        .await
        .expect("list jobs");

    assert!(response.defaulted_to_needs_attention);
    assert_eq!(response.status_filter, vec!["needs_attention".to_string()]);
    assert_eq!(response.total, 2);
    assert_eq!(response.jobs.len(), 2);
    assert_eq!(response.jobs[0].job_id, "job-attn-new");
    assert_eq!(response.jobs[1].job_id, "job-attn-old");
}

#[tokio::test]
async fn list_jobs_supports_status_printer_and_pagination_filters() {
    let db_path = build_test_db_path("list-jobs-filters");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");
    seed_additional_test_printer(&conn, "printer-alt", "Alt Printer", false, true);

    insert_test_job_with_printer(
        &conn,
        "job-printing-a",
        "req-printing-a",
        "printing",
        Some("test-printer"),
    );
    insert_test_job_with_printer(
        &conn,
        "job-printing-b",
        "req-printing-b",
        "printing",
        Some("printer-alt"),
    );
    insert_test_job_with_printer(
        &conn,
        "job-queued-a",
        "req-queued-a",
        "queued",
        Some("test-printer"),
    );
    insert_test_job_with_printer(
        &conn,
        "job-succeeded-a",
        "req-succeeded-a",
        "succeeded",
        Some("test-printer"),
    );
    conn.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![400, "job-printing-a"],
    )
    .expect("update printing a");
    conn.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![300, "job-printing-b"],
    )
    .expect("update printing b");
    conn.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![200, "job-queued-a"],
    )
    .expect("update queued a");
    conn.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![100, "job-succeeded-a"],
    )
    .expect("update succeeded a");

    let Json(response) = list_jobs(
        State(state),
        Query(ListJobsQuery {
            page: Some(1),
            page_size: Some(1),
            status: Some("printing,queued".to_string()),
            printer_id: Some("test-printer".to_string()),
            q: None,
        }),
    )
    .await
    .expect("list filtered jobs");

    assert!(!response.defaulted_to_needs_attention);
    assert_eq!(
        response.status_filter,
        vec!["printing".to_string(), "queued".to_string()]
    );
    assert_eq!(response.printer_id.as_deref(), Some("test-printer"));
    assert_eq!(response.total, 2);
    assert_eq!(response.total_pages, 2);
    assert_eq!(response.jobs.len(), 1);
    assert_eq!(response.jobs[0].job_id, "job-printing-a");
}

#[tokio::test]
async fn list_jobs_rejects_invalid_status_filter() {
    let db_path = build_test_db_path("list-jobs-invalid-status");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path);

    let err = list_jobs(
        State(state),
        Query(ListJobsQuery {
            page: None,
            page_size: None,
            status: Some("not-a-status".to_string()),
            printer_id: None,
            q: None,
        }),
    )
    .await
    .expect_err("invalid status should fail");

    match err {
        ApiError::Structured { code, .. } => assert_eq!(code, "INVALID_JOB_STATUS_FILTER"),
        other => panic!("expected structured error, got {other}"),
    }
}

#[tokio::test]
async fn list_jobs_supports_search_query_filter() {
    let db_path = build_test_db_path("list-jobs-search-query");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");
    seed_additional_test_printer(&conn, "printer-alt", "Alt Printer", false, true);

    insert_test_job_with_printer(
        &conn,
        "job-design",
        "req-design",
        "succeeded",
        Some("test-printer"),
    );
    insert_test_job_with_printer(
        &conn,
        "job-finance",
        "req-finance",
        "succeeded",
        Some("printer-alt"),
    );
    conn.execute(
        "UPDATE jobs
            SET source_file_name = ?1,
                printer_name_snapshot = ?2,
                updated_at = ?3
          WHERE id = ?4",
        params!["design-proof.pdf", "Design Room Printer", 200, "job-design"],
    )
    .expect("update design job");
    conn.execute(
        "UPDATE jobs
            SET source_file_name = ?1,
                printer_name_snapshot = ?2,
                updated_at = ?3
          WHERE id = ?4",
        params!["finance-report.pdf", "Finance Printer", 300, "job-finance"],
    )
    .expect("update finance job");

    let Json(response) = list_jobs(
        State(state),
        Query(ListJobsQuery {
            page: None,
            page_size: None,
            status: Some("succeeded".to_string()),
            printer_id: None,
            q: Some("design".to_string()),
        }),
    )
    .await
    .expect("list searched jobs");

    assert_eq!(response.q.as_deref(), Some("design"));
    assert_eq!(response.total, 1);
    assert_eq!(response.jobs.len(), 1);
    assert_eq!(response.jobs[0].job_id, "job-design");
}

#[tokio::test]
async fn list_recent_jobs_returns_latest_jobs_without_default_attention_filter() {
    let db_path = build_test_db_path("list-recent-jobs");
    init_schema(&db_path).expect("init schema");
    let state = build_test_agent_state(db_path.clone());
    let conn = open_conn(&db_path).expect("open conn");

    insert_test_job_with_printer(
        &conn,
        "job-old",
        "req-old",
        "needs_attention",
        Some("test-printer"),
    );
    insert_test_job_with_printer(
        &conn,
        "job-mid",
        "req-mid",
        "printing",
        Some("test-printer"),
    );
    insert_test_job_with_printer(
        &conn,
        "job-new",
        "req-new",
        "succeeded",
        Some("test-printer"),
    );
    conn.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![100, "job-old"],
    )
    .expect("update old");
    conn.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![200, "job-mid"],
    )
    .expect("update mid");
    conn.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![300, "job-new"],
    )
    .expect("update new");

    let Json(response) = list_recent_jobs(
        State(state),
        Query(ListRecentJobsQuery {
            limit: Some(2),
            printer_id: None,
        }),
    )
    .await
    .expect("list recent jobs");

    assert_eq!(response.limit, 2);
    assert_eq!(response.jobs.len(), 2);
    assert_eq!(response.jobs[0].job_id, "job-new");
    assert_eq!(response.jobs[1].job_id, "job-mid");
}

#[test]
fn cleanup_old_diagnostic_bundles_respects_limit() {
    let diag_dir = build_test_temp_dir("diagnostics-cleanup");
    let bundle1 = diag_dir.join("diag-1.zip");
    let bundle2 = diag_dir.join("diag-2.zip");
    let bundle3 = diag_dir.join("diag-3.zip");
    let other = diag_dir.join("diag-4.txt");

    fs::write(&bundle1, vec![1_u8; 10]).expect("write bundle1");
    std::thread::sleep(Duration::from_millis(5));
    fs::write(&bundle2, vec![2_u8; 10]).expect("write bundle2");
    std::thread::sleep(Duration::from_millis(5));
    fs::write(&bundle3, vec![3_u8; 10]).expect("write bundle3");
    fs::write(&other, vec![9_u8; 10]).expect("write other file");

    cleanup_old_diagnostic_bundles(&diag_dir, 2).expect("cleanup old diagnostic bundles");

    assert!(!bundle1.exists());
    assert!(bundle2.exists());
    assert!(bundle3.exists());
    assert!(other.exists());
}

#[test]
fn apply_log_retention_respects_max_files() {
    let log_dir = build_test_temp_dir("log-retention-max-files");
    let file1 = log_dir.join("agent.log.1");
    let file2 = log_dir.join("agent.log.2");
    let file3 = log_dir.join("agent.log.3");
    let ignore = log_dir.join("other.log.1");

    fs::write(&file1, vec![1_u8; 10]).expect("write log file1");
    std::thread::sleep(Duration::from_millis(5));
    fs::write(&file2, vec![2_u8; 10]).expect("write log file2");
    std::thread::sleep(Duration::from_millis(5));
    fs::write(&file3, vec![3_u8; 10]).expect("write log file3");
    fs::write(&ignore, vec![9_u8; 10]).expect("write ignored file");

    let snapshot = apply_log_retention(&log_dir, "agent.log", 2, 0).expect("apply log retention");
    assert_eq!(snapshot.removed_files, 1);
    assert_eq!(snapshot.files_count, 2);
    assert_eq!(snapshot.disk_usage_bytes, 20);
    assert!(!file1.exists());
    assert!(file2.exists());
    assert!(file3.exists());
    assert!(ignore.exists());
}
