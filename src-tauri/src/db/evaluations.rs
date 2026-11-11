//! Evaluation persistence and stale marking.
#![allow(missing_docs)]

use std::collections::BTreeMap;

use rusqlite::params;

use crate::db::{Db, now_rfc3339};
use crate::error::AppError;
use crate::types::{EvaluationStatus, HardFilterRejection, JobId, RunId};

pub const RULES_VERSION: &str = "v1-40-25-20-15";

#[derive(Debug, Clone)]
pub struct EvaluationRecord {
    pub job_id: JobId,
    pub run_id: RunId,
    pub profile_version: i64,
    pub status: EvaluationStatus,
    pub score: Option<i64>,
    pub matching_items: Option<i64>,
    pub total_requirements: Option<i64>,
    pub breakdown: Option<serde_json::Value>,
    pub matched_keywords: Vec<String>,
    pub missing_keywords: Vec<String>,
    pub rejections: Vec<HardFilterRejection>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub threshold: i64,
}

pub fn upsert_evaluation(db: &Db, record: &EvaluationRecord) -> Result<(), AppError> {
    let now = now_rfc3339();
    let breakdown = record
        .breakdown
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let matched = serde_json::to_string(&record.matched_keywords)?;
    let missing = serde_json::to_string(&record.missing_keywords)?;
    let rejections = serde_json::to_string(&record.rejections)?;

    db.with_connection_mut(|connection| {
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO job_evaluations
             (id, job_id, run_id, profile_version, evaluation_status, match_score,
              matching_items_count, total_requirements_count, score_breakdown_json,
              matched_keywords_json, missing_keywords_json, hard_filter_rejections_json,
              rules_version, threshold, provider, model, evaluated_at, stale)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, 0)
             ON CONFLICT(job_id) DO UPDATE SET
                run_id = excluded.run_id,
                profile_version = excluded.profile_version,
                evaluation_status = excluded.evaluation_status,
                match_score = excluded.match_score,
                matching_items_count = excluded.matching_items_count,
                total_requirements_count = excluded.total_requirements_count,
                score_breakdown_json = excluded.score_breakdown_json,
                matched_keywords_json = excluded.matched_keywords_json,
                missing_keywords_json = excluded.missing_keywords_json,
                hard_filter_rejections_json = excluded.hard_filter_rejections_json,
                rules_version = excluded.rules_version,
                threshold = excluded.threshold,
                provider = excluded.provider,
                model = excluded.model,
                evaluated_at = excluded.evaluated_at,
                stale = 0",
            params![
                uuid::Uuid::new_v4().to_string(),
                record.job_id.as_str(),
                record.run_id.as_str(),
                record.profile_version,
                record.status.as_str(),
                record.score,
                record.matching_items,
                record.total_requirements,
                breakdown,
                matched,
                missing,
                rejections,
                RULES_VERSION,
                record.threshold,
                record.provider,
                record.model,
                now,
            ],
        )?;

        transaction.execute(
            "INSERT INTO job_status_events
             (id, job_id, event_type, old_value, new_value, reason, created_at)
             VALUES (?1, ?2, 'EVALUATED', NULL, ?3, NULL, ?4)",
            params![
                uuid::Uuid::new_v4().to_string(),
                record.job_id.as_str(),
                record.status.as_str(),
                now,
            ],
        )?;

        let decision_event = match record.status {
            EvaluationStatus::Approved => Some("APPROVED"),
            EvaluationStatus::Rejected => Some("REJECTED"),
            _ => None,
        };
        if let Some(event_type) = decision_event {
            transaction.execute(
                "INSERT INTO job_status_events
                 (id, job_id, event_type, old_value, new_value, reason, created_at)
                 VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    record.job_id.as_str(),
                    event_type,
                    record.status.as_str(),
                    format!("{RULES_VERSION}; threshold {}", record.threshold),
                    now,
                ],
            )?;
        }

        transaction.commit()?;
        Ok(())
    })
}

pub fn mark_stale(db: &Db, reason: &str) -> Result<(), AppError> {
    db.with_connection(|connection| {
        connection.execute("UPDATE job_evaluations SET stale = 1", [])?;
        connection.execute(
            "INSERT INTO job_status_events
             (id, job_id, event_type, old_value, new_value, reason, created_at)
             SELECT ?1 || ':' || job_id, job_id, 'EVALUATED', evaluation_status,
                    evaluation_status, ?2, ?3
             FROM job_evaluations",
            params![uuid::Uuid::new_v4().to_string(), reason, now_rfc3339()],
        )?;
        Ok(())
    })
}

/// Distribution of scores for evaluated jobs (approved + pending).
pub fn score_distribution(db: &Db) -> Result<BTreeMap<String, i64>, AppError> {
    db.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT match_score FROM job_evaluations
             WHERE match_score IS NOT NULL
               AND evaluation_status IN ('APPROVED', 'PENDING')",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
        let mut buckets: BTreeMap<String, i64> = BTreeMap::from([
            ("0-49".to_string(), 0),
            ("50-59".to_string(), 0),
            ("60-69".to_string(), 0),
            ("70-79".to_string(), 0),
            ("80-89".to_string(), 0),
            ("90-100".to_string(), 0),
        ]);
        for row in rows {
            let score = row?;
            let label = match score {
                0..=49 => "0-49",
                50..=59 => "50-59",
                60..=69 => "60-69",
                70..=79 => "70-79",
                80..=89 => "80-89",
                _ => "90-100",
            };
            if let Some(count) = buckets.get_mut(label) {
                *count += 1;
            }
        }
        Ok(buckets)
    })
}

/// Counts of hard-filter rejection codes across stored evaluations.
pub fn rejected_by_reason(db: &Db) -> Result<BTreeMap<String, i64>, AppError> {
    db.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT hard_filter_rejections_json FROM job_evaluations
             WHERE evaluation_status = 'REJECTED'",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, Option<String>>(0))?;
        let mut counts = BTreeMap::new();
        for row in rows {
            let Some(json) = row? else {
                continue;
            };
            let Ok(rejections) = serde_json::from_str::<Vec<HardFilterRejection>>(&json) else {
                continue;
            };
            for rejection in rejections {
                *counts.entry(rejection.code).or_insert(0) += 1;
            }
        }
        Ok(counts)
    })
}
