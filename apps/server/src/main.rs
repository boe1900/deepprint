mod print_backend;
mod print_server;
mod printer;
mod renderer;
mod storage;

use print_server::{run_print_server, PrintServerConfig};
use storage::DatabaseTarget;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    if let Some(exit_code) = renderer::maybe_run_renderer_subprocess() {
        std::process::exit(exit_code);
    }

    let config = PrintServerConfig::from_env();
    let database_target = DatabaseTarget::resolve_from_env()?;
    let version = env!("CARGO_PKG_VERSION").to_string();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run_print_server(database_target, version, config))?;
    Ok(())
}

fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("deepprint_server=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
