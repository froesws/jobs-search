pub mod csv;
pub mod funnel;
pub mod skill_gap;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnalyticsScope {
    CurrentRun,
    Last7Days,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalyticsRequest {
    pub scope: AnalyticsScope,
    #[serde(default)]
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyticsPayload {
    pub scope: AnalyticsScope,
    pub run_id: Option<String>,
    pub denominator: u32,
    pub last_updated: String,
    pub stale: bool,
    pub skill_gap: Vec<SkillGapMetric>,
    pub funnel: FunnelPayload,
    pub score_distribution: Vec<ScoreBucket>,
    pub llm_metrics: LlmMetricsPayload,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SkillGapMetric {
    pub skill: String,
    pub display_name: String,
    pub demand_count: u32,
    pub demand_percentage: f64,
    pub rank: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct FunnelPayload {
    pub ingested: i64,
    pub duplicates: i64,
    pub rejected_by_reason: BTreeMap<String, i64>,
    pub pending: i64,
    pub approved: i64,
    pub applied: i64,
    pub ignored: i64,
    pub approval_rate: f64,
    pub application_rate: f64,
    pub source_breakdown: BTreeMap<String, i64>,
    pub provider_breakdown: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ScoreBucket {
    pub range: String,
    pub count: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct LlmMetricsPayload {
    pub call_count: i64,
    pub success_count: i64,
    pub error_count: i64,
    pub fallback_count: i64,
    pub p50_latency_ms: i64,
    pub p95_latency_ms: i64,
    pub by_provider: Vec<ProviderMetric>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderMetric {
    pub provider: String,
    pub model: String,
    pub call_count: i64,
    pub success_count: i64,
    pub error_count: i64,
    pub fallback_count: i64,
    pub p50_latency_ms: i64,
    pub p95_latency_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportResult {
    pub path: String,
    pub rows: u32,
    pub bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExportRequest {
    pub scope: AnalyticsScope,
    #[serde(default)]
    pub run_id: Option<String>,
    pub destination: String,
}

/// Placeholder until the CSV writer lands.
pub fn export_csv(_db: &Db, _request: ExportRequest) -> Result<ExportResult, AppError> {
    Err(AppError::Config(
        "analytics CSV export is not available yet".to_string(),
    ))
}

/// Placeholder until the aggregation queries land.
pub fn get_analytics(_db: &Db, request: AnalyticsRequest) -> Result<AnalyticsPayload, AppError> {
    Ok(AnalyticsPayload {
        scope: request.scope,
        run_id: request.run_id,
        denominator: 0,
        last_updated: crate::db::now_rfc3339(),
        stale: false,
        skill_gap: Vec::new(),
        funnel: FunnelPayload::default(),
        score_distribution: Vec::new(),
        llm_metrics: LlmMetricsPayload::default(),
    })
}
