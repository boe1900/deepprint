use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use tracing::info;

use super::super::AgentState;

pub(super) async fn serve_http_app(
    state: Arc<AgentState>,
    app: Router,
) -> Result<(), std::io::Error> {
    let addr = SocketAddr::from((
        state
            .config
            .bind_addr
            .parse::<std::net::IpAddr>()
            .unwrap_or(std::net::IpAddr::from([127, 0, 0, 1])),
        state.config.port,
    ));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(
        "local print agent listening on http://{}:{} (mock_mode={}, backend={})",
        state.config.bind_addr,
        state.config.port,
        state.config.mock_mode,
        state.backend.backend_name(),
    );

    axum::serve(listener, app).await
}
