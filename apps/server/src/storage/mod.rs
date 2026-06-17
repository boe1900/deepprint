use std::path::{Path, PathBuf};

mod sqlite;

pub use sqlite::{
    cancel_needs_attention_job, cancel_printing_job, cancel_queued_job,
    cleanup_render_cache_by_disk_watermark, count_active_admins_at_path,
    count_inflight_jobs_for_printer_at_path, count_jobs_at_path, count_templates_in_group_at_path,
    create_printer_record, delete_auth_user_record_at_path, delete_printer_record,
    delete_template_group_by_id_at_path, delete_template_record_at_path, disable_printer_record,
    enable_printer_record, evict_render_cache_if_needed, fetch_job_by_id_at_path,
    fetch_job_by_request_id_at_path, fetch_template_by_id_at_path, handle_job_failure_at_path,
    increment_agent_metric, increment_agent_metric_conn, init_schema,
    insert_api_key_record_at_path, insert_auth_session_at_path, insert_direct_job_at_path,
    insert_local_auth_user_record_at_path, insert_template_group_at_path,
    insert_template_job_at_path, insert_template_record_at_path, list_api_key_records_at_path,
    list_auth_users_at_path, list_jobs_page_at_path, list_printer_summaries,
    list_printing_jobs_for_monitor, list_recent_jobs_records_at_path,
    list_submitting_jobs_for_monitor, list_template_groups_at_path, list_templates_at_path,
    list_templates_by_group_at_path, load_api_key_by_prefix_and_hash_at_path,
    load_auth_session_at_path, load_auth_user_at_path, load_auth_user_by_id_at_path,
    load_cache_metrics_snapshot, load_cups_base_url, load_failed_jobs_snapshot_at_path,
    load_local_auth_identity_at_path, load_local_auth_identity_by_user_id_at_path,
    load_printer_by_normalized_uri, load_printer_detail_by_id, load_queue_metrics_snapshot,
    mark_managed_discovered_printers as mark_storage_managed_discovered_printers,
    move_printing_job_to_attention, move_submitting_job_to_attention,
    probe_database_health_at_path, record_backend_poll_result, record_backend_unknown,
    recover_inflight_jobs, refresh_printer_snapshot, reset_local_auth_password_by_admin_at_path,
    revoke_api_key_record_at_path, revoke_auth_session_at_path, save_backend_submission,
    save_cups_base_url, save_reconciled_backend_submission, save_render_artifact_result,
    seed_initial_admin_user_if_no_users, set_default_printer_record, touch_api_key_at_path,
    touch_auth_session_at_path, transition_job_status, try_insert_job_event, try_load_render_cache,
    try_upsert_render_cache, update_auth_user_record_at_path, update_local_auth_password_at_path,
    update_template_group_record_at_path, update_template_record_at_path, ApiKeyRecord,
    ApiKeyRecordInput, AuthSessionRecord, AuthUserRecord, AuthUserUpdateInput,
    CacheMetricsSnapshot, DirectJobInsertInput, JobFailureInput, JobRecord,
    LocalAuthUserInsertInput, QueueMetricsSnapshot, RenderArtifactJobUpdateInput, RenderCacheKey,
    TemplateGroupRecord, TemplateJobInsertInput, TemplateRecordInput, TemplateRecordRow,
    API_KEY_STATUS_ACTIVE, METRIC_DEAD_LETTER_TOTAL,
    METRIC_LOG_CLEANUP_TOTAL, METRIC_RENDER_CACHE_HIT_TOTAL, METRIC_RENDER_CACHE_MISS_TOTAL,
    METRIC_RETRY_SCHEDULED_TOTAL, USER_ROLE_ADMIN, USER_ROLE_OPERATOR, USER_STATUS_ACTIVE,
    USER_STATUS_DISABLED,
};

pub use sqlite::has_active_local_auth_user_at_path;
pub use sqlite::is_unique_auth_user_violation;

#[cfg(test)]
pub use sqlite::{
    cleanup_render_cache_by_disk_watermark_conn, count_active_admins, delete_template_record,
    fetch_job_by_id, has_active_local_auth_user, has_table_column, insert_api_key_record,
    insert_auth_session, insert_local_auth_user_record, insert_template_group,
    insert_template_record, load_api_key_by_id, load_api_key_by_prefix_and_hash, load_auth_session,
    load_auth_user, load_failed_jobs_snapshot, load_local_auth_identity,
    load_local_auth_identity_by_user_id, open_connection as open_sqlite_connection,
    read_agent_metric, revoke_api_key_record, revoke_auth_session, touch_api_key,
    touch_auth_session, update_local_auth_password, update_template_record, API_KEY_STATUS_REVOKED,
};

pub use sqlite::claim_next_job;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseTarget {
    Sqlite { path: PathBuf },
}

impl DatabaseTarget {
    pub fn resolve_from_env() -> Result<Self, String> {
        Self::resolve_from_env_with(|key| std::env::var(key).ok())
    }

    fn resolve_from_env_with<F>(mut get_env: F) -> Result<Self, String>
    where
        F: FnMut(&str) -> Option<String>,
    {
        if let Some(value) = get_env("DEEPPRINT_DATABASE_URL") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                if let Some(path) = trimmed.strip_prefix("sqlite://") {
                    if !path.trim().is_empty() {
                        return Ok(Self::Sqlite {
                            path: PathBuf::from(path),
                        });
                    }
                    return Err(
                        "DEEPPRINT_DATABASE_URL is empty after sqlite:// prefix".to_string()
                    );
                }
                return Err(format!(
                    "unsupported DEEPPRINT_DATABASE_URL: {trimmed}. Current build only supports sqlite:// URLs"
                ));
            }
        }

        if let Some(value) = get_env("DEEPPRINT_AGENT_DB_PATH") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Ok(Self::Sqlite {
                    path: PathBuf::from(trimmed),
                });
            }
        }

        let data_dir = get_env("DEEPPRINT_AGENT_DATA_DIR")
            .or_else(|| get_env("DEEPPRINT_DATA_DIR"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("deepprint-server"));

        Ok(Self::Sqlite {
            path: data_dir.join("deepprint.db"),
        })
    }

    pub fn driver_name(&self) -> &'static str {
        match self {
            Self::Sqlite { .. } => "sqlite",
        }
    }

    pub fn sqlite_path(&self) -> &Path {
        match self {
            Self::Sqlite { path } => path.as_path(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use super::DatabaseTarget;

    #[test]
    fn resolve_db_target_prefers_sqlite_database_url() {
        let env = HashMap::from([(
            "DEEPPRINT_DATABASE_URL".to_string(),
            "sqlite:///var/lib/deepprint/deepprint.db".to_string(),
        )]);

        let target =
            DatabaseTarget::resolve_from_env_with(|key| env.get(key).cloned()).expect("sqlite url");
        assert_eq!(
            target,
            DatabaseTarget::Sqlite {
                path: PathBuf::from("/var/lib/deepprint/deepprint.db"),
            }
        );
    }

    #[test]
    fn resolve_db_target_rejects_unsupported_database_url_scheme() {
        let env = HashMap::from([(
            "DEEPPRINT_DATABASE_URL".to_string(),
            "postgres://deepprint:secret@postgres:5432/deepprint".to_string(),
        )]);

        let err = DatabaseTarget::resolve_from_env_with(|key| env.get(key).cloned())
            .expect_err("postgres url");
        assert!(err.contains("only supports sqlite:// URLs"));
    }

    #[test]
    fn resolve_db_target_falls_back_to_agent_db_path() {
        let env = HashMap::from([(
            "DEEPPRINT_AGENT_DB_PATH".to_string(),
            "/tmp/deepprint/custom.db".to_string(),
        )]);

        let target = DatabaseTarget::resolve_from_env_with(|key| env.get(key).cloned())
            .expect("agent db path fallback");
        assert_eq!(
            target,
            DatabaseTarget::Sqlite {
                path: PathBuf::from("/tmp/deepprint/custom.db"),
            }
        );
    }
}
