use std::{fs::File, io::Write as IoWrite};

use serde::Serialize;
use zip::{write::SimpleFileOptions, ZipWriter};

use super::super::models::ApiError;

pub(crate) fn sanitize_zip_entry_name(file_name: &str) -> String {
    file_name.replace(['/', '\\', ':'], "_").replace("..", "_")
}

pub(crate) fn write_zip_json<T: Serialize>(
    zip: &mut ZipWriter<File>,
    entry_name: &str,
    value: &T,
    options: SimpleFileOptions,
) -> Result<(), ApiError> {
    zip.start_file(entry_name, options)
        .map_err(|err| ApiError::Internal(format!("start zip entry {entry_name} failed: {err}")))?;
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|err| ApiError::Internal(format!("serialize {entry_name} failed: {err}")))?;
    zip.write_all(&payload)
        .map_err(|err| ApiError::Internal(format!("write zip entry {entry_name} failed: {err}")))?;
    Ok(())
}

pub(crate) fn write_zip_text(
    zip: &mut ZipWriter<File>,
    entry_name: &str,
    content: &str,
    options: SimpleFileOptions,
) -> Result<(), ApiError> {
    zip.start_file(entry_name, options)
        .map_err(|err| ApiError::Internal(format!("start zip entry {entry_name} failed: {err}")))?;
    zip.write_all(content.as_bytes())
        .map_err(|err| ApiError::Internal(format!("write zip entry {entry_name} failed: {err}")))?;
    Ok(())
}
