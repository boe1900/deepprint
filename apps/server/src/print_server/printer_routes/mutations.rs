use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};

use super::super::{
    submission::map_print_backend_error, utils::now_unix, AgentState, ApiError, ApiResult,
    DeletePrinterResponse, PrinterDetail,
};
use crate::storage::{
    count_inflight_jobs_for_printer_at_path, delete_printer_record, disable_printer_record,
    enable_printer_record, set_default_printer_record,
};

pub(crate) async fn enable_printer(
    State(state): State<Arc<AgentState>>,
    AxumPath(printer_id): AxumPath<String>,
) -> ApiResult<Json<PrinterDetail>> {
    let updated = enable_printer_record(state.db_path.as_ref(), &printer_id, now_unix())
        .map_err(map_print_backend_error)?
        .ok_or_else(|| ApiError::NotFound(format!("printer not found: {printer_id}")))?;

    Ok(Json(updated))
}

pub(crate) async fn disable_printer(
    State(state): State<Arc<AgentState>>,
    AxumPath(printer_id): AxumPath<String>,
) -> ApiResult<Json<PrinterDetail>> {
    let updated = disable_printer_record(state.db_path.as_ref(), &printer_id, now_unix())
        .map_err(map_print_backend_error)?
        .ok_or_else(|| ApiError::NotFound(format!("printer not found: {printer_id}")))?;

    Ok(Json(updated))
}

pub(crate) async fn set_default_printer(
    State(state): State<Arc<AgentState>>,
    AxumPath(printer_id): AxumPath<String>,
) -> ApiResult<Json<PrinterDetail>> {
    let updated = set_default_printer_record(state.db_path.as_ref(), &printer_id, now_unix())
        .map_err(map_print_backend_error)?
        .ok_or_else(|| ApiError::NotFound(format!("printer not found: {printer_id}")))?;

    Ok(Json(updated))
}

pub(crate) async fn delete_printer(
    State(state): State<Arc<AgentState>>,
    AxumPath(printer_id): AxumPath<String>,
) -> ApiResult<Json<DeletePrinterResponse>> {
    let inflight_count =
        count_inflight_jobs_for_printer_at_path(state.db_path.as_ref(), &printer_id)?;
    if inflight_count > 0 {
        return Err(ApiError::Conflict(format!(
            "printer has {inflight_count} inflight job(s): {printer_id}"
        )));
    }

    let deleted = delete_printer_record(state.db_path.as_ref(), &printer_id, now_unix())
        .map_err(map_print_backend_error)?;
    if !deleted {
        return Err(ApiError::NotFound(format!(
            "printer not found: {printer_id}"
        )));
    }

    Ok(Json(DeletePrinterResponse { deleted: true }))
}
