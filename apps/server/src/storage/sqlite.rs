use std::path::Path;

use rusqlite::Connection;

mod api_keys;
mod auth;
mod bootstrap;
mod jobs;
mod metrics;
mod printers;
mod render_cache;
mod settings;
mod templates;

pub use api_keys::*;
pub use auth::*;
pub use bootstrap::*;
pub use jobs::*;
pub use metrics::*;
pub use printers::*;
pub use render_cache::*;
pub use settings::*;
pub use templates::*;

pub fn open_connection(db_path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 3000;",
    )?;
    Ok(conn)
}
