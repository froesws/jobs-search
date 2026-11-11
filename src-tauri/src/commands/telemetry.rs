//! Local log health command.
#![allow(missing_docs)]

use tauri::State;

use crate::telemetry::health::{HealthState, LogHealth};

#[tauri::command]
pub fn cmd_get_log_health(state: State<'_, HealthState>) -> LogHealth {
    state.snapshot()
}
