use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{extract::State, Json};
use uuid::Uuid;

use super::{AgentState, DeepHealthResponse};
use crate::print_server::diagnostic_fs::probe_database_health;
use crate::print_server::models::HealthComponentProbe;
use crate::print_server::utils::elapsed_millis;
use crate::renderer::{self, RenderRequest};

pub(super) async fn deep_health(State(state): State<Arc<AgentState>>) -> Json<DeepHealthResponse> {
    let db = probe_database_health(state.db_path.as_ref());
    let backend = probe_printer_backend_health(state.clone()).await;
    let renderer_subprocess = probe_renderer_subprocess_health(state.clone()).await;
    let overall_ok = db.ok && backend.ok && renderer_subprocess.ok;

    Json(DeepHealthResponse {
        status: if overall_ok { "ok" } else { "degraded" },
        version: state.version.clone(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
        database_driver: state.database_target.driver_name(),
        overall_ok,
        db,
        backend,
        renderer_subprocess,
    })
}

async fn probe_printer_backend_health(state: Arc<AgentState>) -> HealthComponentProbe {
    let started = Instant::now();
    let backend = state.backend.clone();
    let timeout = Duration::from_secs(5);

    match tokio::time::timeout(
        timeout,
        tokio::task::spawn_blocking(move || backend.list_printers()),
    )
    .await
    {
        Ok(Ok(Ok(printers))) => HealthComponentProbe {
            ok: true,
            latency_ms: elapsed_millis(started),
            detail: format!(
                "backend={} discovered printers={}",
                state.backend.backend_name(),
                printers.len()
            ),
        },
        Ok(Ok(Err(err))) => HealthComponentProbe {
            ok: false,
            latency_ms: elapsed_millis(started),
            detail: format!("backend error [{}]: {}", err.code(), err.message()),
        },
        Ok(Err(err)) => HealthComponentProbe {
            ok: false,
            latency_ms: elapsed_millis(started),
            detail: format!("backend join error: {err}"),
        },
        Err(_) => HealthComponentProbe {
            ok: false,
            latency_ms: elapsed_millis(started),
            detail: format!("backend probe timeout after {}s", timeout.as_secs()),
        },
    }
}

async fn probe_renderer_subprocess_health(state: Arc<AgentState>) -> HealthComponentProbe {
    let started = Instant::now();
    let timeout_sec = state.config.render_timeout_sec.clamp(5, 15);
    let timeout = Duration::from_secs(timeout_sec);
    let request = RenderRequest {
        job_id: format!("health-probe-{}", Uuid::new_v4()),
        request_id: format!("health-probe-{}", Uuid::new_v4()),
        template_content: "#set page(width: 60mm, height: auto)\nHealth #data.ok".to_string(),
        data: serde_json::json!({ "ok": "ok" }),
        print_options: serde_json::json!({}),
    };

    match renderer::render_via_subprocess(&request, timeout).await {
        Ok(result) => {
            let _ = std::fs::remove_file(&result.artifact_path);
            HealthComponentProbe {
                ok: true,
                latency_ms: elapsed_millis(started),
                detail: format!(
                    "renderer ok output_kind={}, page_count={}",
                    result.output_kind, result.page_count
                ),
            }
        }
        Err(err) => HealthComponentProbe {
            ok: false,
            latency_ms: elapsed_millis(started),
            detail: format!("renderer probe failed: {err}"),
        },
    }
}
