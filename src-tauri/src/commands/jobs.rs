use tauri::State;

use crate::db::applications::{self, JobApplication, JobEvaluationView};
use crate::db::jobs::{JobDetail, PaginatedJobs};
use crate::db::runs::ExecutionAudit;
use crate::error::ErrorPayload;
use crate::state::AppState;
use crate::types::{JobId, JobTab, RunId};

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
    crate::db::jobs::get_detail(&state.db, &JobId::from(job_id)).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_apply_job(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<JobApplication, ErrorPayload> {
    applications::apply_job(&state.db, &JobId::from(job_id)).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_ignore_job(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<JobApplication, ErrorPayload> {
    applications::ignore_job(&state.db, &JobId::from(job_id)).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_restore_job(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<JobApplication, ErrorPayload> {
    applications::restore_job(&state.db, &JobId::from(job_id)).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_approve_job(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<JobEvaluationView, ErrorPayload> {
    applications::approve_job(&state.db, &JobId::from(job_id)).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_get_audit(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<ExecutionAudit, ErrorPayload> {
    crate::db::runs::get_audit(&state.db, &RunId::from(run_id)).map_err(Into::into)
}
