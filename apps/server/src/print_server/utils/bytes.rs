pub(crate) fn clamp_u64_to_i64(value: u64) -> i64 {
    if value > i64::MAX as u64 {
        i64::MAX
    } else {
        value as i64
    }
}

pub(crate) fn mb_to_bytes(value_mb: u64) -> u64 {
    value_mb.saturating_mul(1024).saturating_mul(1024)
}
