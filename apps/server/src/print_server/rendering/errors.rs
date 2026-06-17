use super::super::models::{ApiError, ProcessJobError};
use crate::renderer::RenderError;

pub(crate) fn map_preview_render_error(err: RenderError) -> ApiError {
    let mapped = map_render_error(err);
    if mapped.retryable {
        return ApiError::ServiceUnavailable(format!("[{}] {}", mapped.code, mapped.message));
    }

    if mapped.code.starts_with("RENDER_TEMPLATE")
        || mapped.code.starts_with("RENDER_RESOURCE")
        || mapped.code.starts_with("RENDER_COMPILE")
        || mapped.code.starts_with("RENDER_PDF")
        || mapped.code == "RENDER_ENGINE_INVALID"
    {
        return ApiError::BadRequest(format!("[{}] {}", mapped.code, mapped.message));
    }

    ApiError::Internal(format!("[{}] {}", mapped.code, mapped.message))
}

pub(super) fn map_render_error(err: RenderError) -> ProcessJobError {
    match err {
        RenderError::Timeout(sec) => ProcessJobError::retryable(
            "RENDER_TIMEOUT",
            format!("renderer subprocess timed out after {sec}s"),
        ),
        RenderError::SubprocessFailed { status, stderr } => {
            let code = classify_render_subprocess_error_code(&stderr);
            ProcessJobError::new(
                code,
                format!("renderer subprocess failed with status={status}: {stderr}"),
            )
        }
        RenderError::ArtifactNotFound(path) => ProcessJobError::new(
            "RENDER_ARTIFACT_MISSING",
            format!("renderer output artifact not found: {path}"),
        ),
        RenderError::ParseResponse(msg) => ProcessJobError::new(
            "RENDER_PROTOCOL_INVALID",
            format!("invalid renderer response: {msg}"),
        ),
        RenderError::SpawnSubprocess(msg) | RenderError::ResolveCurrentExe(msg) => {
            ProcessJobError::new("RENDER_SUBPROCESS_FAILED", msg)
        }
        RenderError::InvalidRenderEngine(msg) => ProcessJobError::new("RENDER_ENGINE_INVALID", msg),
        RenderError::TypstCompileFailed { kind, details } => ProcessJobError::new(
            classify_render_typst_kind_code(kind),
            format!("typst compile failed ({kind}): {details}"),
        ),
        RenderError::TypstPdfFailed(msg) => ProcessJobError::new("RENDER_PDF_FAILED", msg),
        RenderError::PdfMetadata(msg) => ProcessJobError::new("RENDER_PDF_METADATA_FAILED", msg),
        other => ProcessJobError::new("RENDER_FAILED", other.to_string()),
    }
}

fn classify_render_subprocess_error_code(stderr: &str) -> &'static str {
    let lower = stderr.to_lowercase();

    if lower.contains("typst compile failed (template_syntax)") {
        return "RENDER_TEMPLATE_SYNTAX";
    }
    if lower.contains("typst compile failed (template_semantic)") {
        return "RENDER_TEMPLATE_SEMANTIC";
    }
    if lower.contains("typst compile failed (resource_missing)") {
        return "RENDER_RESOURCE_MISSING";
    }
    if lower.contains("typst compile failed (compile_failed)") {
        return "RENDER_COMPILE_FAILED";
    }
    if lower.contains("typst pdf generation failed") {
        return "RENDER_PDF_FAILED";
    }
    if lower.contains("renderer output artifact not found") {
        return "RENDER_ARTIFACT_MISSING";
    }

    "RENDER_SUBPROCESS_FAILED"
}

fn classify_render_typst_kind_code(kind: &str) -> &'static str {
    match kind {
        "template_syntax" => "RENDER_TEMPLATE_SYNTAX",
        "template_semantic" => "RENDER_TEMPLATE_SEMANTIC",
        "resource_missing" => "RENDER_RESOURCE_MISSING",
        _ => "RENDER_COMPILE_FAILED",
    }
}
