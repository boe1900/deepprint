use super::super::AgentConfig;

pub(super) fn normalize_derived_values(cfg: &mut AgentConfig) {
    if cfg.render_cache_disk_high_watermark_mb == 0 {
        cfg.render_cache_disk_low_watermark_mb = 0;
    } else if cfg.render_cache_disk_low_watermark_mb > cfg.render_cache_disk_high_watermark_mb {
        cfg.render_cache_disk_low_watermark_mb =
            cfg.render_cache_disk_high_watermark_mb.saturating_mul(8) / 10;
    }

    if cfg.retry_backoff_base_sec > cfg.retry_backoff_max_sec {
        cfg.retry_backoff_base_sec = cfg.retry_backoff_max_sec;
    }
}
