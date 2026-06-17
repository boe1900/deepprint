use std::path::Path;

use rusqlite::{params, Connection};
use uuid::Uuid;

use super::{
    increment_agent_metric_conn, open_connection, read_agent_metric,
    METRIC_TEMPLATE_WORKSPACE_SEEDED_V1,
};

pub fn init_schema(db_path: &Path) -> rusqlite::Result<()> {
    let conn = open_connection(db_path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 3000;

         CREATE TABLE IF NOT EXISTS jobs (
           id TEXT PRIMARY KEY,
           request_id TEXT NOT NULL UNIQUE,
           job_kind TEXT NOT NULL DEFAULT 'template',
           printer_id TEXT,
           printer_name_snapshot TEXT,
           printer_uri TEXT,
           template_content TEXT NOT NULL,
           data_json TEXT NOT NULL,
           print_options_json TEXT NOT NULL,
           source_file_path TEXT,
           source_file_name TEXT,
           source_content_type TEXT,
           source_file_size_bytes INTEGER,
           status TEXT NOT NULL,
           attempt_count INTEGER NOT NULL DEFAULT 0,
           next_retry_at INTEGER,
           last_error_code TEXT,
           last_error_message TEXT,
           render_artifact_path TEXT,
           render_output_kind TEXT,
           render_page_count INTEGER,
           render_page_width_pt REAL,
           render_page_height_pt REAL,
           backend_name TEXT,
           backend_job_ref_json TEXT,
           submit_started_at INTEGER,
           submitted_at INTEGER,
           last_polled_at INTEGER,
           backend_state TEXT,
           backend_state_message TEXT,
           unknown_since_at INTEGER,
           needs_attention_reason TEXT,
           created_at INTEGER NOT NULL,
           updated_at INTEGER NOT NULL
         );

         CREATE INDEX IF NOT EXISTS idx_jobs_status_created_at
           ON jobs(status, created_at);

         CREATE TABLE IF NOT EXISTS printers (
           id TEXT PRIMARY KEY,
           source TEXT NOT NULL,
           display_name TEXT NOT NULL,
           printer_uri TEXT NOT NULL UNIQUE,
           normalized_uri TEXT NOT NULL UNIQUE,
           is_default INTEGER NOT NULL DEFAULT 0,
           is_enabled INTEGER NOT NULL DEFAULT 1,
           last_known_state TEXT,
           last_state_message TEXT,
           capabilities_json TEXT,
           attributes_json TEXT,
           last_seen_at INTEGER,
           last_validated_at INTEGER,
           last_refreshed_at INTEGER,
           created_at INTEGER NOT NULL,
           updated_at INTEGER NOT NULL
         );

         CREATE INDEX IF NOT EXISTS idx_printers_display_name
           ON printers(display_name, created_at);

         CREATE INDEX IF NOT EXISTS idx_printers_default_enabled
           ON printers(is_default, is_enabled, updated_at);

         CREATE TABLE IF NOT EXISTS render_cache (
           cache_key TEXT PRIMARY KEY,
           template_hash TEXT NOT NULL,
           data_hash TEXT NOT NULL,
           print_options_hash TEXT NOT NULL,
           artifact_path TEXT NOT NULL,
           artifact_size_bytes INTEGER NOT NULL DEFAULT 0,
           output_kind TEXT NOT NULL,
           page_count INTEGER NOT NULL,
           page_width_pt REAL,
           page_height_pt REAL,
           hit_count INTEGER NOT NULL DEFAULT 0,
           created_at INTEGER NOT NULL,
           updated_at INTEGER NOT NULL
         );

         CREATE INDEX IF NOT EXISTS idx_render_cache_template_data
           ON render_cache(template_hash, data_hash);

         CREATE TABLE IF NOT EXISTS agent_metrics (
           key TEXT PRIMARY KEY,
           value INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS job_events (
           id TEXT PRIMARY KEY,
           job_id TEXT NOT NULL,
           event_type TEXT NOT NULL,
           from_status TEXT,
           to_status TEXT,
           message TEXT NOT NULL,
           created_at INTEGER NOT NULL
         );

         CREATE INDEX IF NOT EXISTS idx_job_events_job_created_at
           ON job_events(job_id, created_at);

         CREATE TABLE IF NOT EXISTS dead_letter (
           id TEXT PRIMARY KEY,
           job_id TEXT NOT NULL UNIQUE,
           request_id TEXT NOT NULL,
           final_error_code TEXT NOT NULL,
           final_error_message TEXT NOT NULL,
           attempts INTEGER NOT NULL,
           failed_at INTEGER NOT NULL
         );

         CREATE INDEX IF NOT EXISTS idx_dead_letter_failed_at
           ON dead_letter(failed_at);

         CREATE TABLE IF NOT EXISTS users (
           id TEXT PRIMARY KEY,
           username TEXT NOT NULL UNIQUE,
           email TEXT,
           display_name TEXT NOT NULL,
           role TEXT NOT NULL,
           status TEXT NOT NULL,
           must_change_password INTEGER NOT NULL DEFAULT 0,
           password_changed_at INTEGER,
           created_at INTEGER NOT NULL,
           updated_at INTEGER NOT NULL
         );

         CREATE INDEX IF NOT EXISTS idx_users_status
           ON users(status, updated_at);

         CREATE TABLE IF NOT EXISTS auth_identities (
           id TEXT PRIMARY KEY,
           user_id TEXT NOT NULL,
           provider_type TEXT NOT NULL,
           provider_key TEXT NOT NULL,
           password_hash TEXT,
           provider_subject TEXT,
           provider_meta TEXT,
           created_at INTEGER NOT NULL,
           updated_at INTEGER NOT NULL,
           FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
           UNIQUE(provider_type, provider_key)
         );

         CREATE INDEX IF NOT EXISTS idx_auth_identities_user
           ON auth_identities(user_id, provider_type);

         CREATE TABLE IF NOT EXISTS sessions (
           id TEXT PRIMARY KEY,
           user_id TEXT NOT NULL,
           session_token_hash TEXT NOT NULL UNIQUE,
           created_at INTEGER NOT NULL,
           expires_at INTEGER NOT NULL,
           last_seen_at INTEGER,
           ip TEXT,
           user_agent TEXT,
           revoked_at INTEGER,
           FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
         );

         CREATE INDEX IF NOT EXISTS idx_sessions_user_expires
           ON sessions(user_id, expires_at);

         CREATE INDEX IF NOT EXISTS idx_sessions_token_active
           ON sessions(session_token_hash, expires_at, revoked_at);

         CREATE TABLE IF NOT EXISTS api_keys (
           id TEXT PRIMARY KEY,
           name TEXT NOT NULL,
           key_prefix TEXT NOT NULL UNIQUE,
           secret_hash TEXT NOT NULL UNIQUE,
           scopes_json TEXT NOT NULL,
           status TEXT NOT NULL,
           created_by_user_id TEXT,
           created_at INTEGER NOT NULL,
           updated_at INTEGER NOT NULL,
           last_used_at INTEGER,
           revoked_at INTEGER,
           expires_at INTEGER,
           FOREIGN KEY(created_by_user_id) REFERENCES users(id) ON DELETE SET NULL
         );

         CREATE INDEX IF NOT EXISTS idx_api_keys_status_created_at
           ON api_keys(status, created_at);

         CREATE INDEX IF NOT EXISTS idx_api_keys_prefix_hash
           ON api_keys(key_prefix, secret_hash);

         CREATE TABLE IF NOT EXISTS template_groups (
           id TEXT PRIMARY KEY,
           name TEXT NOT NULL UNIQUE,
           sort_order INTEGER NOT NULL DEFAULT 0,
           created_at INTEGER NOT NULL,
           updated_at INTEGER NOT NULL
         );

         CREATE INDEX IF NOT EXISTS idx_template_groups_sort_order
           ON template_groups(sort_order, created_at);

         CREATE TABLE IF NOT EXISTS templates (
           id TEXT PRIMARY KEY,
           group_id TEXT NOT NULL,
           name TEXT NOT NULL,
           description TEXT NOT NULL DEFAULT '',
           output_name TEXT NOT NULL,
           typst_code TEXT NOT NULL,
           sample_data TEXT NOT NULL,
           sort_order INTEGER NOT NULL DEFAULT 0,
           created_at INTEGER NOT NULL,
           updated_at INTEGER NOT NULL,
           FOREIGN KEY(group_id) REFERENCES template_groups(id) ON DELETE RESTRICT,
           UNIQUE(group_id, name)
         );

         CREATE INDEX IF NOT EXISTS idx_templates_group_sort_order
           ON templates(group_id, sort_order, created_at);

         CREATE TABLE IF NOT EXISTS app_settings (
           key TEXT PRIMARY KEY,
           value TEXT NOT NULL,
           updated_at INTEGER NOT NULL
         );
        ",
    )?;

    ensure_jobs_column(&conn, "render_artifact_path", "TEXT")?;
    ensure_jobs_column(&conn, "render_output_kind", "TEXT")?;
    ensure_jobs_column(&conn, "render_page_count", "INTEGER")?;
    ensure_jobs_column(&conn, "render_page_width_pt", "REAL")?;
    ensure_jobs_column(&conn, "render_page_height_pt", "REAL")?;
    ensure_jobs_column(&conn, "job_kind", "TEXT NOT NULL DEFAULT 'template'")?;
    ensure_jobs_column(&conn, "printer_id", "TEXT")?;
    ensure_jobs_column(&conn, "printer_name_snapshot", "TEXT")?;
    ensure_jobs_column(&conn, "printer_uri", "TEXT")?;
    ensure_jobs_column(&conn, "source_file_path", "TEXT")?;
    ensure_jobs_column(&conn, "source_file_name", "TEXT")?;
    ensure_jobs_column(&conn, "source_content_type", "TEXT")?;
    ensure_jobs_column(&conn, "source_file_size_bytes", "INTEGER")?;
    ensure_jobs_column(&conn, "backend_name", "TEXT")?;
    migrate_backend_name_column(&conn)?;
    ensure_jobs_column(&conn, "backend_job_id", "TEXT")?;
    ensure_jobs_column(&conn, "backend_job_ref_json", "TEXT")?;
    ensure_jobs_column(&conn, "submit_started_at", "INTEGER")?;
    ensure_jobs_column(&conn, "submitted_at", "INTEGER")?;
    ensure_jobs_column(&conn, "last_polled_at", "INTEGER")?;
    ensure_jobs_column(&conn, "backend_state", "TEXT")?;
    ensure_jobs_column(&conn, "backend_state_message", "TEXT")?;
    ensure_jobs_column(&conn, "unknown_since_at", "INTEGER")?;
    ensure_jobs_column(&conn, "needs_attention_reason", "TEXT")?;
    ensure_jobs_column(&conn, "next_retry_at", "INTEGER")?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_jobs_status_next_retry_created_at
           ON jobs(status, next_retry_at, created_at);",
    )?;
    migrate_backend_job_ref_json(&conn)?;
    ensure_render_cache_column(&conn, "artifact_size_bytes", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_templates_column(&conn, "description", "TEXT NOT NULL DEFAULT ''")?;
    ensure_templates_column(
        &conn,
        "output_name",
        "TEXT NOT NULL DEFAULT 'template-output.pdf'",
    )?;
    ensure_templates_column(&conn, "typst_code", "TEXT NOT NULL DEFAULT ''")?;
    ensure_templates_column(&conn, "sample_data", "TEXT NOT NULL DEFAULT '{}'")?;
    ensure_templates_column(&conn, "sort_order", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_templates_column(&conn, "created_at", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_templates_column(&conn, "updated_at", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_users_column(&conn, "must_change_password", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_users_column(&conn, "password_changed_at", "INTEGER")?;
    seed_template_workspace_if_needed(&conn)?;

    Ok(())
}

pub fn has_table_column(
    conn: &Connection,
    table_name: &str,
    column_name: &str,
) -> rusqlite::Result<bool> {
    let sql = format!("PRAGMA table_info({table_name})");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column_name {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn seed_initial_admin_user_if_no_users(
    db_path: &Path,
    username: &str,
    provider_key: &str,
    email: Option<String>,
    display_name: &str,
    password_hash: &str,
    now: i64,
) -> rusqlite::Result<bool> {
    let conn = open_connection(db_path)?;
    let tx = conn.unchecked_transaction()?;
    let user_count: i64 = tx.query_row("SELECT COUNT(1) FROM users", [], |row| row.get(0))?;
    if user_count > 0 {
        tx.rollback()?;
        return Ok(false);
    }

    let user_id = format!("user-{}", Uuid::new_v4());
    let identity_id = format!("identity-{}", Uuid::new_v4());
    tx.execute(
        "INSERT INTO users
           (id, username, email, display_name, role, status, must_change_password,
            password_changed_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, NULL, ?7, ?8)",
        params![
            user_id,
            username,
            email,
            display_name,
            "admin",
            "active",
            now,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO auth_identities
           (id, user_id, provider_type, provider_key, password_hash, provider_subject,
            provider_meta, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?6, ?7)",
        params![
            identity_id,
            user_id,
            "local",
            provider_key,
            password_hash,
            now,
            now
        ],
    )?;
    tx.commit()?;
    Ok(true)
}

fn migrate_backend_job_ref_json(conn: &Connection) -> rusqlite::Result<()> {
    if !has_table_column(conn, "jobs", "backend_job_id")?
        || !has_table_column(conn, "jobs", "backend_job_ref_json")?
    {
        return Ok(());
    }

    conn.execute(
        "UPDATE jobs
         SET backend_job_ref_json = backend_job_id
         WHERE (backend_job_ref_json IS NULL OR TRIM(backend_job_ref_json) = '')
           AND backend_job_id IS NOT NULL
           AND TRIM(backend_job_id) <> ''",
        [],
    )?;

    Ok(())
}

fn migrate_backend_name_column(conn: &Connection) -> rusqlite::Result<()> {
    if !has_table_column(conn, "jobs", "backend_name")? {
        return Ok(());
    }

    if has_table_column(conn, "jobs", "printer_backend")? {
        conn.execute(
            "UPDATE jobs
             SET backend_name = printer_backend
             WHERE (backend_name IS NULL OR TRIM(backend_name) = '')
               AND printer_backend IS NOT NULL
               AND TRIM(printer_backend) <> ''",
            [],
        )?;
    }

    if has_table_column(conn, "jobs", "backend")? {
        conn.execute(
            "UPDATE jobs
             SET backend_name = backend
             WHERE (backend_name IS NULL OR TRIM(backend_name) = '')
               AND backend IS NOT NULL
               AND TRIM(backend) <> ''",
            [],
        )?;
    }

    Ok(())
}

fn ensure_jobs_column(
    conn: &Connection,
    column_name: &str,
    column_decl: &str,
) -> rusqlite::Result<()> {
    if has_table_column(conn, "jobs", column_name)? {
        return Ok(());
    }

    let sql = format!("ALTER TABLE jobs ADD COLUMN {column_name} {column_decl}");
    conn.execute(&sql, [])?;
    Ok(())
}

fn ensure_render_cache_column(
    conn: &Connection,
    column_name: &str,
    column_decl: &str,
) -> rusqlite::Result<()> {
    if has_table_column(conn, "render_cache", column_name)? {
        return Ok(());
    }

    let sql = format!("ALTER TABLE render_cache ADD COLUMN {column_name} {column_decl}");
    conn.execute(&sql, [])?;
    Ok(())
}

fn ensure_templates_column(
    conn: &Connection,
    column_name: &str,
    column_decl: &str,
) -> rusqlite::Result<()> {
    if has_table_column(conn, "templates", column_name)? {
        return Ok(());
    }

    let sql = format!("ALTER TABLE templates ADD COLUMN {column_name} {column_decl}");
    conn.execute(&sql, [])?;
    Ok(())
}

fn ensure_users_column(
    conn: &Connection,
    column_name: &str,
    column_decl: &str,
) -> rusqlite::Result<()> {
    if has_table_column(conn, "users", column_name)? {
        return Ok(());
    }

    let sql = format!("ALTER TABLE users ADD COLUMN {column_name} {column_decl}");
    conn.execute(&sql, [])?;
    Ok(())
}

fn seed_template_workspace_if_needed(conn: &Connection) -> rusqlite::Result<()> {
    let seeded = read_agent_metric(conn, METRIC_TEMPLATE_WORKSPACE_SEEDED_V1)?;
    if seeded > 0 {
        return Ok(());
    }

    let group_count: i64 =
        conn.query_row("SELECT COUNT(1) FROM template_groups", [], |row| row.get(0))?;
    let template_count: i64 =
        conn.query_row("SELECT COUNT(1) FROM templates", [], |row| row.get(0))?;

    if group_count > 0 || template_count > 0 {
        increment_agent_metric_conn(conn, METRIC_TEMPLATE_WORKSPACE_SEEDED_V1, 1)?;
        return Ok(());
    }

    let now = now_unix();
    let tx = conn.unchecked_transaction()?;
    let groups = [
        (
            "group-billing",
            "账单与发票",
            0_i64,
            vec![(
                "template-invoice",
                "发票模板",
                "适合标准销售发票与对账单。",
                "invoice-generated.pdf",
                "#set page(margin: 18mm)\n#let amount(v) = [¥ #v]\n\n= 销售发票\n\n客户：#data.customer\n发票号：#data.invoice_no\n金额：#amount(data.amount)\n税率：#data.tax_rate\n到期日：#data.due_date",
                "{\n  \"customer\": \"张三\",\n  \"invoice_no\": \"INV-2026-0518\",\n  \"amount\": 1280,\n  \"tax_rate\": 0.13,\n  \"due_date\": \"2026-05-22\"\n}",
                0_i64,
            )],
        ),
        (
            "group-retail",
            "零售单据",
            1_i64,
            vec![(
                "template-receipt",
                "收据模板",
                "适合门店小票与付款凭证。",
                "receipt-generated.pdf",
                "#set page(margin: 14mm)\n\n= 付款收据\n\n收款方：#data.merchant\n付款人：#data.payer\n金额：¥ #data.amount\n支付方式：#data.payment_method",
                "{\n  \"payer\": \"李四\",\n  \"merchant\": \"DeepPrint Studio\",\n  \"amount\": 399,\n  \"payment_method\": \"银行卡\"\n}",
                0_i64,
            )],
        ),
        (
            "group-warehouse",
            "仓储与标签",
            2_i64,
            vec![(
                "template-label",
                "标签模板",
                "适合仓储标签、货架签和快递贴纸。",
                "label-generated.pdf",
                "#set page(width: 90mm, height: 50mm, margin: 8mm)\n\n= 仓储标签\n\nSKU：#data.sku\n批次：#data.batch\n库位：#data.location\n数量：#data.qty",
                "{\n  \"sku\": \"DP-INK-04\",\n  \"batch\": \"B-240518\",\n  \"location\": \"A-03-08\",\n  \"qty\": 24\n}",
                0_i64,
            )],
        ),
        (
            "group-certificate",
            "证书与证明",
            3_i64,
            vec![(
                "template-certificate",
                "证书模板",
                "适合奖状、培训结业证和内部授权书。",
                "certificate-generated.pdf",
                "#set page(margin: 24mm)\n\n= 结业证书\n\n授予：#data.recipient\n课程：#data.course\n日期：#data.date\n讲师：#data.issuer",
                "{\n  \"recipient\": \"王小明\",\n  \"course\": \"设备校准培训\",\n  \"date\": \"2026-05-18\",\n  \"issuer\": \"DeepPrint Lab\"\n}",
                0_i64,
            )],
        ),
    ];

    for (group_id, group_name, group_sort_order, templates) in groups {
        tx.execute(
            "INSERT INTO template_groups (id, name, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![group_id, group_name, group_sort_order, now],
        )?;

        for (
            template_id,
            template_name,
            description,
            output_name,
            typst_code,
            sample_data,
            sort_order,
        ) in templates
        {
            tx.execute(
                "INSERT INTO templates (
                   id, group_id, name, description, output_name, typst_code, sample_data,
                   sort_order, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                params![
                    template_id,
                    group_id,
                    template_name,
                    description,
                    output_name,
                    typst_code,
                    sample_data,
                    sort_order,
                    now,
                ],
            )?;
        }
    }

    increment_agent_metric_conn(&tx, METRIC_TEMPLATE_WORKSPACE_SEEDED_V1, 1)?;
    tx.commit()?;
    Ok(())
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
