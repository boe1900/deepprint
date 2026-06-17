use std::path::{Path, PathBuf};

use super::{ENV_TYPST_FONTS_ROOT, ENV_TYPST_LOCAL_PACKAGES_ROOT, ENV_TYPST_PREVIEW_CACHE_ROOT};

pub(super) fn configure_typst_package_env(
    local_root: &Path,
    preview_cache_root: &Path,
    fonts_root: &Path,
) {
    std::env::set_var(ENV_TYPST_LOCAL_PACKAGES_ROOT, local_root);
    std::env::set_var(ENV_TYPST_PREVIEW_CACHE_ROOT, preview_cache_root);
    std::env::set_var(ENV_TYPST_FONTS_ROOT, fonts_root);
}

pub(super) fn resolve_typst_local_packages_root() -> PathBuf {
    resolve_path_from_env(
        ENV_TYPST_LOCAL_PACKAGES_ROOT,
        default_typst_local_packages_root(),
    )
}

pub(super) fn resolve_typst_preview_cache_root() -> PathBuf {
    resolve_path_from_env(
        ENV_TYPST_PREVIEW_CACHE_ROOT,
        default_typst_preview_cache_root(),
    )
}

pub(super) fn resolve_typst_fonts_root() -> PathBuf {
    resolve_path_from_env(ENV_TYPST_FONTS_ROOT, default_typst_fonts_root())
}

fn resolve_path_from_env(env_key: &str, fallback: PathBuf) -> PathBuf {
    match std::env::var(env_key) {
        Ok(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                fallback
            } else {
                PathBuf::from(trimmed)
            }
        }
        Err(_) => fallback,
    }
}

fn default_data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(std::env::temp_dir)
}

fn default_typst_local_packages_root() -> PathBuf {
    default_data_dir()
        .join("deepprint-studio")
        .join("typst")
        .join("packages")
}

fn default_typst_preview_cache_root() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("deepprint-studio")
        .join("typst")
        .join("packages")
}

fn default_typst_fonts_root() -> PathBuf {
    default_data_dir()
        .join("deepprint-studio")
        .join("typst")
        .join("fonts")
}

pub(super) fn default_log_dir() -> PathBuf {
    std::env::temp_dir().join("deepprint-studio").join("logs")
}

pub(super) fn default_diagnostics_dir() -> PathBuf {
    std::env::temp_dir()
        .join("deepprint-studio")
        .join("diagnostics")
}
