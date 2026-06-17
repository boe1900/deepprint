#[path = "env/groups.rs"]
mod groups;
#[path = "env/helpers.rs"]
mod helpers;
#[path = "env/normalize.rs"]
mod normalize;

use super::AgentConfig;

pub(super) fn agent_config_from_env() -> AgentConfig {
    let mut cfg = AgentConfig::default();

    groups::apply_core_env(&mut cfg);
    groups::apply_render_env(&mut cfg);
    groups::apply_backend_env(&mut cfg);
    groups::apply_log_env(&mut cfg);
    groups::apply_auth_env(&mut cfg);
    normalize::normalize_derived_values(&mut cfg);

    cfg
}
