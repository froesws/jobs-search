//! Funnel, score-distribution, and LLM metric aggregation.

use std::collections::BTreeMap;

use crate::analytics::{AnalyticsScope, FunnelPayload};
use crate::db::Db;
use crate::error::AppError;

pub fn compute_funnel(
    db: &Db,
    scope: AnalyticsScope,
    run_id: Option<&str>,
) -> Result<FunnelPayload, AppError> {
    let mut payload = FunnelPayload::default();
    db.with_connection(|connection| {
        let (sql, params): (&str, Vec<&str>) = match scope {
            AnalyticsScope::CurrentRun => (
                "SELECT r.total_ingested, r.duplicates_merged, r.rejected_count,
                        r.pending_count, r.approved_count, r.error_count,
                        r.source_failures_json,
                        COALESCE(m.rejected_by_reason_json, '{}'),
                        COALESCE(m.applied_count, 0),
                        COALESCE(m.ignored_count, 0),
                        COALESCE(m.source_breakdown_json, '{}'),
                        COALESCE(m.provider_breakdown_json, '{}')
                 FROM execution_runs r
                 LEFT JOIN run_metrics m ON m.run_id = r.id
                 WHERE r.id = ?1",
                vec![run_id.unwrap_or_default()],
            ),
            AnalyticsScope::Last7Days => (
                "SELECT r.total_ingested, r.duplicates_merged, r.rejected_count,
                        r.pending_count, r.approved_count, r.error_count,
                        r.source_failures_json,
                        COALESCE(m.rejected_by_reason_json, '{}'),
                        COALESCE(m.applied_count, 0),
                        COALESCE(m.ignored_count, 0),
                        COALESCE(m.source_breakdown_json, '{}'),
                        COALESCE(m.provider_breakdown_json, '{}')
                 FROM execution_runs r
                 LEFT JOIN run_metrics m ON m.run_id = r.id
                 WHERE datetime(r.started_at) >= datetime('now', '-7 days')",
                Vec::new(),
            ),
        };

        let mut statement = connection.prepare(sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(params), |row| {
            Ok(RunRow {
                ingested: row.get(0)?,
                duplicates: row.get(1)?,
                rejected: row.get(2)?,
                pending: row.get(3)?,
                approved: row.get(4)?,
                error: row.get(5)?,
                source_failures: row.get(6)?,
                rejected_by_reason: row.get(7)?,
                applied: row.get(8)?,
                ignored: row.get(9)?,
                source_breakdown: row.get(10)?,
                provider_breakdown: row.get(11)?,
            })
        })?;

        let mut rejected = BTreeMap::new();
        let mut sources = BTreeMap::new();
        let mut providers = BTreeMap::new();
        let mut failures = Vec::new();
        let mut rejected_fallback = 0;
        for row in rows {
            let row = row?;
            rejected_fallback += row.rejected;
            payload.ingested += row.ingested;
            payload.duplicates += row.duplicates;
            payload.pending += row.pending;
            payload.approved += row.approved;
            payload.error += row.error;
            payload.applied += row.applied;
            payload.ignored += row.ignored;
            merge(&mut rejected, &row.rejected_by_reason);
            merge(&mut sources, &row.source_breakdown);
            merge(&mut providers, &row.provider_breakdown);
            for failure in parse_failures(&row.source_failures) {
                if !failures.contains(&failure) {
                    failures.push(failure);
                }
            }
        }
        payload.rejected_by_reason = rejected;
        payload.source_breakdown = sources;
        payload.provider_breakdown = providers;
        payload.source_failures = failures;
        let rejected_total: i64 = if payload.rejected_by_reason.is_empty() {
            rejected_fallback
        } else {
            payload.rejected_by_reason.values().sum()
        };
        payload.approval_rate = rate(
            payload.approved,
            payload.approved + payload.pending + rejected_total,
        );
        payload.application_rate = rate(payload.applied, payload.approved);
        Ok(())
    })?;
    Ok(payload)
}

struct RunRow {
    ingested: i64,
    duplicates: i64,
    rejected: i64,
    pending: i64,
    approved: i64,
    error: i64,
    source_failures: String,
    rejected_by_reason: String,
    applied: i64,
    ignored: i64,
    source_breakdown: String,
    provider_breakdown: String,
}

fn merge(target: &mut BTreeMap<String, i64>, json: &str) {
    if let Ok(values) = serde_json::from_str::<BTreeMap<String, i64>>(json) {
        for (key, count) in values {
            *target.entry(key).or_insert(0) += count;
        }
    }
}

fn parse_failures(json: &str) -> Vec<String> {
    serde_json::from_str(json).unwrap_or_default()
}

fn rate(numerator: i64, denominator: i64) -> f64 {
    if denominator <= 0 {
        0.0
    } else {
        ((numerator as f64 / denominator as f64) * 1000.0).round() / 10.0
    }
}
