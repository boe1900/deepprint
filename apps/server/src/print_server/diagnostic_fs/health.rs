use std::{path::Path, time::Instant};

use super::super::{models::HealthComponentProbe, utils::elapsed_millis};

pub(crate) fn probe_database_health(db_path: &Path) -> HealthComponentProbe {
    let started = Instant::now();
    match crate::storage::probe_database_health_at_path(db_path) {
        Ok(_) => HealthComponentProbe {
            ok: true,
            latency_ms: elapsed_millis(started),
            detail: "database open/read ok".to_string(),
        },
        Err(err) => HealthComponentProbe {
            ok: false,
            latency_ms: elapsed_millis(started),
            detail: format!("database check failed: {err}"),
        },
    }
}
