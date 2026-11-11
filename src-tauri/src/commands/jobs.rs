use tauri::State;

use crate::db::jobs::{JobDetail, PaginatedJobs};
use crate::error::ErrorPayload;
use crate::state::AppState;
use crate::types::JobTab;

#[tauri::command]
pub fn cmd_get_jobs(
    state: State<'_, AppState>,
    tab: JobTab,
    limit: u32,
    offset: u32,
) -> Result<PaginatedJobs, ErrorPayload> {
    crate::db::jobs::list_summaries(&state.db, tab, limit, offset).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_get_job(state: State<'_, AppState>, job_id: String) -> Result<JobDetail, ErrorPayload> {
    crate::db::jobs::get_detail(&state.db, &job_id).map_err(Into::into)
}
