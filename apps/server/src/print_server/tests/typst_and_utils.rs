use super::*;
use crate::print_server::shared::ENV_TYPST_DEFAULT_FONTS_ROOT;
use crate::print_server::typst_assets::{
    ClearTypstPreviewCacheResponse, InstallTypstFontResponse, InstallTypstPackageResponse,
    TypstFontInfo, TypstFontsResponse, TypstPackageInfo, TypstPackagesResponse,
};

#[test]
fn validate_install_typst_package_payload_requires_archive() {
    let payload = InstallTypstPackageRequest {
        archive_base64: "   ".to_string(),
        file_name: None,
        replace_existing: false,
    };
    let err =
        validate_install_typst_package_payload(&payload).expect_err("should reject empty archive");
    match err {
        ApiError::BadRequest(message) => {
            assert!(message.contains("archive_base64"));
        }
        other => panic!("expected bad request, got {other}"),
    }
}

#[test]
fn sanitize_package_segment_rejects_invalid_values() {
    assert!(sanitize_package_segment("ok_name-1.0", "name").is_ok());
    assert!(sanitize_package_segment("", "name").is_err());
    assert!(sanitize_package_segment("..", "name").is_err());
    assert!(sanitize_package_segment("a/b", "name").is_err());
    assert!(sanitize_package_segment("a\\b", "name").is_err());
}

#[test]
fn locate_typst_package_root_finds_single_nested_root() {
    let staging_dir = build_test_temp_dir("typst-locate-root");
    let package_dir = staging_dir.join("my_pkg");
    fs::create_dir_all(&package_dir).expect("create package dir");
    fs::write(
        package_dir.join("typst.toml"),
        "[package]\nname = \"my_pkg\"\nversion = \"0.1.0\"\n",
    )
    .expect("write typst.toml");

    let located = locate_typst_package_root(&staging_dir).expect("locate package root");
    assert_eq!(located, package_dir);
}

#[test]
fn read_typst_package_manifest_parses_name_and_version() {
    let temp_dir = build_test_temp_dir("typst-manifest");
    let path = temp_dir.join("typst.toml");
    fs::write(
        &path,
        "[package]\nname = \"my_label\"\nversion = \"0.3.2\"\nentrypoint = \"lib.typ\"\n",
    )
    .expect("write typst.toml");

    let manifest = read_typst_package_manifest(&path).expect("read manifest");
    assert_eq!(manifest.name, "my_label");
    assert_eq!(manifest.version, "0.3.2");
}

#[test]
fn collect_typst_packages_from_namespace_builds_expected_import_snippet() {
    let root = build_test_temp_dir("typst-collect");
    let local_pkg_dir = root.join("local").join("my_pkg").join("0.1.0");
    fs::create_dir_all(&local_pkg_dir).expect("create local package path");
    fs::write(
        local_pkg_dir.join("typst.toml"),
        "[package]\nname = \"my_pkg\"\nversion = \"0.1.0\"\n",
    )
    .expect("write local typst.toml");

    let preview_pkg_dir = root.join("preview").join("tiaoma").join("0.3.0");
    fs::create_dir_all(&preview_pkg_dir).expect("create preview package path");
    fs::write(
        preview_pkg_dir.join("typst.toml"),
        "[package]\nname = \"tiaoma\"\nversion = \"0.3.0\"\n",
    )
    .expect("write preview typst.toml");

    let local_packages =
        collect_typst_packages_from_namespace(&root, "local", TypstPackageOrigin::Local)
            .expect("collect local packages");
    assert_eq!(local_packages.len(), 1);
    assert_eq!(
        local_packages[0].import_snippet,
        "#import \"@local/my_pkg:0.1.0\": *"
    );

    let preview_packages =
        collect_typst_packages_from_namespace(&root, "preview", TypstPackageOrigin::PreviewCache)
            .expect("collect preview packages");
    assert_eq!(preview_packages.len(), 1);
    assert_eq!(
        preview_packages[0].import_snippet,
        "#import \"@preview/tiaoma:0.3.0\": *"
    );
}

#[test]
fn typst_package_responses_do_not_expose_server_paths() {
    let package = TypstPackageInfo {
        origin: TypstPackageOrigin::Local,
        namespace: "local".to_string(),
        name: "example".to_string(),
        version: "0.1.0".to_string(),
        import_snippet: "#import \"@local/example:0.1.0\": *".to_string(),
    };
    let packages_json = serde_json::to_value(TypstPackagesResponse {
        packages: vec![package],
    })
    .expect("serialize packages response");
    let packages_object = packages_json
        .as_object()
        .expect("packages response should be an object");
    assert!(!packages_object.contains_key("local_packages_root"));
    assert!(!packages_object.contains_key("preview_cache_root"));
    let first_package = packages_object["packages"][0]
        .as_object()
        .expect("package item should be an object");
    assert!(!first_package.contains_key("install_path"));

    let install_json = serde_json::to_value(InstallTypstPackageResponse {
        origin: TypstPackageOrigin::Local,
        namespace: "local".to_string(),
        name: "example".to_string(),
        version: "0.1.0".to_string(),
        replaced: false,
        import_snippet: "#import \"@local/example:0.1.0\": *".to_string(),
    })
    .expect("serialize install package response");
    let install_object = install_json
        .as_object()
        .expect("install package response should be an object");
    assert!(!install_object.contains_key("install_path"));

    let clear_json = serde_json::to_value(ClearTypstPreviewCacheResponse { removed: true })
        .expect("serialize clear preview cache response");
    let clear_object = clear_json
        .as_object()
        .expect("clear preview cache response should be an object");
    assert!(!clear_object.contains_key("preview_cache_root"));
}

#[test]
fn validate_install_typst_font_payload_requires_fields() {
    let empty_file = InstallTypstFontRequest {
        file_base64: " ".to_string(),
        file_name: "custom.ttf".to_string(),
        replace_existing: false,
    };
    assert!(matches!(
        validate_install_typst_font_payload(&empty_file),
        Err(ApiError::BadRequest(_))
    ));

    let empty_name = InstallTypstFontRequest {
        file_base64: "Zg==".to_string(),
        file_name: " ".to_string(),
        replace_existing: false,
    };
    assert!(matches!(
        validate_install_typst_font_payload(&empty_name),
        Err(ApiError::BadRequest(_))
    ));
}

#[test]
fn sanitize_typst_font_file_name_validates_extension_and_path() {
    assert_eq!(
        sanitize_typst_font_file_name("NotoSansSC-Regular.ttf").expect("accept valid font file"),
        "NotoSansSC-Regular.ttf"
    );
    assert!(sanitize_typst_font_file_name("font.txt").is_err());
    assert!(sanitize_typst_font_file_name("../font.ttf").is_err());
    assert!(sanitize_typst_font_file_name("a/b.ttf").is_err());
    assert!(sanitize_typst_font_file_name(" ").is_err());
}

#[test]
fn collect_typst_fonts_filters_and_sorts_supported_fonts() {
    let root = build_test_temp_dir("typst-fonts-collect");
    fs::write(root.join("b-font.otf"), b"otf").expect("write otf");
    fs::write(root.join("a-font.ttf"), b"ttf").expect("write ttf");
    fs::write(root.join("ignore.txt"), b"txt").expect("write txt");

    let fonts = collect_typst_fonts(&root).expect("collect fonts");
    assert_eq!(fonts.len(), 2);
    assert_eq!(fonts[0].file_name, "a-font.ttf");
    assert_eq!(fonts[1].file_name, "b-font.otf");
    assert!(fonts[0].size_bytes > 0);
}

#[test]
fn typst_font_responses_do_not_expose_server_paths() {
    let font = TypstFontInfo {
        file_name: "Example.ttf".to_string(),
        size_bytes: 1234,
        modified_at_ms: Some(42),
    };
    let fonts_json = serde_json::to_value(TypstFontsResponse {
        fonts: vec![font],
    })
    .expect("serialize fonts response");
    let fonts_object = fonts_json
        .as_object()
        .expect("fonts response should be an object");
    assert!(!fonts_object.contains_key("fonts_root"));
    let first_font = fonts_object["fonts"][0]
        .as_object()
        .expect("font item should be an object");
    assert!(!first_font.contains_key("file_path"));

    let install_json = serde_json::to_value(InstallTypstFontResponse {
        file_name: "Example.ttf".to_string(),
        size_bytes: 1234,
        replaced: false,
    })
    .expect("serialize install response");
    let install_object = install_json
        .as_object()
        .expect("install response should be an object");
    assert!(!install_object.contains_key("file_path"));
}

#[test]
fn ensure_default_typst_fonts_copies_project_fonts_once() {
    let managed_root = build_test_temp_dir("typst-fonts-managed");
    let default_root = build_test_temp_dir("typst-fonts-default");
    std::env::set_var(ENV_TYPST_DEFAULT_FONTS_ROOT, &default_root);
    fs::write(default_root.join("DefaultSans-Regular.ttf"), b"default-ttf")
        .expect("write default font");
    fs::write(default_root.join("ignore.txt"), b"ignore").expect("write ignored file");

    ensure_default_typst_fonts(&managed_root).expect("seed managed fonts");

    let fonts = collect_typst_fonts(&managed_root).expect("collect managed fonts");
    assert_eq!(fonts.len(), 1);
    assert_eq!(fonts[0].file_name, "DefaultSans-Regular.ttf");
    assert!(managed_root.join("DefaultSans-Regular.ttf").exists());

    fs::write(managed_root.join("DefaultSans-Regular.ttf"), b"custom").expect("overwrite");
    ensure_default_typst_fonts(&managed_root).expect("seed should not replace existing file");
    assert_eq!(
        fs::read(managed_root.join("DefaultSans-Regular.ttf")).expect("read managed font"),
        b"custom"
    );

    fs::remove_file(managed_root.join("DefaultSans-Regular.ttf")).expect("delete managed font");
    ensure_default_typst_fonts(&managed_root).expect("initialized root should not re-seed");
    assert!(!managed_root.join("DefaultSans-Regular.ttf").exists());

    std::env::remove_var(ENV_TYPST_DEFAULT_FONTS_ROOT);
}

#[test]
fn normalize_pagination_clamps_page_and_page_size() {
    let window = normalize_pagination(Some(99), Some(999), 123, 50, 200);
    assert_eq!(window.page_size, 200);
    assert_eq!(window.total, 123);
    assert_eq!(window.total_pages, 1);
    assert_eq!(window.page, 1);
    assert_eq!(window.start, 0);

    let window = normalize_pagination(Some(2), Some(50), 120, 50, 200);
    assert_eq!(window.page_size, 50);
    assert_eq!(window.total_pages, 3);
    assert_eq!(window.page, 2);
    assert_eq!(window.start, 50);
}

#[test]
fn normalize_pagination_handles_empty_total() {
    let window = normalize_pagination(None, None, 0, 50, 200);
    assert_eq!(window.page, 1);
    assert_eq!(window.page_size, 50);
    assert_eq!(window.total, 0);
    assert_eq!(window.total_pages, 1);
    assert_eq!(window.start, 0);
}
