use std::collections::BTreeMap;

use rusqlite::params;

use crate::db::{Db, now_rfc3339};
use crate::error::AppError;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RunStart {
    pub run_id: String,
    pub started_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchMetrics {
    pub run_id: String,
    pub status: String,
    pub total_ingested: i64,
    pub duplicates_merged: i64,
    pub rejected_count: i64,
    pub pending_count: i64,
    pub approved_count: i64,
    pub error_count: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub source_failures: Vec<String>,
}

pub struct RunFinish<'a> {
    pub run_id: &'a str,
    pub status: &'a str,
    pub total_ingested: i64,
    pub duplicates_merged: i64,
    pub rejected_count: i64,
    pub pending_count: i64,
    pub approved_count: i64,
    pub error_count: i64,
    pub source_failures: Vec<String>,
    pub rejected_by_reason: BTreeMap<String, i64>,
    pub score_distribution: BTreeMap<String, i64>,
    pub source_breakdown: BTreeMap<String, i64>,
}

impl RunFinish<'_> {
    pub fn evaluated_count(&self) -> i64 {
        self.rejected_count + self.pending_count + self.approved_count
    }

    pub fn approval_rate(&self) -> f64 {
        if self.evaluated_count() == 0 {
            0.0
        } else {
            self.approved_count as f64 / self.evaluated_count() as f64 * 100.0
        }
    }
}

pub fn start_run(db: &Db, search_config_json: &str) -> Result<RunStart, AppError> {
    let run_id = uuid::Uuid::new_v4().to_string();
    let started_at = now_rfc3339();
    db.with_connection(|connection| {
        connection.execute(
            "INSERT INTO execution_runs
             (id, started_at, finished_at, search_config_json, status)
             VALUES (?1, ?2, NULL, ?3, 'RUNNING')",
            params![run_id, started_at, search_config_json],
        )?;
        Ok(())
    })?;
    Ok(RunStart { run_id, started_at })
}

pub fn finish_run(db: &Db, finish: &RunFinish<'_>) -> Result<SearchMetrics, AppError> {
    let finished_at = now_rfc3339();
    let source_failures_json = serde_json::to_string(&finish.source_failures)?;
    let rejected_by_reason = serde_json::to_string(&finish.rejected_by_reason)?;
    let score_distribution = serde_json::to_string(&finish.score_distribution)?;
    let source_breakdown = serde_json::to_string(&finish.source_breakdown)?;

    let metrics = db.with_connection_mut(|connection| {
        let transaction = connection.transaction()?;
        transaction.execute(
            "UPDATE execution_runs SET
                finished_at = ?1, status = ?2, total_ingested = ?3,
                duplicates_merged = ?4, rejected_count = ?5, pending_count = ?6,
                approved_count = ?7, error_count = ?8, source_failures_json = ?9
             WHERE id = ?10",
            params![
                finished_at,
                finish.status,
                finish.total_ingested,
                finish.duplicates_merged,
                finish.rejected_count,
                finish.pending_count,
                finish.approved_count,
                finish.error_count,
                source_failures_json,
                finish.run_id,
            ],
        )?;

        let provider_breakdown = provider_breakdown_with_connection(&transaction, finish.run_id)?;
        transaction.execute(
            "INSERT INTO run_metrics
             (run_id, total_ingested, duplicates_merged, rejected_by_reason_json,
              pending_count, approved_count, applied_count, ignored_count,
              approval_rate, application_rate, score_distribution_json,
              source_breakdown_json, provider_breakdown_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, ?7, 0, ?8, ?9, ?10, ?11)
             ON CONFLICT(run_id) DO UPDATE SET
                total_ingested = excluded.total_ingested,
                duplicates_merged = excluded.duplicates_merged,
                rejected_by_reason_json = excluded.rejected_by_reason_json,
                pending_count = excluded.pending_count,
                approved_count = excluded.approved_count,
                approval_rate = excluded.approval_rate,
                score_distribution_json = excluded.score_distribution_json,
                source_breakdown_json = excluded.source_breakdown_json,
                provider_breakdown_json = excluded.provider_breakdown_json,
                updated_at = excluded.updated_at",
            params![
                finish.run_id,
                finish.total_ingested,
                finish.duplicates_merged,
                rejected_by_reason,
                finish.pending_count,
                finish.approved_count,
                finish.approval_rate(),
                score_distribution,
                source_breakdown,
                serde_json::to_string(&provider_breakdown)?,
                finished_at,
            ],
        )?;
        transaction.commit()?;
        Ok(SearchMetrics {
            run_id: finish.run_id.to_string(),
            status: finish.status.to_string(),
            total_ingested: finish.total_ingested,
            duplicates_merged: finish.duplicates_merged,
            rejected_count: finish.rejected_count,
            pending_count: finish.pending_count,
            approved_count: finish.approved_count,
            error_count: finish.error_count,
            started_at: String::new(),
            finished_at: Some(finished_at.clone()),
            source_failures: finish.source_failures.clone(),
        })
    })?;

    Ok(metrics)
}

fn provider_breakdown_with_connection(
    connection: &rusqlite::Connection,
    run_id: &str,
) -> Result<BTreeMap<String, i64>, AppError> {
    let mut statement = connection.prepare(
        "SELECT provider || '/' || model, COUNT(*) FROM llm_metrics
         WHERE run_id = ?1 GROUP BY provider, model ORDER BY provider",
    )?;
    let rows = statement.query_map([run_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    let mut values = BTreeMap::new();
    for row in rows {
        let (key, count) = row?;
        values.insert(key, count);
    }
    Ok(values)
}

#[allow(clippy::too_many_arguments)]
pub fn record_llm_metric(
    db: &Db,
    run_id: &str,
    provider: &str,
    model: &str,
    operation: &str,
    latency_ms: i64,
    outcome: &str,
    error_category: Option<&str>,
    fallback_from: Option<&str>,
) -> Result<(), AppError> {
    db.with_connection(|connection| {
        connection.execute(
            "INSERT INTO llm_metrics
             (id, run_id, provider, model, operation, latency_ms, outcome,
              error_category, fallback_from, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                uuid::Uuid::new_v4().to_string(),
                run_id,
                provider,
                model,
                operation,
                latency_ms,
                outcome,
                error_category,
                fallback_from,
                now_rfc3339(),
            ],
        )?;
        Ok(())
    })
}
