use std::{
    io::{BufReader, Cursor, Write},
    path::{Path, PathBuf},
};

use exif::{In, Reader as ExifReader, Tag};
use flate2::{write::ZlibEncoder, Compression};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use lopdf::{content::Content, content::Operation, Dictionary, Document, Object, Stream};
use tracing::warn;

use super::super::{
    models::{ApiResult, ProcessJobError},
    try_insert_job_event, ApiError, JobRecord, JOB_KIND_DIRECT_FILE,
};
use crate::{
    printer::{OrientationRequested, PrintOptions},
    renderer::RenderResult,
};

const POINTS_PER_MM: f64 = 72.0 / 25.4;
const DIRECT_IMAGE_MARGIN_MM: f64 = 10.0;
const DIRECT_IMAGE_DEFAULT_DPI: f64 = 96.0;
const DIRECT_IMAGE_MAX_EDGE_PX: u32 = 3000;

pub(crate) fn build_direct_file_render_result(
    job: &JobRecord,
    print_options: &PrintOptions,
) -> Result<RenderResult, ProcessJobError> {
    let source_path = job
        .source_file_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ProcessJobError::new(
                "DIRECT_SOURCE_MISSING",
                "direct job missing source_file_path",
            )
        })?;
    let source = PathBuf::from(source_path);
    if !source.exists() {
        return Err(ProcessJobError::new(
            "DIRECT_SOURCE_NOT_FOUND",
            format!("direct source file not found: {}", source.display()),
        ));
    }

    let normalized_content_type = job
        .source_content_type
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if normalized_content_type.starts_with("image/") {
        return build_direct_image_render_result(
            &job.id,
            source.as_path(),
            &normalized_content_type,
            print_options,
        );
    }

    Ok(RenderResult {
        artifact_path: source.to_string_lossy().to_string(),
        output_kind: detect_direct_output_kind(
            source.as_path(),
            job.source_content_type.as_deref(),
        ),
        page_count: 0,
        page_width_pt: None,
        page_height_pt: None,
    })
}

pub(crate) fn sanitize_source_file_name(input: &str) -> String {
    let mut output = String::new();
    for ch in input.trim().chars() {
        let mapped = match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            value if value.is_control() => '_',
            value => value,
        };
        output.push(mapped);
        if output.len() >= 180 {
            break;
        }
    }

    if output.trim().is_empty() {
        "document.bin".to_string()
    } else {
        output
    }
}

pub(crate) fn stage_direct_job_source(
    job_id: &str,
    file_name: &str,
    content: &[u8],
) -> ApiResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join("deepprint-studio")
        .join("direct-jobs")
        .join(job_id);
    std::fs::create_dir_all(&dir).map_err(|err| {
        ApiError::Internal(format!(
            "unable to create direct job staging dir {}: {err}",
            dir.display()
        ))
    })?;

    let path = dir.join(file_name);
    std::fs::write(&path, content).map_err(|err| {
        ApiError::Internal(format!(
            "unable to write direct job source file {}: {err}",
            path.display()
        ))
    })?;

    Ok(path)
}

pub(crate) fn cleanup_direct_job_source(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => warn!(
            "failed to remove direct job staged file {}: {err}",
            path.display()
        ),
    }

    if let Some(parent) = path.parent() {
        match std::fs::remove_dir(parent) {
            Ok(_) => {}
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::NotFound
                        | std::io::ErrorKind::DirectoryNotEmpty
                        | std::io::ErrorKind::PermissionDenied
                ) => {}
            Err(err) => warn!(
                "failed to remove direct job staged dir {}: {err}",
                parent.display()
            ),
        }
    }
}

fn build_direct_image_render_result(
    job_id: &str,
    source_path: &Path,
    _content_type: &str,
    print_options: &PrintOptions,
) -> Result<RenderResult, ProcessJobError> {
    let source_bytes = std::fs::read(source_path).map_err(|err| {
        ProcessJobError::new(
            "DIRECT_SOURCE_READ_FAILED",
            format!(
                "unable to read direct image source {}: {err}",
                source_path.display()
            ),
        )
    })?;

    let image = image::load_from_memory(&source_bytes).map_err(|err| {
        ProcessJobError::new(
            "DIRECT_IMAGE_DECODE_FAILED",
            format!(
                "unable to decode direct image source {}: {err}",
                source_path.display()
            ),
        )
    })?;
    let image = downscale_direct_image_if_needed(apply_exif_orientation(image, &source_bytes));
    let (width_px, height_px) = image.dimensions();
    if width_px == 0 || height_px == 0 {
        return Err(ProcessJobError::new(
            "DIRECT_IMAGE_INVALID_SIZE",
            format!("direct image has invalid size {}x{}", width_px, height_px),
        ));
    }

    let rgb_bytes = image_to_rgb_on_white(&image);
    let mut encoded_bytes = Vec::new();
    {
        let mut encoder = ZlibEncoder::new(&mut encoded_bytes, Compression::default());
        encoder.write_all(&rgb_bytes).map_err(|err| {
            ProcessJobError::new(
                "DIRECT_IMAGE_PDF_COMPRESS_FAILED",
                format!("unable to compress direct image RGB data: {err}"),
            )
        })?;
        encoder.finish().map_err(|err| {
            ProcessJobError::new(
                "DIRECT_IMAGE_PDF_COMPRESS_FAILED",
                format!("unable to finalize direct image RGB compression: {err}"),
            )
        })?;
    }
    let image_stream = Stream::new(
        Dictionary::from_iter(vec![
            ("Type", Object::Name(b"XObject".to_vec())),
            ("Subtype", Object::Name(b"Image".to_vec())),
            ("Width", Object::Integer(i64::from(width_px))),
            ("Height", Object::Integer(i64::from(height_px))),
            ("ColorSpace", Object::Name(b"DeviceRGB".to_vec())),
            ("BitsPerComponent", Object::Integer(8)),
            ("Filter", Object::Name(b"FlateDecode".to_vec())),
            ("Length", Object::Integer(encoded_bytes.len() as i64)),
        ]),
        encoded_bytes,
    );

    let layout = build_direct_image_layout(width_px, height_px, print_options);
    let content = Content {
        operations: build_direct_image_pdf_operations(&layout),
    };
    let content_stream = Stream::new(
        Dictionary::new(),
        content.encode().map_err(|err| {
            ProcessJobError::new(
                "DIRECT_IMAGE_PDF_ENCODE_FAILED",
                format!("unable to encode direct image PDF content: {err}"),
            )
        })?,
    );

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let page_id = document.new_object_id();
    let image_id = document.add_object(image_stream);
    let content_id = document.add_object(content_stream);

    let resources = Dictionary::from_iter(vec![(
        "XObject",
        Dictionary::from_iter(vec![("Im0", Object::Reference(image_id))]).into(),
    )]);
    let page = Dictionary::from_iter(vec![
        ("Type", Object::Name(b"Page".to_vec())),
        ("Parent", Object::Reference(pages_id)),
        (
            "MediaBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(layout.page_width_pt as f32),
                Object::Real(layout.page_height_pt as f32),
            ]),
        ),
        ("Resources", resources.into()),
        ("Contents", Object::Reference(content_id)),
    ]);
    let pages = Dictionary::from_iter(vec![
        ("Type", Object::Name(b"Pages".to_vec())),
        ("Count", Object::Integer(1)),
        ("Kids", Object::Array(vec![Object::Reference(page_id)])),
    ]);
    let catalog_id = document.add_object(Dictionary::from_iter(vec![
        ("Type", Object::Name(b"Catalog".to_vec())),
        ("Pages", Object::Reference(pages_id)),
    ]));
    document.objects.insert(page_id, Object::Dictionary(page));
    document.objects.insert(pages_id, Object::Dictionary(pages));
    document.trailer.set("Root", Object::Reference(catalog_id));
    document.compress();

    let artifact_dir = std::env::temp_dir()
        .join("deepprint-studio")
        .join("artifacts");
    std::fs::create_dir_all(&artifact_dir).map_err(|err| {
        ProcessJobError::new(
            "DIRECT_IMAGE_ARTIFACT_DIR_FAILED",
            format!(
                "unable to create direct image artifact dir {}: {err}",
                artifact_dir.display()
            ),
        )
    })?;
    let artifact_path = artifact_dir.join(format!("{job_id}.pdf"));
    document.save(&artifact_path).map_err(|err| {
        ProcessJobError::new(
            "DIRECT_IMAGE_PDF_SAVE_FAILED",
            format!(
                "unable to save direct image PDF {}: {err}",
                artifact_path.display()
            ),
        )
    })?;

    Ok(RenderResult {
        artifact_path: artifact_path.to_string_lossy().to_string(),
        output_kind: "direct_pdf".to_string(),
        page_count: 1,
        page_width_pt: Some(layout.page_width_pt),
        page_height_pt: Some(layout.page_height_pt),
    })
}

#[derive(Debug, Clone, Copy)]
struct DirectImageLayout {
    page_width_pt: f64,
    page_height_pt: f64,
    image_x_pt: f64,
    image_y_pt: f64,
    image_width_pt: f64,
    image_height_pt: f64,
}

fn build_direct_image_layout(
    image_width_px: u32,
    image_height_px: u32,
    print_options: &PrintOptions,
) -> DirectImageLayout {
    let (page_width_pt, page_height_pt) = oriented_page_size_pt(
        media_dimensions_pt(print_options.media.as_deref()),
        print_options.orientation_requested,
    );
    let margin_pt = DIRECT_IMAGE_MARGIN_MM * POINTS_PER_MM;
    let clip_x_pt = margin_pt.min(page_width_pt / 2.0);
    let clip_y_pt = margin_pt.min(page_height_pt / 2.0);
    let clip_width_pt = (page_width_pt - 2.0 * clip_x_pt).max(1.0);
    let clip_height_pt = (page_height_pt - 2.0 * clip_y_pt).max(1.0);

    let source_width_pt = f64::from(image_width_px) * 72.0 / DIRECT_IMAGE_DEFAULT_DPI;
    let source_height_pt = f64::from(image_height_px) * 72.0 / DIRECT_IMAGE_DEFAULT_DPI;
    let scale = (clip_width_pt / source_width_pt).min(clip_height_pt / source_height_pt);
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let image_width_pt = source_width_pt * scale;
    let image_height_pt = source_height_pt * scale;
    let image_x_pt = clip_x_pt + (clip_width_pt - image_width_pt) / 2.0;
    let image_y_pt = clip_y_pt + (clip_height_pt - image_height_pt) / 2.0;

    DirectImageLayout {
        page_width_pt,
        page_height_pt,
        image_x_pt,
        image_y_pt,
        image_width_pt,
        image_height_pt,
    }
}

fn build_direct_image_pdf_operations(layout: &DirectImageLayout) -> Vec<Operation> {
    vec![
        Operation::new("q", vec![]),
        Operation::new(
            "cm",
            vec![
                Object::Real(layout.image_width_pt as f32),
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(layout.image_height_pt as f32),
                Object::Real(layout.image_x_pt as f32),
                Object::Real(layout.image_y_pt as f32),
            ],
        ),
        Operation::new("Do", vec![Object::Name(b"Im0".to_vec())]),
        Operation::new("Q", vec![]),
    ]
}

fn media_dimensions_pt(media: Option<&str>) -> (f64, f64) {
    match media.map(str::trim).filter(|value| !value.is_empty()) {
        Some("iso_a1_594x841mm") => mm_to_pt(594.0, 841.0),
        Some("iso_a2_420x594mm") => mm_to_pt(420.0, 594.0),
        Some("iso_a3_297x420mm") => mm_to_pt(297.0, 420.0),
        Some("iso_a5_148x210mm") => mm_to_pt(148.0, 210.0),
        Some("na_letter_8.5x11in") => inches_to_pt(8.5, 11.0),
        Some("na_legal_8.5x14in") => inches_to_pt(8.5, 14.0),
        Some("iso_a4_210x297mm") | _ => mm_to_pt(210.0, 297.0),
    }
}

fn oriented_page_size_pt(
    (width_pt, height_pt): (f64, f64),
    orientation: Option<OrientationRequested>,
) -> (f64, f64) {
    match orientation {
        Some(OrientationRequested::Landscape) => (width_pt.max(height_pt), width_pt.min(height_pt)),
        Some(OrientationRequested::Portrait) | None => {
            (width_pt.min(height_pt), width_pt.max(height_pt))
        }
    }
}

fn mm_to_pt(width_mm: f64, height_mm: f64) -> (f64, f64) {
    (width_mm * POINTS_PER_MM, height_mm * POINTS_PER_MM)
}

fn inches_to_pt(width_in: f64, height_in: f64) -> (f64, f64) {
    (width_in * 72.0, height_in * 72.0)
}

fn image_to_rgb_on_white(image: &DynamicImage) -> Vec<u8> {
    let rgba = image.to_rgba8();
    let mut output = Vec::with_capacity(rgba.len() / 4 * 3);
    for pixel in rgba.pixels() {
        let [red, green, blue, alpha] = pixel.0;
        if alpha == 255 {
            output.extend_from_slice(&[red, green, blue]);
            continue;
        }

        let alpha = u16::from(alpha);
        let inv_alpha = 255 - alpha;
        output.push(composite_channel_on_white(red, alpha, inv_alpha));
        output.push(composite_channel_on_white(green, alpha, inv_alpha));
        output.push(composite_channel_on_white(blue, alpha, inv_alpha));
    }
    output
}

fn composite_channel_on_white(channel: u8, alpha: u16, inv_alpha: u16) -> u8 {
    let value = u16::from(channel) * alpha + 255 * inv_alpha + 127;
    (value / 255) as u8
}

fn apply_exif_orientation(image: DynamicImage, source_bytes: &[u8]) -> DynamicImage {
    match read_exif_orientation(source_bytes) {
        Some(3) => image.rotate180(),
        Some(6) => image.rotate90(),
        Some(8) => image.rotate270(),
        _ => image,
    }
}

fn downscale_direct_image_if_needed(image: DynamicImage) -> DynamicImage {
    let (width_px, height_px) = image.dimensions();
    let long_edge = width_px.max(height_px);
    if long_edge <= DIRECT_IMAGE_MAX_EDGE_PX {
        return image;
    }

    let scale = f64::from(DIRECT_IMAGE_MAX_EDGE_PX) / f64::from(long_edge);
    let target_width = ((f64::from(width_px) * scale).round() as u32).max(1);
    let target_height = ((f64::from(height_px) * scale).round() as u32).max(1);
    image.resize_exact(target_width, target_height, FilterType::CatmullRom)
}

fn read_exif_orientation(source_bytes: &[u8]) -> Option<u32> {
    let cursor = Cursor::new(source_bytes);
    let exif = ExifReader::new()
        .read_from_container(&mut BufReader::new(cursor))
        .ok()?;
    exif.get_field(Tag::Orientation, In::PRIMARY)?
        .value
        .get_uint(0)
}

pub(crate) fn cleanup_direct_source_for_terminal_job(
    db_path: &Path,
    job: &JobRecord,
    reason: &str,
) {
    if job.job_kind != JOB_KIND_DIRECT_FILE {
        return;
    }
    let Some(source_path) = job
        .source_file_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };

    cleanup_direct_job_source(Path::new(source_path));
    let _ = try_insert_job_event(
        db_path,
        &job.id,
        "direct_source_cleanup",
        None,
        None,
        reason,
    );
}

fn detect_direct_output_kind(path: &Path, content_type: Option<&str>) -> String {
    let normalized_content_type = content_type
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if normalized_content_type == "application/pdf" {
        return "direct_pdf".to_string();
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if extension == "pdf" {
        return "direct_pdf".to_string();
    }

    "direct_file".to_string()
}
