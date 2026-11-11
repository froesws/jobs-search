//! Funnel, score-distribution, and LLM metric aggregation.

use std::collections::BTreeMap;

use crate::analytics::{
    AnalyticsScope, FunnelPayload, LlmMetricsPayload, ProviderMetric, ScoreBucket,
};
use crate::db::Db;
use crate::error::AppError;

const BUCKETS: [(&str, i64, i64); 6] = [
    ("0-49", 0, 49),
    ("50-59", 50, 59),
    ("60-69", 60, 69),
    ("70-79", 70, 79),
    ("80-89", 80, 89),
    ("90-100", 90, 100),
];

pub fn score_distribution(jobs: &[crate::analytics::EligibleJob]) -> Vec<ScoreBucket> {
    BUCKETS
        .iter()
        .map(|(range, min, max)| ScoreBucket {
            range: (*range).to_string(),
            count: jobs
                .iter()
                .filter(|job| {
                    job.score
                        .is_some_and(|score| score >= *min && score <= *max)
                })
                .count() as i64,
        })
        .collect()
}

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

pub fn llm_metrics(
    db: &Db,
    scope: AnalyticsScope,
    run_id: Option<&str>,
) -> Result<LlmMetricsPayload, AppError> {
    db.with_connection(|connection| {
        let (sql, params): (&str, Vec<&str>) = match scope {
            AnalyticsScope::CurrentRun => (
                "SELECT provider, model, latency_ms, outcome, fallback_from
                 FROM llm_metrics WHERE run_id = ?1 ORDER BY created_at ASC",
                vec![run_id.unwrap_or_default()],
            ),
            AnalyticsScope::Last7Days => (
                "SELECT provider, model, latency_ms, outcome, fallback_from
                 FROM llm_metrics
                 WHERE datetime(created_at) >= datetime('now', '-7 days')
                 ORDER BY created_at ASC",
                Vec::new(),
            ),
        };
        let mut statement = connection.prepare(sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(params), |row| {
            Ok(LlmRow {
                provider: row.get(0)?,
                model: row.get(1)?,
                latency: row.get(2)?,
                outcome: row.get(3)?,
                fallback_from: row.get::<_, Option<String>>(4)?,
            })
        })?;

        let mut payload = LlmMetricsPayload::default();
        let mut latencies = Vec::new();
        let mut providers: BTreeMap<(String, String), ProviderAccumulator> = BTreeMap::new();
        for row in rows {
            let row = row?;
            latencies.push(row.latency);
            let is_success = row.outcome == "SUCCESS";
            let is_error = row.outcome == "ERROR";
            let is_fallback = row.fallback_from.is_some() || row.outcome == "FALLBACK";
            if is_success {
                payload.success_count += 1;
            }
            if is_error {
                payload.error_count += 1;
            }
            if is_fallback {
                payload.fallback_count += 1;
            }
            let entry = providers.entry((row.provider, row.model)).or_default();
            entry.calls += 1;
            entry.success += i64::from(is_success);
            entry.error += i64::from(is_error);
            entry.fallback += i64::from(is_fallback);
            entry.latencies.push(row.latency);
        }
        payload.call_count = latencies.len() as i64;
        payload.p50_latency_ms = percentile(&mut latencies, 50.0);
        payload.p95_latency_ms = percentile(&mut latencies, 95.0);
        payload.by_provider = providers
            .into_iter()
            .map(|((provider, model), mut value)| ProviderMetric {
                provider,
                model,
                call_count: value.calls,
                success_count: value.success,
                error_count: value.error,
                fallback_count: value.fallback,
                p50_latency_ms: percentile(&mut value.latencies, 50.0),
                p95_latency_ms: percentile(&mut value.latencies, 95.0),
            })
            .collect();
        Ok(payload)
    })
}

#[derive(Default)]
struct ProviderAccumulator {
    calls: i64,
    success: i64,
    error: i64,
    fallback: i64,
    latencies: Vec<i64>,
}

struct LlmRow {
    provider: String,
    model: String,
    latency: i64,
    outcome: String,
    fallback_from: Option<String>,
}

fn percentile(values: &mut [i64], percentile: f64) -> i64 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    let rank = ((percentile / 100.0) * values.len() as f64).ceil() as usize;
    values[rank.saturating_sub(1).min(values.len() - 1)]
}
