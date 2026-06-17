use serde::{Deserialize, Serialize};

#[path = "typst_assets/fonts.rs"]
mod fonts;
#[path = "typst_assets/packages.rs"]
mod packages;
#[path = "typst_assets/validation.rs"]
mod validation;

pub(super) use fonts::{
    delete_typst_font, ensure_default_typst_fonts, install_typst_font, list_typst_fonts,
};
pub(super) use packages::{
    clear_typst_preview_cache, delete_typst_package, install_typst_package, list_typst_packages,
};
#[cfg(test)]
pub(super) use fonts::collect_typst_fonts;
#[cfg(test)]
pub(super) use packages::{
    collect_typst_packages_from_namespace, locate_typst_package_root, read_typst_package_manifest,
};
#[cfg(test)]
pub(super) use validation::{
    sanitize_package_segment, sanitize_typst_font_file_name, validate_install_typst_font_payload,
    validate_install_typst_package_payload,
};

#[derive(Debug, Deserialize)]
pub(super) struct InstallTypstPackageRequest {
    pub(super) archive_base64: String,
    #[serde(default)]
    pub(super) file_name: Option<String>,
    #[serde(default)]
    pub(super) replace_existing: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct DeleteTypstPackageRequest {
    origin: TypstPackageOrigin,
    namespace: String,
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct InstallTypstFontRequest {
    pub(super) file_base64: String,
    pub(super) file_name: String,
    #[serde(default)]
    pub(super) replace_existing: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct DeleteTypstFontRequest {
    file_name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum TypstPackageOrigin {
    Local,
    PreviewCache,
}

#[derive(Debug, Deserialize)]
struct TypstPackageToml {
    package: TypstPackageTomlPackage,
}

#[derive(Debug, Deserialize)]
pub(super) struct TypstPackageTomlPackage {
    pub(super) name: String,
    pub(super) version: String,
}

#[derive(Debug, Serialize)]
pub(super) struct TypstPackageInfo {
    pub(super) origin: TypstPackageOrigin,
    pub(super) namespace: String,
    pub(super) name: String,
    pub(super) version: String,
    pub(super) import_snippet: String,
}

#[derive(Debug, Serialize)]
pub(super) struct TypstPackagesResponse {
    pub(super) packages: Vec<TypstPackageInfo>,
}

#[derive(Debug, Serialize)]
pub(super) struct TypstFontInfo {
    pub(super) file_name: String,
    pub(super) size_bytes: i64,
    pub(super) modified_at_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
pub(super) struct TypstFontsResponse {
    pub(super) fonts: Vec<TypstFontInfo>,
}

#[derive(Debug, Serialize)]
pub(super) struct InstallTypstPackageResponse {
    pub(super) origin: TypstPackageOrigin,
    pub(super) namespace: String,
    pub(super) name: String,
    pub(super) version: String,
    pub(super) replaced: bool,
    pub(super) import_snippet: String,
}

#[derive(Debug, Serialize)]
pub(super) struct InstallTypstFontResponse {
    pub(super) file_name: String,
    pub(super) size_bytes: i64,
    pub(super) replaced: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct DeleteTypstPackageResponse {
    pub(super) origin: TypstPackageOrigin,
    pub(super) namespace: String,
    pub(super) name: String,
    pub(super) version: String,
    pub(super) deleted: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct ClearTypstPreviewCacheResponse {
    pub(super) removed: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct DeleteTypstFontResponse {
    file_name: String,
    deleted: bool,
}
