pub(super) fn set_string(env_key: &str, target: &mut String) {
    if let Ok(value) = std::env::var(env_key) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            *target = trimmed.to_string();
        }
    }
}

pub(super) fn set_parsed<T, F>(env_key: &str, target: &mut T, normalize: F)
where
    T: std::str::FromStr,
    F: FnOnce(T) -> T,
{
    if let Ok(value) = std::env::var(env_key) {
        if let Ok(parsed) = value.trim().parse::<T>() {
            *target = normalize(parsed);
        }
    }
}

pub(super) fn bool_env(env_key: &str) -> Option<bool> {
    let value = std::env::var(env_key).ok()?;
    let normalized = value.trim().to_lowercase();
    Some(matches!(normalized.as_str(), "1" | "true" | "yes" | "on"))
}
