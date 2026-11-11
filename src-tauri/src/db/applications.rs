//! Application lifecycle and append-only status events.
//!
//! Evaluation status and lifecycle state are independent. Applying or ignoring
//! a job updates `job_applications` and appends a `job_status_events` row; it
//! never rewrites history.

use rusqlite::{OptionalExtension, params};
use serde::Serialize;

use crate::db::{Db, now_rfc3339};
use crate::error::AppError;
use crate::types::JobId;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct JobApplication {
    pub job_id: JobId,
    pub lifecycle_state: String,
    pub applied_at: Option<String>,
    pub approval_source: String,
    pub evaluation_snapshot_json: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct JobEvaluationView {
    pub job_id: JobId,
    pub evaluation_status: String,
    pub match_score: Option<i64>,
    pub approval_source: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StatusEvent {
    pub event_type: String,
    pub old_value: Option<String>,
    pub new_value: String,
    pub reason: Option<String>,
    pub created_at: String,
}

struct EvaluationSnapshot {
    status: String,
    score: Option<i64>,
    matching: Option<i64>,
    total: Option<i64>,
    breakdown: Option<String>,
    matched: Option<String>,
    missing: Option<String>,
    rejections: Option<String>,
    rules_version: String,
    threshold: i64,
    provider: Option<String>,
    model: Option<String>,
    approval_source: String,
    evaluated_at: Option<String>,
    stale: bool,
    profile_version: i64,
}

pub fn get_application(db: &Db, job_id: &JobId) -> Result<Option<JobApplication>, AppError> {
    db.with_connection(|connection| {
        Ok(connection
            .query_row(
                "SELECT job_id, lifecycle_state, applied_at, approval_source, evaluation_snapshot_json
                 FROM job_applications WHERE job_id = ?1",
                [job_id.as_str()],
                application_from_row,
            )
            .optional()?)
    })
}

/// Mark an APPROVED job as applied and capture an immutable evaluation snapshot.
pub fn apply_job(db: &Db, job_id: &JobId) -> Result<JobApplication, AppError> {
    let evaluation =
        load_evaluation(db, job_id)?.ok_or_else(|| AppError::not_found("evaluation"))?;
    if evaluation.status != "APPROVED" {
        return Err(AppError::validation("only approved jobs can be applied"));
    }
    if evaluation.stale {
        return Err(AppError::validation(
            "this evaluation is stale; re-run the search before applying",
        ));
    }

    let previous = get_application(db, job_id)?;
    let now = now_rfc3339();
    let application = JobApplication {
        job_id: job_id.clone(),
        lifecycle_state: "APPLIED".to_string(),
        applied_at: Some(now.clone()),
        approval_source: evaluation.approval_source.clone(),
        evaluation_snapshot_json: snapshot_json(job_id, &evaluation),
    };
    persist_application(
        db,
        &application,
        &now,
        "APPLIED",
        Some("operator marked applied"),
        previous.as_ref(),
    )
}

pub fn ignore_job(db: &Db, job_id: &JobId) -> Result<JobApplication, AppError> {
    transition(db, job_id, "IGNORED", "operator ignored the job")
}

pub fn restore_job(db: &Db, job_id: &JobId) -> Result<JobApplication, AppError> {
    transition(db, job_id, "DISCOVERED", "operator restored the job")
}

fn transition(
    db: &Db,
    job_id: &JobId,
    next: &str,
    reason: &str,
) -> Result<JobApplication, AppError> {
    if crate::db::jobs::get_detail(db, job_id).is_err() {
        return Err(AppError::not_found("job"));
    }
    let previous = get_application(db, job_id)?;
    let evaluation = load_evaluation(db, job_id)?;
    let approval_source = evaluation
        .as_ref()
        .map(|value| value.approval_source.clone())
        .unwrap_or_else(|| "automatic".to_string());
    let snapshot = evaluation
        .as_ref()
        .map(|value| snapshot_json(job_id, value))
        .unwrap_or_else(|| "{}".to_string());
    let now = now_rfc3339();
    let applied_at = if next == "APPLIED" {
        Some(now.clone())
    } else {
        previous.as_ref().and_then(|value| value.applied_at.clone())
    };
    let application = JobApplication {
        job_id: job_id.clone(),
        lifecycle_state: next.to_string(),
        applied_at,
        approval_source,
        evaluation_snapshot_json: snapshot,
    };
    persist_application(
        db,
        &application,
        &now,
        event_for_state(next),
        Some(reason),
        previous.as_ref(),
    )
}

/// Upsert `job_applications` and append the lifecycle event atomically.
fn persist_application(
    db: &Db,
    application: &JobApplication,
    now: &str,
    event_type: &str,
    reason: Option<&str>,
    previous: Option<&JobApplication>,
) -> Result<JobApplication, AppError> {
    db.with_connection_mut(|connection| {
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO job_applications
             (job_id, lifecycle_state, applied_at, approval_source, evaluation_snapshot_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(job_id) DO UPDATE SET
                lifecycle_state = excluded.lifecycle_state,
                applied_at = excluded.applied_at,
                approval_source = excluded.approval_source,
                evaluation_snapshot_json = excluded.evaluation_snapshot_json,
                updated_at = excluded.updated_at",
            params![
                application.job_id.as_str(),
                application.lifecycle_state,
                application.applied_at,
                application.approval_source,
                application.evaluation_snapshot_json,
                now,
            ],
        )?;
        append_event_with_connection(
            &transaction,
            &application.job_id,
            event_type,
            previous.map(|value| value.lifecycle_state.as_str()),
            &application.lifecycle_state,
            reason,
            now,
        )?;
        transaction.commit()?;
        Ok(())
    })?;
    Ok(application.clone())
}

/// Manually approve a PENDING evaluation. The original score and breakdown are
/// preserved; only the status and approval source change.
pub fn approve_job(db: &Db, job_id: &JobId) -> Result<JobEvaluationView, AppError> {
    let evaluation =
        load_evaluation(db, job_id)?.ok_or_else(|| AppError::not_found("evaluation"))?;
    if evaluation.score.is_none() {
        return Err(AppError::validation(
            "a job without a score cannot be approved",
        ));
    }
    if evaluation.status == "REJECTED" {
        return Err(AppError::validation(
            "rejected jobs cannot be manually approved",
        ));
    }

    let now = now_rfc3339();
    db.with_connection_mut(|connection| {
        let transaction = connection.transaction()?;
        transaction.execute(
            "UPDATE job_evaluations
             SET evaluation_status = 'APPROVED', approval_source = 'manual'
             WHERE job_id = ?1",
            [job_id.as_str()],
        )?;
        append_event_with_connection(
            &transaction,
            job_id,
            "APPROVED",
            Some(evaluation.status.as_str()),
            "APPROVED",
            Some("manual approval"),
            &now,
        )?;
        transaction.commit()?;
        Ok(())
    })?;

    Ok(JobEvaluationView {
        job_id: job_id.clone(),
        evaluation_status: "APPROVED".to_string(),
        match_score: evaluation.score,
        approval_source: "manual".to_string(),
    })
}

pub fn events(db: &Db, job_id: &JobId) -> Result<Vec<StatusEvent>, AppError> {
    db.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT event_type, old_value, new_value, reason, created_at
             FROM job_status_events WHERE job_id = ?1 ORDER BY created_at ASC, id ASC",
        )?;
        let rows = statement.query_map([job_id.as_str()], |row| {
            Ok(StatusEvent {
                event_type: row.get(0)?,
                old_value: row.get(1)?,
                new_value: row.get(2)?,
                reason: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        let mut values = Vec::new();
        for row in rows {
            values.push(row?);
        }
        Ok(values)
    })
}

fn load_evaluation(db: &Db, job_id: &JobId) -> Result<Option<EvaluationSnapshot>, AppError> {
    db.with_connection(|connection| {
        Ok(connection
            .query_row(
                "SELECT evaluation_status, match_score, matching_items_count,
                        total_requirements_count, score_breakdown_json,
                        matched_keywords_json, missing_keywords_json,
                        hard_filter_rejections_json, rules_version, threshold,
                        provider, model, approval_source, evaluated_at, stale,
                        profile_version
                 FROM job_evaluations WHERE job_id = ?1",
                [job_id.as_str()],
                |row| {
                    Ok(EvaluationSnapshot {
                        status: row.get(0)?,
                        score: row.get(1)?,
                        matching: row.get(2)?,
                        total: row.get(3)?,
                        breakdown: row.get(4)?,
                        matched: row.get(5)?,
                        missing: row.get(6)?,
                        rejections: row.get(7)?,
                        rules_version: row.get(8)?,
                        threshold: row.get(9)?,
                        provider: row.get(10)?,
                        model: row.get(11)?,
                        approval_source: row.get(12)?,
                        evaluated_at: row.get(13)?,
                        stale: row.get(14)?,
                        profile_version: row.get(15)?,
                    })
                },
            )
            .optional()?)
    })
}

fn snapshot_json(job_id: &JobId, evaluation: &EvaluationSnapshot) -> String {
    serde_json::json!({
        "job_id": job_id.as_str(),
        "evaluation_status": evaluation.status,
        "match_score": evaluation.score,
        "matching_items_count": evaluation.matching,
        "total_requirements_count": evaluation.total,
        "score_breakdown": evaluation.breakdown,
        "matched_keywords": evaluation.matched,
        "missing_keywords": evaluation.missing,
        "hard_filter_rejections": evaluation.rejections,
        "rules_version": evaluation.rules_version,
        "threshold": evaluation.threshold,
        "provider": evaluation.provider,
        "model": evaluation.model,
        "approval_source": evaluation.approval_source,
        "evaluated_at": evaluation.evaluated_at,
        "stale": evaluation.stale,
        "profile_version": evaluation.profile_version,
    })
    .to_string()
}

fn event_for_state(state: &str) -> &'static str {
    match state {
        "APPLIED" => "APPLIED",
        "IGNORED" => "IGNORED",
        "DISCOVERED" => "RESTORED",
        _ => "EVALUATED",
    }
}

#[allow(clippy::too_many_arguments)]
fn append_event_with_connection(
    connection: &rusqlite::Connection,
    job_id: &JobId,
    event_type: &str,
    old_value: Option<&str>,
    new_value: &str,
    reason: Option<&str>,
    now: &str,
) -> Result<(), AppError> {
    connection.execute(
        "INSERT INTO job_status_events
         (id, job_id, event_type, old_value, new_value, reason, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            uuid::Uuid::new_v4().to_string(),
            job_id.as_str(),
            event_type,
            old_value,
            new_value,
            sanitize_reason(reason),
            now,
        ],
    )?;
    Ok(())
}

fn sanitize_reason(reason: Option<&str>) -> Option<String> {
    reason.map(|value| {
        value
            .chars()
            .filter(|character| !character.is_control())
            .take(280)
            .collect::<String>()
    })
}

fn application_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobApplication> {
    Ok(JobApplication {
        job_id: JobId::from(row.get::<_, String>(0)?),
        lifecycle_state: row.get(1)?,
        applied_at: row.get(2)?,
        approval_source: row.get(3)?,
        evaluation_snapshot_json: row.get(4)?,
    })
}
