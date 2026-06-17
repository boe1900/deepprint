#[path = "http_server/cors.rs"]
mod cors;
#[path = "http_server/routes.rs"]
mod routes;
#[path = "http_server/serve.rs"]
mod serve;

use std::sync::Arc;

use super::AgentState;

#[cfg(test)]
pub(super) fn build_test_http_app(state: Arc<AgentState>) -> axum::Router {
    routes::build_http_app(state)
}

pub(super) async fn run_http_server(state: Arc<AgentState>) -> Result<(), std::io::Error> {
    let app = routes::build_http_app(state.clone());
    serve::serve_http_app(state, app).await
}
