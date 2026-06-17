use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(crate) fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}

pub(crate) fn system_time_to_unix_ms(value: SystemTime) -> Option<i64> {
    let millis = value.duration_since(UNIX_EPOCH).ok()?.as_millis();
    if millis > i64::MAX as u128 {
        Some(i64::MAX)
    } else {
        Some(millis as i64)
    }
}

pub(crate) fn elapsed_millis(started: Instant) -> u64 {
    let elapsed = started.elapsed().as_millis();
    if elapsed > u64::MAX as u128 {
        u64::MAX
    } else {
        elapsed as u64
    }
}
