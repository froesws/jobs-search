use tauri::State;

use crate::analytics::{
    AnalyticsPayload, AnalyticsRequest, ExportRequest, ExportResult, get_analytics,
};
use crate::error::ErrorPayload;
use crate::state::AppState;

#[tauri::command]
pub fn cmd_get_analytics(
    state: State<'_, AppState>,
    request: AnalyticsRequest,
) -> Result<AnalyticsPayload, ErrorPayload> {
    get_analytics(&state.db, request).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_export_analytics_csv(
    state: State<'_, AppState>,
    request: ExportRequest,
) -> Result<ExportResult, ErrorPayload> {
    crate::analytics::export_csv(&state.db, request).map_err(Into::into)
}
