use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};

use super::super::{
    ipp_backend, mock_print_backend, submission::map_print_backend_error, utils::now_unix,
    AgentState, ApiError, ApiResult, PrinterDetail, PrinterTargetInput,
    RefreshPrinterSnapshotInput, ValidatePrinterRequest, ValidatedPrinterTarget,
};
use crate::storage::{
    load_printer_by_normalized_uri, load_printer_detail_by_id, refresh_printer_snapshot,
};

pub(crate) async fn validate_printer_target(
    state: &AgentState,
    target: &PrinterTargetInput,
) -> ApiResult<ValidatedPrinterTarget> {
    let mut validated = if state.config.mock_mode {
        if let Some(mock) = mock_print_backend::validate_mock_target(target).await {
            mock
        } else {
            ipp_backend::validate_printer_target(target)
                .await
                .map_err(map_print_backend_error)?
        }
    } else {
        ipp_backend::validate_printer_target(target)
            .await
            .map_err(map_print_backend_error)?
    };

    if let Some(existing) =
        load_printer_by_normalized_uri(state.db_path.as_ref(), &validated.normalized_uri)
            .map_err(map_print_backend_error)?
    {
        validated.already_managed = true;
        validated.managed_printer_id = Some(existing.id);
    }

    Ok(validated)
}

pub(crate) async fn validate_printer(
    State(state): State<Arc<AgentState>>,
    Json(payload): Json<ValidatePrinterRequest>,
) -> ApiResult<Json<ValidatedPrinterTarget>> {
    Ok(Json(
        validate_printer_target(&state, &payload.target).await?,
    ))
}

pub(crate) async fn refresh_printer(
    State(state): State<Arc<AgentState>>,
    AxumPath(printer_id): AxumPath<String>,
) -> ApiResult<Json<PrinterDetail>> {
    let existing = load_printer_detail_by_id(state.db_path.as_ref(), &printer_id)
        .map_err(map_print_backend_error)?
        .ok_or_else(|| ApiError::NotFound(format!("printer not found: {printer_id}")))?;

    let refreshed = if state.config.mock_mode {
        if let Some(detail) = mock_print_backend::get_mock_printer_detail(&existing.uri).await {
            detail
        } else {
            ipp_backend::get_printer_detail(&existing.uri)
                .await
                .map_err(map_print_backend_error)?
        }
    } else {
        ipp_backend::get_printer_detail(&existing.uri)
            .await
            .map_err(map_print_backend_error)?
    };

    let now = now_unix();
    let updated = refresh_printer_snapshot(
        state.db_path.as_ref(),
        &printer_id,
        &RefreshPrinterSnapshotInput {
            printer_uri: refreshed.uri.clone(),
            normalized_uri: refreshed.normalized_uri.clone(),
            last_known_state: refreshed.state.clone(),
            last_state_message: refreshed.state_message.clone(),
            capabilities: refreshed.capabilities.clone(),
            attributes: refreshed.attributes.clone(),
            last_seen_at: Some(now),
            last_validated_at: Some(now),
            last_refreshed_at: Some(now),
        },
        now,
    )
    .map_err(map_print_backend_error)?
    .ok_or_else(|| ApiError::NotFound(format!("printer not found: {printer_id}")))?;

    Ok(Json(updated))
}
