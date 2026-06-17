use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    middleware::{self},
    routing::{get, post},
    Router,
};

use super::super::{
    auth, diagnostics, jobs, open, printer_routes, settings,
    submission::{create_direct_job, create_job, preview_typst},
    template, typst_assets, AgentState,
};
use super::cors::build_cors_layer;

pub(super) fn build_http_app(state: Arc<AgentState>) -> Router {
    let direct_job_body_limit = compute_direct_job_body_limit(state.config.direct_job_max_bytes);

    Router::new()
        .route("/v1/auth/login", post(auth::auth_login))
        .route("/v1/auth/change-password", post(auth::auth_change_password))
        .route("/v1/auth/logout", post(auth::auth_logout))
        .route("/v1/auth/me", get(auth::auth_me))
        .route("/v1/health", get(diagnostics::health))
        .route("/v1/health/deep", get(diagnostics::deep_health))
        .route("/v1/settings/cups", get(settings::get_cups_settings))
        .route("/v1/jobs", get(jobs::list_jobs))
        .route("/v1/jobs/recent", get(jobs::list_recent_jobs))
        .route("/v1/api-keys", get(auth::list_api_keys))
        .route("/v1/api-keys/create", post(auth::create_api_key))
        .route(
            "/v1/api-keys/{api_key_id}/revoke",
            post(auth::revoke_api_key),
        )
        .route("/v1/open/templates", get(open::open_list_templates))
        .route("/v1/open/me", get(open::open_me))
        .route("/v1/open/printers", get(open::open_list_printers))
        .route(
            "/v1/open/printers/{printer_id}",
            get(open::open_get_printer_detail),
        )
        .route("/v1/open/preview", post(open::open_preview_typst))
        .route("/v1/open/print", post(open::open_create_job))
        .route(
            "/v1/open/print/direct",
            post(open::open_create_direct_job).layer(DefaultBodyLimit::max(direct_job_body_limit)),
        )
        .route("/v1/open/jobs/{job_id}", get(open::open_get_job))
        .route(
            "/v1/open/jobs/by-request-id/{request_id}",
            get(open::open_get_job_by_request_id),
        )
        .route("/v1/users", get(auth::list_users))
        .route("/v1/users/create", post(auth::create_user))
        .route("/v1/users/{user_id}/update", post(auth::update_user))
        .route(
            "/v1/users/{user_id}/reset-password",
            post(auth::reset_user_password),
        )
        .route("/v1/users/{user_id}/delete", post(auth::delete_user))
        .route("/v1/printers", get(printer_routes::list_printers))
        .route(
            "/v1/printers/discover/cups",
            get(printer_routes::discover_cups_printers),
        )
        .route(
            "/v1/printers/discover/mdns",
            get(printer_routes::discover_mdns_printers),
        )
        .route(
            "/v1/printers/{printer_id}",
            get(printer_routes::get_printer_detail),
        )
        .route(
            "/v1/templates/workspace",
            get(template::get_template_workspace),
        )
        .route("/v1/jobs/{job_id}", get(jobs::get_job))
        .route("/v1/typst/packages", get(typst_assets::list_typst_packages))
        .route("/v1/typst/fonts", get(typst_assets::list_typst_fonts))
        .merge(build_write_routes(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::enforce_console_session,
        ))
        .layer(build_cors_layer())
        .with_state(state)
}

fn build_write_routes(state: Arc<AgentState>) -> Router<Arc<AgentState>> {
    let direct_job_body_limit = compute_direct_job_body_limit(state.config.direct_job_max_bytes);

    Router::new()
        .route("/v1/jobs", post(create_job))
        .route(
            "/v1/jobs/direct",
            post(create_direct_job).layer(DefaultBodyLimit::max(direct_job_body_limit)),
        )
        .route("/v1/printers", post(printer_routes::create_printer))
        .route(
            "/v1/printers/validate",
            post(printer_routes::validate_printer),
        )
        .route(
            "/v1/printers/{printer_id}/refresh",
            post(printer_routes::refresh_printer),
        )
        .route(
            "/v1/printers/{printer_id}/enable",
            post(printer_routes::enable_printer),
        )
        .route(
            "/v1/printers/{printer_id}/disable",
            post(printer_routes::disable_printer),
        )
        .route(
            "/v1/printers/{printer_id}/set-default",
            post(printer_routes::set_default_printer),
        )
        .route(
            "/v1/printers/{printer_id}",
            axum::routing::delete(printer_routes::delete_printer),
        )
        .route("/v1/preview/typst", post(preview_typst))
        .route("/v1/settings/cups", post(settings::update_cups_settings))
        .route(
            "/v1/settings/cups/test",
            post(settings::test_cups_connection),
        )
        .route(
            "/v1/templates/groups/create",
            post(template::create_template_group),
        )
        .route(
            "/v1/templates/groups/{group_id}/update",
            post(template::update_template_group),
        )
        .route(
            "/v1/templates/groups/{group_id}/delete",
            post(template::delete_template_group),
        )
        .route("/v1/templates/create", post(template::create_template))
        .route(
            "/v1/templates/{template_id}/update",
            post(template::update_template),
        )
        .route(
            "/v1/templates/{template_id}/delete",
            post(template::delete_template),
        )
        .route("/v1/jobs/{job_id}/cancel", post(jobs::cancel_job))
        .route(
            "/v1/diagnostics/export",
            post(diagnostics::export_diagnostics),
        )
        .route(
            "/v1/typst/packages/install",
            post(typst_assets::install_typst_package),
        )
        .route(
            "/v1/typst/packages/delete",
            post(typst_assets::delete_typst_package),
        )
        .route(
            "/v1/typst/fonts/install",
            post(typst_assets::install_typst_font),
        )
        .route(
            "/v1/typst/fonts/delete",
            post(typst_assets::delete_typst_font),
        )
        .route(
            "/v1/typst/packages/clear-preview-cache",
            post(typst_assets::clear_typst_preview_cache),
        )
}

fn compute_direct_job_body_limit(max_file_bytes: u64) -> usize {
    let base64_bytes = max_file_bytes
        .saturating_add(2)
        .saturating_div(3)
        .saturating_mul(4);
    let json_overhead = 64 * 1024_u64;
    let request_limit = base64_bytes.saturating_add(json_overhead);

    request_limit.min(usize::MAX as u64) as usize
}
