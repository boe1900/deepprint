use std::{path::Path, sync::Arc};

use axum::{extract::State, Json};

use super::super::super::{AgentState, ApiError, ApiResult};
use super::super::{TypstPackageInfo, TypstPackageOrigin, TypstPackagesResponse};
use super::manifest::read_typst_package_manifest;

pub(crate) async fn list_typst_packages(
    State(state): State<Arc<AgentState>>,
) -> ApiResult<Json<TypstPackagesResponse>> {
    let mut packages = Vec::new();
    packages.extend(collect_typst_packages_from_namespace(
        state.typst_local_packages_root.as_ref(),
        "local",
        TypstPackageOrigin::Local,
    )?);
    packages.extend(collect_typst_packages_from_namespace(
        state.typst_preview_cache_root.as_ref(),
        "preview",
        TypstPackageOrigin::PreviewCache,
    )?);

    packages.sort_by(|left, right| {
        (
            typst_package_origin_order(left.origin),
            left.namespace.as_str(),
            left.name.as_str(),
            left.version.as_str(),
        )
            .cmp(&(
                typst_package_origin_order(right.origin),
                right.namespace.as_str(),
                right.name.as_str(),
                right.version.as_str(),
            ))
    });

    Ok(Json(TypstPackagesResponse { packages }))
}

pub(crate) fn collect_typst_packages_from_namespace(
    root: &Path,
    namespace: &str,
    origin: TypstPackageOrigin,
) -> ApiResult<Vec<TypstPackageInfo>> {
    let namespace = super::super::validation::sanitize_package_segment(namespace, "namespace")?;
    let namespace_dir = root.join(&namespace);
    if !namespace_dir.exists() {
        return Ok(Vec::new());
    }

    let mut packages = Vec::new();
    let package_entries = std::fs::read_dir(&namespace_dir)
        .map_err(|err| ApiError::Internal(format!("read namespace dir failed: {err}")))?;

    for package_entry in package_entries {
        let package_entry = package_entry
            .map_err(|err| ApiError::Internal(format!("read package entry failed: {err}")))?;
        let package_dir = package_entry.path();
        if !package_dir.is_dir() {
            continue;
        }
        let package_name = match super::super::validation::sanitize_package_segment(
            package_entry.file_name().to_string_lossy().as_ref(),
            "package name",
        ) {
            Ok(value) => value,
            Err(_) => continue,
        };

        collect_package_versions(
            &mut packages,
            origin,
            &namespace,
            &package_name,
            &package_dir,
        )?;
    }

    Ok(packages)
}

fn collect_package_versions(
    packages: &mut Vec<TypstPackageInfo>,
    origin: TypstPackageOrigin,
    namespace: &str,
    package_name: &str,
    package_dir: &Path,
) -> ApiResult<()> {
    let version_entries = std::fs::read_dir(package_dir)
        .map_err(|err| ApiError::Internal(format!("read package versions failed: {err}")))?;
    for version_entry in version_entries {
        let version_entry = version_entry
            .map_err(|err| ApiError::Internal(format!("read package version failed: {err}")))?;
        let version_dir = version_entry.path();
        if !version_dir.is_dir() {
            continue;
        }

        let mut resolved_name = package_name.to_string();
        let mut resolved_version = match super::super::validation::sanitize_package_segment(
            version_entry.file_name().to_string_lossy().as_ref(),
            "package version",
        ) {
            Ok(value) => value,
            Err(_) => continue,
        };

        let manifest_path = version_dir.join("typst.toml");
        if manifest_path.is_file() {
            if let Ok(manifest) = read_typst_package_manifest(&manifest_path) {
                if let (Ok(name), Ok(version)) = (
                    super::super::validation::sanitize_package_segment(
                        &manifest.name,
                        "package.name",
                    ),
                    super::super::validation::sanitize_package_segment(
                        &manifest.version,
                        "package.version",
                    ),
                ) {
                    resolved_name = name;
                    resolved_version = version;
                }
            }
        }

        packages.push(TypstPackageInfo {
            origin,
            namespace: namespace.to_string(),
            name: resolved_name.clone(),
            version: resolved_version.clone(),
            import_snippet: if origin == TypstPackageOrigin::PreviewCache {
                format!("#import \"@preview/{resolved_name}:{resolved_version}\": *")
            } else {
                format!("#import \"@local/{resolved_name}:{resolved_version}\": *")
            },
        });
    }

    Ok(())
}

fn typst_package_origin_order(origin: TypstPackageOrigin) -> u8 {
    match origin {
        TypstPackageOrigin::Local => 0,
        TypstPackageOrigin::PreviewCache => 1,
    }
}
