use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use lopdf::{Dictionary, Document, Object};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::process::Command;
use tracing::debug;
use typst_as_lib::{
    cached_file_resolver::IntoCachedFileResolver,
    file_resolver::FileSystemResolver,
    package_resolver::{FileSystemCache, PackageResolver},
    typst_kit_options::TypstKitFontOptions,
    TypstEngine,
};

pub const RENDERER_MODE_FLAG: &str = "--deepprint-renderer";
const REQUEST_FILE_FLAG: &str = "--request-file";
const DEFAULT_MAIN_FILE: &str = "main.typ";
const DATA_FILE: &str = "data.json";
const PRINT_OPTIONS_FILE: &str = "print_options.json";
const ENV_TYPST_LOCAL_PACKAGES_ROOT: &str = "DEEPPRINT_TYPST_LOCAL_PACKAGES_ROOT";
const ENV_TYPST_PREVIEW_CACHE_ROOT: &str = "DEEPPRINT_TYPST_PREVIEW_CACHE_ROOT";
const ENV_TYPST_FONTS_ROOT: &str = "DEEPPRINT_TYPST_FONTS_ROOT";

static PREVIEW_RENDERER: LazyLock<Mutex<Option<PreviewTypstRenderer>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderRequest {
    pub job_id: String,
    pub request_id: String,
    pub template_content: String,
    pub data: Value,
    pub print_options: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderResult {
    pub artifact_path: String,
    pub output_kind: String,
    pub page_count: u32,
    pub page_width_pt: Option<f64>,
    pub page_height_pt: Option<f64>,
}

struct PreviewTypstRenderer {
    workspace_root: PathBuf,
    engine: TypstEngine,
}

impl PreviewTypstRenderer {
    fn new() -> Result<Self, RenderError> {
        let workspace_root = build_preview_render_root();
        fs::create_dir_all(&workspace_root).map_err(|err| {
            RenderError::RequestFileIo(format!(
                "create preview render root {} failed: {err}",
                workspace_root.display()
            ))
        })?;

        let fonts_root = resolve_typst_fonts_root();
        let font_options = TypstKitFontOptions::new()
            .include_system_fonts(false)
            .include_embedded_fonts(true)
            .include_dirs([fonts_root]);
        let local_packages_root = resolve_typst_local_packages_root();
        let preview_cache_root = resolve_typst_preview_cache_root();

        let file_system_resolver =
            FileSystemResolver::new(workspace_root.clone()).local_package_root(local_packages_root);
        let package_resolver = PackageResolver::builder()
            .cache(FileSystemCache(preview_cache_root))
            .build()
            .into_cached();

        let mut builder = TypstEngine::builder()
            .search_fonts_with(font_options)
            .add_file_resolver(file_system_resolver)
            .add_file_resolver(package_resolver);
        let _ = builder.comemo_evict_max_age(None);

        let engine = builder.build();
        debug!(
            workspace_root = %workspace_root.display(),
            "initialized typst preview renderer"
        );

        Ok(Self {
            workspace_root,
            engine,
        })
    }

    fn render(&mut self, request: &RenderRequest) -> Result<RenderResult, RenderError> {
        let render_started = Instant::now();
        let prepare_started = Instant::now();
        write_render_root_files(&self.workspace_root, request)?;
        let prepare_ms = prepare_started.elapsed().as_millis();

        let compile_started = Instant::now();
        let warned = self.engine.compile(DEFAULT_MAIN_FILE);
        let doc = warned.output.map_err(|err| {
            let details = err.to_string();
            RenderError::TypstCompileFailed {
                kind: classify_typst_error_kind(&details),
                details,
            }
        })?;
        let compile_ms = compile_started.elapsed().as_millis();

        let pdf_started = Instant::now();
        let pdf_bytes = typst_pdf::pdf(&doc, &Default::default())
            .map_err(|err| RenderError::TypstPdfFailed(format!("{err:?}")))?;
        let pdf_ms = pdf_started.elapsed().as_millis();

        let artifact_dir = build_artifact_dir();
        fs::create_dir_all(&artifact_dir).map_err(|err| {
            RenderError::RequestFileIo(format!(
                "create artifact dir {} failed: {err}",
                artifact_dir.display()
            ))
        })?;

        let artifact_path = artifact_dir.join(format!("{}.pdf", request.job_id));
        fs::write(&artifact_path, &pdf_bytes).map_err(|err| {
            RenderError::RequestFileIo(format!(
                "write pdf {} failed: {err}",
                artifact_path.display()
            ))
        })?;

        let metadata_started = Instant::now();
        let (page_count, page_width_pt, page_height_pt) = read_pdf_metadata_from_bytes(&pdf_bytes)?;
        let metadata_ms = metadata_started.elapsed().as_millis();

        debug!(
            job_id = %request.job_id,
            total_ms = render_started.elapsed().as_millis(),
            prepare_ms,
            compile_ms,
            pdf_ms,
            metadata_ms,
            page_count,
            "typst preview rendered with warm engine"
        );

        Ok(RenderResult {
            artifact_path: artifact_path.to_string_lossy().to_string(),
            output_kind: RenderEngine::Typst.as_str().to_string(),
            page_count,
            page_width_pt,
            page_height_pt,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum RenderEngine {
    Typst,
    Text,
}

impl RenderEngine {
    fn from_env() -> Result<Self, RenderError> {
        let raw = std::env::var("DEEPPRINT_RENDER_ENGINE").unwrap_or_else(|_| "typst".to_string());
        let normalized = raw.trim().to_lowercase();
        match normalized.as_str() {
            "" | "typst" => Ok(Self::Typst),
            "text" => Ok(Self::Text),
            _ => Err(RenderError::InvalidRenderEngine(raw)),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Typst => "typst",
            Self::Text => "text",
        }
    }
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("missing --request-file argument")]
    MissingRequestFileArg,
    #[error("invalid renderer argument: {0}")]
    InvalidCliArg(String),
    #[error("invalid DEEPPRINT_RENDER_ENGINE: {0} (expected typst/text)")]
    InvalidRenderEngine(String),
    #[error("request file io failed: {0}")]
    RequestFileIo(String),
    #[error("serialize render payload failed: {0}")]
    SerializePayload(String),
    #[error("parse render payload failed: {0}")]
    ParsePayload(String),
    #[error("resolve current executable failed: {0}")]
    ResolveCurrentExe(String),
    #[error("spawn renderer subprocess failed: {0}")]
    SpawnSubprocess(String),
    #[error("renderer subprocess timeout after {0}s")]
    Timeout(u64),
    #[error("renderer subprocess failed with status={status}: {stderr}")]
    SubprocessFailed { status: i32, stderr: String },
    #[error("typst compile failed ({kind}): {details}")]
    TypstCompileFailed { kind: &'static str, details: String },
    #[error("typst pdf generation failed: {0}")]
    TypstPdfFailed(String),
    #[error("pdf metadata extraction failed: {0}")]
    PdfMetadata(String),
    #[error("parse render response failed: {0}")]
    ParseResponse(String),
    #[error("renderer output artifact not found: {0}")]
    ArtifactNotFound(String),
    #[error("preview renderer mutex poisoned")]
    PreviewRendererPoisoned,
    #[error("preview renderer task join failed: {0}")]
    PreviewRendererJoin(String),
}

pub fn maybe_run_renderer_subprocess() -> Option<i32> {
    let mut args = std::env::args().skip(1);
    let mode_flag = args.next()?;

    if mode_flag != RENDERER_MODE_FLAG {
        return None;
    }

    let exit_code = match parse_request_file_arg(args).and_then(|path| run_renderer_cli(&path)) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    };

    Some(exit_code)
}

pub async fn render_via_subprocess(
    request: &RenderRequest,
    timeout: Duration,
) -> Result<RenderResult, RenderError> {
    let request_path = build_request_file_path(&request.job_id);
    let request_dir = request_path.parent().ok_or_else(|| {
        RenderError::RequestFileIo("unable to resolve renderer request directory".to_string())
    })?;

    fs::create_dir_all(request_dir)
        .map_err(|err| RenderError::RequestFileIo(format!("create request dir failed: {err}")))?;

    let payload = serde_json::to_vec(request)
        .map_err(|err| RenderError::SerializePayload(err.to_string()))?;
    fs::write(&request_path, payload)
        .map_err(|err| RenderError::RequestFileIo(format!("write request file failed: {err}")))?;

    let current_exe =
        std::env::current_exe().map_err(|err| RenderError::ResolveCurrentExe(err.to_string()))?;

    let mut command = Command::new(current_exe);
    command
        .arg(RENDERER_MODE_FLAG)
        .arg(REQUEST_FILE_FLAG)
        .arg(&request_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output_result = tokio::time::timeout(timeout, command.output()).await;
    let _ = fs::remove_file(&request_path);

    let output = match output_result {
        Ok(Ok(output)) => output,
        Ok(Err(err)) => return Err(RenderError::SpawnSubprocess(err.to_string())),
        Err(_) => return Err(RenderError::Timeout(timeout.as_secs())),
    };

    if !output.status.success() {
        return Err(RenderError::SubprocessFailed {
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let result: RenderResult = serde_json::from_slice(&output.stdout)
        .map_err(|err| RenderError::ParseResponse(err.to_string()))?;

    let artifact = PathBuf::from(&result.artifact_path);
    if !artifact.exists() {
        return Err(RenderError::ArtifactNotFound(result.artifact_path));
    }

    Ok(result)
}

pub async fn render_preview_in_process(
    request: RenderRequest,
) -> Result<RenderResult, RenderError> {
    tokio::task::spawn_blocking(move || {
        let mut guard = PREVIEW_RENDERER
            .lock()
            .map_err(|_| RenderError::PreviewRendererPoisoned)?;
        if guard.is_none() {
            *guard = Some(PreviewTypstRenderer::new()?);
        }
        guard
            .as_mut()
            .expect("preview renderer initialized")
            .render(&request)
    })
    .await
    .map_err(|err| RenderError::PreviewRendererJoin(err.to_string()))?
}

pub fn invalidate_preview_renderer() {
    if let Ok(mut guard) = PREVIEW_RENDERER.lock() {
        *guard = None;
    }
}

pub async fn warmup_preview_renderer() -> Result<(), RenderError> {
    tokio::task::spawn_blocking(move || {
        let mut guard = PREVIEW_RENDERER
            .lock()
            .map_err(|_| RenderError::PreviewRendererPoisoned)?;
        if guard.is_none() {
            *guard = Some(PreviewTypstRenderer::new()?);
        }
        Ok::<(), RenderError>(())
    })
    .await
    .map_err(|err| RenderError::PreviewRendererJoin(err.to_string()))?
}

fn parse_request_file_arg(args: impl Iterator<Item = String>) -> Result<PathBuf, RenderError> {
    let mut iter = args;
    let Some(arg) = iter.next() else {
        return Err(RenderError::MissingRequestFileArg);
    };
    if arg != REQUEST_FILE_FLAG {
        return Err(RenderError::InvalidCliArg(arg));
    }

    let Some(path) = iter.next() else {
        return Err(RenderError::MissingRequestFileArg);
    };
    Ok(PathBuf::from(path))
}

fn run_renderer_cli(request_file: &Path) -> Result<(), RenderError> {
    let raw = fs::read(request_file).map_err(|err| {
        RenderError::RequestFileIo(format!(
            "read request file {} failed: {err}",
            request_file.display()
        ))
    })?;

    let request: RenderRequest =
        serde_json::from_slice(&raw).map_err(|err| RenderError::ParsePayload(err.to_string()))?;

    let engine = RenderEngine::from_env()?;
    let result = match engine {
        RenderEngine::Typst => render_typst_pdf(&request)?,
        RenderEngine::Text => render_text_artifact(&request)?,
    };

    let output = serde_json::to_string(&result)
        .map_err(|err| RenderError::SerializePayload(err.to_string()))?;
    println!("{output}");

    Ok(())
}

fn render_typst_pdf(request: &RenderRequest) -> Result<RenderResult, RenderError> {
    let root_dir = build_render_root(&request.job_id);
    write_render_root_files(&root_dir, request)?;

    let local_packages_root = resolve_typst_local_packages_root();
    let preview_cache_root = resolve_typst_preview_cache_root();
    let fonts_root = resolve_typst_fonts_root();
    let font_options = TypstKitFontOptions::new()
        .include_system_fonts(false)
        .include_embedded_fonts(true)
        .include_dirs([fonts_root]);

    let file_system_resolver = FileSystemResolver::new(root_dir.clone())
        .local_package_root(local_packages_root)
        .into_cached();
    let package_resolver = PackageResolver::builder()
        .cache(FileSystemCache(preview_cache_root))
        .build()
        .into_cached();

    let engine = TypstEngine::builder()
        .search_fonts_with(font_options)
        .add_file_resolver(file_system_resolver)
        .add_file_resolver(package_resolver)
        .build();

    let warned = engine.compile(DEFAULT_MAIN_FILE);
    let doc = warned.output.map_err(|err| {
        let details = err.to_string();
        RenderError::TypstCompileFailed {
            kind: classify_typst_error_kind(&details),
            details,
        }
    })?;

    let pdf_bytes = typst_pdf::pdf(&doc, &Default::default())
        .map_err(|err| RenderError::TypstPdfFailed(format!("{err:?}")))?;

    let artifact_dir = build_artifact_dir();
    fs::create_dir_all(&artifact_dir).map_err(|err| {
        RenderError::RequestFileIo(format!(
            "create artifact dir {} failed: {err}",
            artifact_dir.display()
        ))
    })?;

    let artifact_path = artifact_dir.join(format!("{}.pdf", request.job_id));
    fs::write(&artifact_path, pdf_bytes).map_err(|err| {
        RenderError::RequestFileIo(format!(
            "write pdf {} failed: {err}",
            artifact_path.display()
        ))
    })?;

    let (page_count, page_width_pt, page_height_pt) = read_pdf_metadata(&artifact_path)?;

    Ok(RenderResult {
        artifact_path: artifact_path.to_string_lossy().to_string(),
        output_kind: RenderEngine::Typst.as_str().to_string(),
        page_count,
        page_width_pt,
        page_height_pt,
    })
}

fn build_typst_source(template_content: &str) -> String {
    format!(
        "#let data = json(\"{}\")\n#let print_options = json(\"{}\")\n\n{}\n",
        DATA_FILE, PRINT_OPTIONS_FILE, template_content
    )
}

fn write_render_root_files(root_dir: &Path, request: &RenderRequest) -> Result<(), RenderError> {
    fs::create_dir_all(root_dir).map_err(|err| {
        RenderError::RequestFileIo(format!(
            "create render root {} failed: {err}",
            root_dir.display()
        ))
    })?;

    let main_file_path = root_dir.join(DEFAULT_MAIN_FILE);
    let data_file_path = root_dir.join(DATA_FILE);
    let print_options_file_path = root_dir.join(PRINT_OPTIONS_FILE);

    let main_source = build_typst_source(&request.template_content);
    fs::write(&main_file_path, main_source).map_err(|err| {
        RenderError::RequestFileIo(format!("write {} failed: {err}", main_file_path.display()))
    })?;

    let data_pretty = serde_json::to_vec_pretty(&request.data)
        .map_err(|err| RenderError::SerializePayload(err.to_string()))?;
    fs::write(&data_file_path, data_pretty).map_err(|err| {
        RenderError::RequestFileIo(format!("write {} failed: {err}", data_file_path.display()))
    })?;

    let print_options_pretty = serde_json::to_vec_pretty(&request.print_options)
        .map_err(|err| RenderError::SerializePayload(err.to_string()))?;
    fs::write(&print_options_file_path, print_options_pretty).map_err(|err| {
        RenderError::RequestFileIo(format!(
            "write {} failed: {err}",
            print_options_file_path.display()
        ))
    })?;

    Ok(())
}

fn render_text_artifact(request: &RenderRequest) -> Result<RenderResult, RenderError> {
    let artifact_dir = build_artifact_dir();
    fs::create_dir_all(&artifact_dir).map_err(|err| {
        RenderError::RequestFileIo(format!(
            "create artifact dir {} failed: {err}",
            artifact_dir.display()
        ))
    })?;

    let artifact_path = artifact_dir.join(format!("{}.txt", request.job_id));
    let data = serde_json::to_string_pretty(&request.data)
        .map_err(|err| RenderError::SerializePayload(err.to_string()))?;
    let print_options = serde_json::to_string_pretty(&request.print_options)
        .map_err(|err| RenderError::SerializePayload(err.to_string()))?;

    let content = format!(
        "DeepPrint Render Artifact (subprocess)\n\njob_id: {}\nrequest_id: {}\n\n--- template ---\n{}\n\n--- data ---\n{}\n\n--- print_options ---\n{}\n",
        request.job_id, request.request_id, request.template_content, data, print_options
    );

    fs::write(&artifact_path, content).map_err(|err| {
        RenderError::RequestFileIo(format!(
            "write text artifact {} failed: {err}",
            artifact_path.display()
        ))
    })?;

    Ok(RenderResult {
        artifact_path: artifact_path.to_string_lossy().to_string(),
        output_kind: RenderEngine::Text.as_str().to_string(),
        page_count: 1,
        page_width_pt: None,
        page_height_pt: None,
    })
}

fn classify_typst_error_kind(details: &str) -> &'static str {
    let msg = details.to_lowercase();
    if msg.contains("syntax") {
        return "template_syntax";
    }
    if msg.contains("unknown variable")
        || msg.contains("unknown function")
        || msg.contains("cannot")
        || msg.contains("expected")
    {
        return "template_semantic";
    }
    if msg.contains("file error")
        || msg.contains("does not exist")
        || msg.contains("file not found")
        || msg.contains("not found")
        || msg.contains("read")
    {
        return "resource_missing";
    }
    "compile_failed"
}

fn read_pdf_metadata(path: &Path) -> Result<(u32, Option<f64>, Option<f64>), RenderError> {
    let doc = Document::load(path).map_err(|err| RenderError::PdfMetadata(err.to_string()))?;
    read_pdf_metadata_from_document(&doc)
}

fn read_pdf_metadata_from_bytes(
    bytes: &[u8],
) -> Result<(u32, Option<f64>, Option<f64>), RenderError> {
    let doc = Document::load_mem(bytes).map_err(|err| RenderError::PdfMetadata(err.to_string()))?;
    read_pdf_metadata_from_document(&doc)
}

fn read_pdf_metadata_from_document(
    doc: &Document,
) -> Result<(u32, Option<f64>, Option<f64>), RenderError> {
    let pages = doc.get_pages();
    let page_count = pages.len() as u32;

    let mut width = None;
    let mut height = None;

    if let Some((_, object_id)) = pages.iter().next() {
        if let Some((w, h)) = first_page_size_pt(&doc, *object_id) {
            width = Some(w);
            height = Some(h);
        }
    }

    Ok((page_count, width, height))
}

fn first_page_size_pt(doc: &Document, page_id: lopdf::ObjectId) -> Option<(f64, f64)> {
    let page_obj = doc.get_object(page_id).ok()?;
    let page_dict = as_dict(doc, page_obj)?;
    let media_box = page_dict.get(b"MediaBox").ok()?;
    let media_box = resolve_object(doc, media_box)?;
    let arr = match media_box {
        Object::Array(values) => values,
        _ => return None,
    };

    if arr.len() < 4 {
        return None;
    }

    let x0 = object_to_f64(doc, &arr[0])?;
    let y0 = object_to_f64(doc, &arr[1])?;
    let x1 = object_to_f64(doc, &arr[2])?;
    let y1 = object_to_f64(doc, &arr[3])?;

    Some(((x1 - x0).abs(), (y1 - y0).abs()))
}

fn as_dict<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Dictionary> {
    match resolve_object(doc, obj)? {
        Object::Dictionary(dict) => Some(dict),
        _ => None,
    }
}

fn resolve_object<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
    match obj {
        Object::Reference(id) => doc.get_object(*id).ok(),
        _ => Some(obj),
    }
}

fn object_to_f64(doc: &Document, obj: &Object) -> Option<f64> {
    match resolve_object(doc, obj)? {
        Object::Integer(v) => Some(*v as f64),
        Object::Real(v) => Some(*v as f64),
        _ => None,
    }
}

fn resolve_typst_local_packages_root() -> PathBuf {
    resolve_path_from_env(
        ENV_TYPST_LOCAL_PACKAGES_ROOT,
        default_typst_local_packages_root(),
    )
}

fn resolve_typst_preview_cache_root() -> PathBuf {
    resolve_path_from_env(
        ENV_TYPST_PREVIEW_CACHE_ROOT,
        default_typst_preview_cache_root(),
    )
}

fn resolve_typst_fonts_root() -> PathBuf {
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

fn default_typst_local_packages_root() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
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
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("deepprint-studio")
        .join("typst")
        .join("fonts")
}

fn build_artifact_dir() -> PathBuf {
    std::env::temp_dir()
        .join("deepprint-studio")
        .join("artifacts")
}

fn build_render_root(job_id: &str) -> PathBuf {
    std::env::temp_dir()
        .join("deepprint-studio")
        .join("render-work")
        .join(job_id)
}

fn build_preview_render_root() -> PathBuf {
    std::env::temp_dir()
        .join("deepprint-studio")
        .join("render-work")
        .join("_preview")
}

fn build_request_file_path(job_id: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis();
    std::env::temp_dir()
        .join("deepprint-studio")
        .join("renderer-requests")
        .join(format!("{job_id}-{ts}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_typst_error_kind_maps_file_not_found_to_resource_missing() {
        let message = "Typst source error: file not found (searched at /tmp/abc)";
        assert_eq!(classify_typst_error_kind(message), "resource_missing");
    }

    #[test]
    fn classify_typst_error_kind_maps_unknown_symbol_to_template_semantic() {
        let message = "unknown function: foo_bar";
        assert_eq!(classify_typst_error_kind(message), "template_semantic");
    }
}
