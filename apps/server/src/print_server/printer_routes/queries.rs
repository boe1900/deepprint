use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};

use super::super::{
    submission::map_print_backend_error, AgentState, ApiError, ApiResult, PrinterDetail,
    PrinterSummary, PrintersListResponse,
};
use crate::storage::{list_printer_summaries, load_printer_detail_by_id};

pub(crate) async fn list_printers(
    State(state): State<Arc<AgentState>>,
) -> ApiResult<Json<PrintersListResponse<PrinterSummary>>> {
    let printers =
        list_printer_summaries(state.db_path.as_ref()).map_err(map_print_backend_error)?;
    Ok(Json(PrintersListResponse { printers }))
}

pub(crate) async fn get_printer_detail(
    State(state): State<Arc<AgentState>>,
    AxumPath(printer_id): AxumPath<String>,
) -> ApiResult<Json<PrinterDetail>> {
    let record = load_printer_detail_by_id(state.db_path.as_ref(), &printer_id)
        .map_err(map_print_backend_error)?
        .ok_or_else(|| ApiError::NotFound(format!("printer not found: {printer_id}")))?;

    Ok(Json(record))
}
