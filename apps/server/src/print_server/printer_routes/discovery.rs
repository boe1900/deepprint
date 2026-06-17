use std::sync::Arc;

use axum::{extract::State, Json};

use super::super::{
    submission::map_print_backend_error, AgentState, ApiResult, DiscoveredPrintersResponse,
};
use crate::storage::mark_storage_managed_discovered_printers;

pub(crate) async fn discover_cups_printers(
    State(state): State<Arc<AgentState>>,
) -> ApiResult<Json<DiscoveredPrintersResponse>> {
    let cups_base_url = state.current_cups_base_url();
    let mut printers = crate::print_backend::discover::discover_cups_printers(&cups_base_url)
        .await
        .map_err(map_print_backend_error)?;
    mark_storage_managed_discovered_printers(state.db_path.as_ref(), &mut printers)
        .map_err(map_print_backend_error)?;
    let message = if printers.is_empty() {
        Some("已连接到 CUPS，但没有发现可共享导入的打印机。请检查 CUPS 是否开启共享以及打印机是否已发布。".to_string())
    } else {
        None
    };
    Ok(Json(DiscoveredPrintersResponse {
        printers,
        cups_base_url: Some(cups_base_url),
        reachable: Some(true),
        message,
    }))
}

pub(crate) async fn discover_mdns_printers(
    State(state): State<Arc<AgentState>>,
) -> ApiResult<Json<DiscoveredPrintersResponse>> {
    let mut printers = crate::print_backend::discover::discover_mdns_printers().await;
    mark_storage_managed_discovered_printers(state.db_path.as_ref(), &mut printers)
        .map_err(map_print_backend_error)?;
    Ok(Json(DiscoveredPrintersResponse {
        printers,
        cups_base_url: None,
        reachable: None,
        message: None,
    }))
}
