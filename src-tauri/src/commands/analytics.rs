//! Analytics payload and CSV export commands.
#![allow(missing_docs)]

use tauri::State;

use crate::analytics::{
    AnalyticsPayload, AnalyticsRequest, ExportRequest, ExportResult, get_analytics,
};
use crate::error::{AppError, ErrorPayload};
use crate::state::AppState;

#[tauri::command]
pub async fn cmd_get_analytics(
    state: State<'_, AppState>,
    request: AnalyticsRequest,
) -> Result<AnalyticsPayload, ErrorPayload> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || get_analytics(&db, request))
        .await
        .map_err(|_| AppError::Database("analytics task failed".to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
pub async fn cmd_export_analytics_csv(
    state: State<'_, AppState>,
    request: ExportRequest,
) -> Result<ExportResult, ErrorPayload> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || crate::analytics::export_csv(&db, request))
        .await
        .map_err(|_| AppError::Database("analytics export task failed".to_string()))?
        .map_err(Into::into)
}
