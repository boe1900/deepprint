use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};

use super::super::{
    submission::map_print_backend_error, utils::now_unix, AddPrinterRequest, AddPrinterResponse,
    AgentState, ApiResult, CreatePrinterRecord, PrinterSummary, PrinterTargetInput,
};
use super::validation::validate_printer_target;
use crate::storage::{create_printer_record, load_printer_by_normalized_uri};

pub(crate) async fn create_printer(
    State(state): State<Arc<AgentState>>,
    Json(payload): Json<AddPrinterRequest>,
) -> ApiResult<(StatusCode, Json<AddPrinterResponse>)> {
    let validated = validate_printer_target(
        &state,
        &PrinterTargetInput::Uri {
            uri: payload.printer_uri.clone(),
        },
    )
    .await?;

    let display_name = payload.display_name.trim();
    let effective_name = if display_name.is_empty() {
        validated.discovered_name.clone()
    } else {
        display_name.to_string()
    };

    if let Some(existing) =
        load_printer_by_normalized_uri(state.db_path.as_ref(), &validated.normalized_uri)
            .map_err(map_print_backend_error)?
    {
        return Ok((
            StatusCode::OK,
            Json(AddPrinterResponse {
                printer: PrinterSummary {
                    id: existing.id,
                    name: existing.name,
                    uri: existing.uri,
                    source: existing.source,
                    is_default: existing.is_default,
                    enabled: existing.enabled,
                    state: existing.state,
                    state_message: existing.state_message,
                    last_validated_at: existing.last_validated_at,
                    last_seen_at: existing.last_seen_at,
                },
                created: false,
            }),
        ));
    }

    let now = now_unix();
    let created = create_printer_record(
        state.db_path.as_ref(),
        &CreatePrinterRecord {
            id: uuid::Uuid::new_v4().to_string(),
            source: payload.source,
            display_name: effective_name,
            printer_uri: validated.printer_uri,
            normalized_uri: validated.normalized_uri,
            last_known_state: validated.state,
            last_state_message: validated.state_message,
            capabilities: validated.capabilities,
            attributes: validated.attributes,
            last_seen_at: Some(now),
            last_validated_at: Some(now),
            last_refreshed_at: Some(now),
        },
        now,
    )
    .map_err(map_print_backend_error)?;

    Ok((
        StatusCode::CREATED,
        Json(AddPrinterResponse {
            printer: created,
            created: true,
        }),
    ))
}
