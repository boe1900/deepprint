use axum::{extract::State, Json};
use std::sync::Arc;

#[path = "diagnostics/deep_health.rs"]
mod deep_health_probe;
#[path = "diagnostics/export.rs"]
mod export;
#[path = "diagnostics/health.rs"]
mod health_endpoint;
#[path = "diagnostics/models.rs"]
mod models;

use super::{AgentState, ApiResult};

pub(super) use models::{
    DeepHealthResponse, DiagnosticConfigSnapshot, DiagnosticExportRequest,
    DiagnosticExportResponse, DiagnosticHealthSnapshot, DiagnosticManifest, HealthResponse,
};

pub(super) async fn health(State(state): State<Arc<AgentState>>) -> Json<HealthResponse> {
    health_endpoint::health(State(state)).await
}

pub(super) async fn deep_health(State(state): State<Arc<AgentState>>) -> Json<DeepHealthResponse> {
    deep_health_probe::deep_health(State(state)).await
}

pub(super) async fn export_diagnostics(
    State(state): State<Arc<AgentState>>,
    payload: Option<Json<DiagnosticExportRequest>>,
) -> ApiResult<Json<DiagnosticExportResponse>> {
    export::export_diagnostics(State(state), payload).await
}
