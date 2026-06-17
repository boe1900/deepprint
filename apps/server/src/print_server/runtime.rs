use std::{sync::Arc, time::Instant};
use tracing::warn;

use super::{
    background_tasks::spawn_background_tasks,
    bootstrap,
    http_server::run_http_server,
    now_unix,
    platform_paths::{
        configure_typst_package_env, resolve_typst_fonts_root, resolve_typst_local_packages_root,
        resolve_typst_preview_cache_root,
    },
    typst_assets,
    AgentBootError, AgentConfig, AgentState, PrintServerBootError, PrintServerConfig,
};
use crate::{
    printer,
    storage::{init_schema, recover_inflight_jobs, DatabaseTarget},
};

pub async fn run(
    database_target: DatabaseTarget,
    version: String,
    mut config: AgentConfig,
) -> Result<(), AgentBootError> {
    let db_path = database_target.sqlite_path().to_path_buf();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    init_schema(&db_path)?;
    if let Ok(Some(saved_cups_base_url)) = crate::storage::load_cups_base_url(&db_path) {
        if !saved_cups_base_url.trim().is_empty() {
            config.cups_base_url = saved_cups_base_url;
        }
    }
    bootstrap::seed_initial_admin_if_configured(&db_path, now_unix()).map_err(|err| {
        AgentBootError::InvalidConfig(format!("initialize admin user failed: {err}"))
    })?;
    let recovered_jobs = recover_inflight_jobs(&db_path, config.mock_mode)?;
    if recovered_jobs.rendering_requeued > 0 {
        warn!(
            "recovered {} rendering jobs to queued",
            recovered_jobs.rendering_requeued
        );
    }
    if recovered_jobs.printing_requeued > 0 {
        warn!(
            "recovered {} printing jobs to queued for non-durable mock backend",
            recovered_jobs.printing_requeued
        );
    }
    let typst_local_packages_root = resolve_typst_local_packages_root();
    let typst_preview_cache_root = resolve_typst_preview_cache_root();
    let typst_fonts_root = resolve_typst_fonts_root();
    std::fs::create_dir_all(typst_local_packages_root.join("local"))?;
    std::fs::create_dir_all(&typst_preview_cache_root)?;
    std::fs::create_dir_all(&typst_fonts_root)?;
    typst_assets::ensure_default_typst_fonts(&typst_fonts_root)
        .map_err(|err| AgentBootError::InvalidConfig(format!("initialize default fonts failed: {err}")))?;
    configure_typst_package_env(
        &typst_local_packages_root,
        &typst_preview_cache_root,
        &typst_fonts_root,
    );

    let backend = printer::create_backend(config.mock_mode);

    let state = Arc::new(AgentState {
        database_target: Arc::new(database_target),
        db_path: Arc::new(db_path),
        started_at: Instant::now(),
        version,
        cups_base_url: Arc::new(std::sync::RwLock::new(config.cups_base_url.clone())),
        config,
        backend,
        typst_local_packages_root: Arc::new(typst_local_packages_root),
        typst_preview_cache_root: Arc::new(typst_preview_cache_root),
        typst_fonts_root: Arc::new(typst_fonts_root),
    });

    spawn_background_tasks(state.clone());

    run_http_server(state).await?;
    Ok(())
}

pub async fn run_print_server(
    database_target: DatabaseTarget,
    version: String,
    config: PrintServerConfig,
) -> Result<(), PrintServerBootError> {
    run(database_target, version, config).await
}
