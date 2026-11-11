//! Analytics aggregation for dashboards and CSV export.

pub mod csv;
pub mod funnel;
pub mod skill_gap;

use std::collections::BTreeMap;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::AppError;
use crate::types::{JobId, RunId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
/// Scope for analytics aggregation.
#[non_exhaustive]
pub enum AnalyticsScope {
    /// The latest completed run.
    CurrentRun,
    /// Evaluations from the last 7 days.
    Last7Days,
}

#[derive(Debug, Clone, Deserialize)]
/// Analytics query: scope plus an optional run for CURRENT_RUN.
pub struct AnalyticsRequest {
    /// The selected scope.
    pub scope: AnalyticsScope,
    #[serde(default)]
    /// The run for CURRENT_RUN, when applicable.
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
/// Full analytics response for one scope.
pub struct AnalyticsPayload {
    /// The selected scope.
    pub scope: AnalyticsScope,
    /// The run for CURRENT_RUN, when applicable.
    pub run_id: Option<String>,
    /// Total eligible jobs in scope.
    pub denominator: u32,
    /// Most recent evaluation time (UTC).
    pub last_updated: String,
    /// True when eligible evaluations are stale.
    pub stale: bool,
    /// Ranked missing-skill demand.
    pub skill_gap: Vec<SkillGapMetric>,
    /// Funnel counts, rates, and breakdowns.
    pub funnel: FunnelPayload,
    /// Evaluated scores by bucket.
    pub score_distribution: Vec<ScoreBucket>,
    /// LLM call counts and latency.
    pub llm_metrics: LlmMetricsPayload,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
/// One ranked missing-skill demand entry.
pub struct SkillGapMetric {
    /// Normalized skill key.
    pub skill: String,
    /// Consistent display casing.
    pub display_name: String,
    /// Jobs missing this skill, counted once per job.
    pub demand_count: u32,
    /// Demand count over the denominator, in percent.
    pub demand_percentage: f64,
    /// Position after ranking.
    pub rank: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
/// Funnel counts, rates, and breakdowns for one scope.
pub struct FunnelPayload {
    /// Postings fetched.
    pub ingested: i64,
    /// Duplicate postings merged.
    pub duplicates: i64,
    /// Hard-filter rejections by reason.
    pub rejected_by_reason: BTreeMap<String, i64>,
    /// Scored below the threshold.
    pub pending: i64,
    /// Passed the threshold.
    pub approved: i64,
    /// Evaluations that failed.
    pub error: i64,
    /// Jobs marked applied.
    pub applied: i64,
    /// Jobs ignored by the operator.
    pub ignored: i64,
    /// Approved divided by evaluated, in percent.
    pub approval_rate: f64,
    /// Applied divided by approved, in percent.
    pub application_rate: f64,
    /// Failures reported by sources.
    pub source_failures: Vec<String>,
    /// Counts by source.
    pub source_breakdown: BTreeMap<String, i64>,
    /// Counts by provider/model.
    pub provider_breakdown: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
/// Score range and the count of evaluated jobs in it.
pub struct ScoreBucket {
    /// Score bucket label.
    pub range: String,
    /// Jobs in the bucket.
    pub count: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
/// LLM call counts and latency percentiles for one scope.
pub struct LlmMetricsPayload {
    /// Total calls.
    pub call_count: i64,
    /// Successful calls.
    pub success_count: i64,
    /// Failed calls.
    pub error_count: i64,
    /// Calls that used a fallback provider/model.
    pub fallback_count: i64,
    /// Median latency in milliseconds.
    pub p50_latency_ms: i64,
    /// 95th percentile latency in milliseconds.
    pub p95_latency_ms: i64,
    /// Per provider/model metrics.
    pub by_provider: Vec<ProviderMetric>,
}

#[derive(Debug, Clone, Serialize)]
/// Per provider/model LLM call metrics.
pub struct ProviderMetric {
    /// Provider name.
    pub provider: String,
    /// Model identifier.
    pub model: String,
    /// Total calls.
    pub call_count: i64,
    /// Successful calls.
    pub success_count: i64,
    /// Failed calls.
    pub error_count: i64,
    /// Calls that used a fallback provider/model.
    pub fallback_count: i64,
    /// Median latency in milliseconds.
    pub p50_latency_ms: i64,
    /// 95th percentile latency in milliseconds.
    pub p95_latency_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
/// CSV export outcome: local path, row count, and byte size.
pub struct ExportResult {
    /// Local destination path.
    pub path: String,
    /// Exported job rows.
    pub rows: u32,
    /// File size in bytes.
    pub bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
/// CSV export request with an operator-selected local destination.
pub struct ExportRequest {
    /// The selected scope.
    pub scope: AnalyticsScope,
    #[serde(default)]
    /// The run for CURRENT_RUN, when applicable.
    pub run_id: Option<String>,
    /// Operator-selected local path.
    pub destination: String,
}

/// Export the analytics CSV for a scope to a local destination.
pub fn export_csv(db: &Db, request: ExportRequest) -> Result<ExportResult, AppError> {
    csv::export(db, request)
}

/// One evaluation included in analytics aggregation.
#[derive(Debug, Clone)]
pub struct EligibleJob {
    /// Stable job identifier.
    pub job_id: JobId,
    /// Evaluation time (UTC).
    pub evaluated_at: Option<String>,
    /// Skills the profile lacks for this job.
    pub missing_keywords: Vec<String>,
    /// Skills matched by the profile.
    pub matched_keywords: Vec<String>,
    /// 0-100 match score, when evaluated.
    pub score: Option<i64>,
    /// True when the evaluation predates a profile change.
    pub stale: bool,
}

/// Jobs eligible for analytics: completed APPROVED or PENDING evaluations.
/// Rejected, ERROR, incomplete, and deleted evaluations are excluded.
pub fn eligible_jobs(
    db: &Db,
    scope: AnalyticsScope,
    run_id: Option<&RunId>,
) -> Result<Vec<EligibleJob>, AppError> {
    db.with_connection(|connection| {
        let mut values = Vec::new();
        match scope {
            AnalyticsScope::CurrentRun => {
                let Some(run_id) = run_id else {
                    return Ok(values);
                };
                let mut statement = connection.prepare(
                    "SELECT job_id, evaluated_at, missing_keywords_json, matched_keywords_json,
                            match_score, stale
                     FROM job_evaluations
                     WHERE run_id = ?1 AND evaluation_status IN ('APPROVED', 'PENDING')
                     ORDER BY evaluated_at ASC",
                )?;
                collect_jobs(
                    &mut statement,
                    rusqlite::params![run_id.as_str()],
                    &mut values,
                )?;
            }
            AnalyticsScope::Last7Days => {
                let mut statement = connection.prepare(
                    "SELECT job_id, evaluated_at, missing_keywords_json, matched_keywords_json,
                            match_score, stale
                     FROM job_evaluations
                     WHERE evaluation_status IN ('APPROVED', 'PENDING')
                       AND datetime(evaluated_at) >= datetime('now', '-7 days')
                     ORDER BY evaluated_at ASC",
                )?;
                collect_jobs(&mut statement, rusqlite::params![], &mut values)?;
            }
        }
        Ok(values)
    })
}

fn collect_jobs<P: rusqlite::Params>(
    statement: &mut rusqlite::Statement<'_>,
    params: P,
    values: &mut Vec<EligibleJob>,
) -> Result<(), AppError> {
    let rows = statement.query_map(params, |row| {
        Ok(EligibleJob {
            job_id: JobId::from(row.get::<_, String>(0)?),
            evaluated_at: row.get(1)?,
            missing_keywords: parse_keywords(row.get(2)?),
            matched_keywords: parse_keywords(row.get(3)?),
            score: row.get(4)?,
            stale: row.get(5)?,
        })
    })?;
    for row in rows {
        values.push(row?);
    }
    Ok(())
}

fn parse_keywords(value: Option<String>) -> Vec<String> {
    value
        .and_then(|value| serde_json::from_str::<Vec<String>>(&value).ok())
        .unwrap_or_default()
}

/// The stored run selected for CURRENT_RUN: explicit run when supplied,
/// otherwise the most recent SUCCESS/PARTIAL run.
pub fn resolve_run_id(
    db: &Db,
    requested: Option<&str>,
    scope: AnalyticsScope,
) -> Result<Option<RunId>, AppError> {
    if scope != AnalyticsScope::CurrentRun {
        return Ok(None);
    }
    if let Some(run_id) = requested {
        let exists: Option<String> = db.with_connection(|connection| {
            Ok(connection
                .query_row(
                    "SELECT id FROM execution_runs WHERE id = ?1",
                    [run_id],
                    |row| row.get(0),
                )
                .optional()?)
        })?;
        return exists
            .map(RunId::from)
            .map(Some)
            .ok_or_else(|| AppError::not_found("run"));
    }
    db.with_connection(|connection| {
        Ok(connection
            .query_row(
                "SELECT id FROM execution_runs
                 WHERE status IN ('SUCCESS', 'PARTIAL')
                 ORDER BY started_at DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0).map(RunId::from),
            )
            .optional()?)
    })
}

/// Placeholder until the aggregation queries land.
#[tracing::instrument(skip(db), fields(scope = ?request.scope))]
pub fn get_analytics(db: &Db, request: AnalyticsRequest) -> Result<AnalyticsPayload, AppError> {
    let run_id = resolve_run_id(db, request.run_id.as_deref(), request.scope)?;
    let jobs = eligible_jobs(db, request.scope, run_id.as_ref())?;
    let profile = crate::db::profile::load_active_profile(db)?;
    let denominator = u32::try_from(jobs.len())
        .map_err(|_| AppError::validation("too many eligible jobs for analytics"))?;
    let last_updated = jobs
        .iter()
        .filter_map(|job| job.evaluated_at.clone())
        .max()
        .unwrap_or_else(crate::db::now_rfc3339);
    let stale = jobs.iter().any(|job| job.stale);
    let run_ref = run_id.as_ref().map(RunId::as_str);
    let funnel = funnel::compute_funnel(db, request.scope, run_ref)?;
    let llm_metrics = funnel::llm_metrics(db, request.scope, run_ref)?;

    Ok(AnalyticsPayload {
        scope: request.scope,
        run_id: run_id.map(|value| value.as_str().to_string()),
        denominator,
        last_updated,
        stale,
        skill_gap: skill_gap::compute(&jobs, profile.as_ref()),
        funnel,
        score_distribution: funnel::score_distribution(&jobs),
        llm_metrics,
    })
}
