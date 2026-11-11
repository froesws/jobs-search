//! Job feed, lifecycle, and audit commands.
#![allow(missing_docs)]

use tauri::State;

use crate::db::applications::{self, JobApplication, JobEvaluationView};
use crate::db::jobs::{JobDetail, PaginatedJobs, PaginatedRunJobs};
use crate::db::runs::ExecutionAudit;
use crate::error::{AppError, ErrorPayload};
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
pub fn cmd_get_run_jobs(
    state: State<'_, AppState>,
    run_id: String,
    limit: u32,
    offset: u32,
) -> Result<PaginatedRunJobs, ErrorPayload> {
    crate::db::jobs::list_run_jobs(&state.db, &run_id, limit, offset).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_export_run_jobs_csv(
    state: State<'_, AppState>,
    run_id: String,
    destination: String,
) -> Result<crate::analytics::ExportResult, ErrorPayload> {
    export_run_jobs(&state, &run_id, &destination).map_err(Into::into)
}

fn export_run_jobs(
    state: &AppState,
    run_id: &str,
    destination: &str,
) -> Result<crate::analytics::ExportResult, AppError> {
    let path = std::path::Path::new(destination);
    if path.extension().and_then(|value| value.to_str()) != Some("csv") {
        return Err(AppError::validation(
            "the export destination must be a .csv file",
        ));
    }
    if path.is_dir() {
        return Err(AppError::validation(
            "the export destination is a directory",
        ));
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.is_dir()
    {
        return Err(AppError::validation("the export directory does not exist"));
    }

    let rows = crate::db::jobs::run_job_rows(&state.db, run_id)?;
    let mut contents = String::from(
        "job_id,title,company,location,work_modality,url,evaluation_status,match_score,rejection_reason,provider,model\n",
    );
    for row in &rows {
        let values = [
            row.job_id.to_string(),
            row.title.clone(),
            row.company.clone(),
            row.location.clone(),
            row.work_modality.clone(),
            row.url.clone(),
            row.evaluation_status.clone().unwrap_or_default(),
            row.match_score
                .map(|value| value.to_string())
                .unwrap_or_default(),
            row.rejection_reason.clone().unwrap_or_default(),
            row.provider.clone().unwrap_or_default(),
            row.model.clone().unwrap_or_default(),
        ];
        let line = values
            .iter()
            .map(|value| escape_csv(value))
            .collect::<Vec<_>>()
            .join(",");
        contents.push_str(&line);
        contents.push('\n');
    }
    std::fs::write(path, contents.as_bytes())?;
    Ok(crate::analytics::ExportResult {
        path: destination.to_string(),
        rows: rows.len() as u32,
        bytes: contents.len() as u64,
    })
}

fn escape_csv(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::db::evaluations::{EvaluationRecord, upsert_evaluation};
    use crate::db::jobs::{NewJob, upsert_job};
    use crate::db::runs::start_run;
    use crate::types::{EvaluationStatus, HardFilterRejection, WorkModality};

    fn state() -> (AppState, tempfile::TempDir) {
        let directory = tempfile::tempdir().expect("tempdir");
        let database = directory
            .path()
            .join("jobs-search.sqlite3")
            .display()
            .to_string();
        let map = std::collections::HashMap::from([("JOBS_SEARCH_DB_PATH".to_string(), database)]);
        let config = AppConfig::from_lookup(
            move |key| map.get(key).cloned(),
            directory.path().to_path_buf(),
            directory.path(),
        )
        .expect("config");
        let state = AppState::new(config).expect("state");
        (state, directory)
    }

    #[test]
    fn exports_the_run_jobs_as_csv() {
        let (state, directory) = state();
        let run = start_run(&state.db, "{}").expect("run");
        let job = upsert_job(
            &state.db,
            &NewJob {
                canonical_key: "acme|rust|remote".to_string(),
                canonical_hash: "hash-1".to_string(),
                title: "Rust, \"Senior\"".to_string(),
                company: "Acme".to_string(),
                location: "Remote - Brazil".to_string(),
                work_modality: WorkModality::Remote,
                url: "https://example.com/job".to_string(),
                posted_at: None,
                description_clean: "Rust".to_string(),
                extracted_metadata_json: None,
                needs_review: false,
            },
        )
        .expect("job")
        .job_id;
        upsert_evaluation(
            &state.db,
            &EvaluationRecord {
                job_id: job,
                run_id: run.run_id.clone(),
                profile_version: 1,
                status: EvaluationStatus::Rejected,
                score: None,
                matching_items: None,
                total_requirements: None,
                breakdown: None,
                matched_keywords: vec![],
                missing_keywords: vec![],
                rejections: vec![HardFilterRejection::new(
                    "MODALITY_NOT_ELIGIBLE",
                    "wrong city",
                )],
                provider: None,
                model: None,
                threshold: 50,
            },
        )
        .expect("evaluation");

        let destination = directory.path().join("run.csv");
        let result = export_run_jobs(
            &state,
            run.run_id.as_ref(),
            destination.to_str().expect("path"),
        )
        .expect("export");

        assert_eq!(result.rows, 1);
        let contents = std::fs::read_to_string(&destination).expect("read");
        assert!(contents.starts_with("job_id,title,company"));
        assert!(contents.contains("\"Rust, \"\"Senior\"\"\""));
        assert!(contents.contains("wrong city"));
        assert!(contents.contains("REJECTED"));
    }

    #[test]
    fn rejects_a_non_csv_destination() {
        let (state, directory) = state();
        let path = directory.path().join("run.txt");
        let error = export_run_jobs(&state, "run-1", path.to_str().expect("path"))
            .expect_err("invalid extension");
        assert!(error.message().contains(".csv"));
    }

    #[test]
    fn escapes_csv_values() {
        assert_eq!(escape_csv("plain"), "plain");
        assert_eq!(escape_csv("a,b"), "\"a,b\"");
        assert_eq!(escape_csv("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(escape_csv("two\nlines"), "\"two\nlines\"");
    }
}
