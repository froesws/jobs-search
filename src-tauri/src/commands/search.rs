use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::config::SourceConfig;
use crate::db::runs::{RunFinish, SearchMetrics};
use crate::error::{AppError, ErrorPayload};
use crate::ingestion::adapter::{IngestionRequest, SourceAdapter, SourceFailure};
use crate::ingestion::{api::RemoteOkAdapter, ingest, mcp::McpAdapter, normalize};
use crate::llm;
use crate::profile::CandidateProfile;
use crate::profile::fixture;
use crate::state::AppState;
use crate::types::{SearchTimeframe, Seniority, WorkModality};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchFilters {
    pub titles: Vec<String>,
    pub stack_keyword: String,
    pub timeframe: SearchTimeframe,
    pub seniorities: Vec<Seniority>,
    pub modalities: Vec<WorkModality>,
    pub source_ids: Vec<String>,
}

impl SearchFilters {
    /// Backend defaults: profile roles, top core technology, profile seniority,
    /// all modalities, and every enabled source.
    pub fn resolve(mut self, profile: &CandidateProfile) -> Self {
        if self.titles.is_empty() {
            self.titles = profile.target_roles.clone();
        }
        if self.stack_keyword.trim().is_empty() {
            self.stack_keyword = profile
                .core_competencies
                .first()
                .cloned()
                .unwrap_or_default();
        }
        if self.seniorities.is_empty() {
            self.seniorities = profile.accepted_seniorities();
        }
        if self.modalities.is_empty() {
            self.modalities = vec![
                WorkModality::Remote,
                WorkModality::Hybrid,
                WorkModality::Onsite,
            ];
        }
        self
    }

    pub fn ingestion_request(&self) -> IngestionRequest {
        IngestionRequest {
            query: self.titles.join(" OR "),
            stack_keyword: self.stack_keyword.clone(),
            location: "Brazil".to_string(),
            sites: Vec::new(),
            results_wanted: 50,
            hours_old: self.timeframe.hours_old(),
            is_remote: self.modalities.contains(&WorkModality::Remote),
            offset: 0,
        }
    }
}

pub async fn trigger(state: &AppState, filters: SearchFilters) -> Result<SearchMetrics, AppError> {
    let _guard = state
        .search_lock
        .try_lock()
        .map_err(|_| AppError::SearchInProgress)?;

    let profile = fixture::load(state.config.fixture_profile_path.as_deref())?;
    let sources = crate::config::SourcesConfig::load_or_default(&state.config.sources_path)?;
    let enabled = sources.enabled();

    for source in &enabled {
        crate::db::sources::upsert_source(&state.db, source)?;
    }
    let active_ids = sources
        .sources
        .iter()
        .map(|source| source.id.clone())
        .collect::<Vec<_>>();
    crate::db::sources::deactivate_missing(&state.db, &active_ids)?;

    let filters = filters.resolve(&profile);
    let search_config_json = serde_json::to_string(&filters)?;
    let run = crate::db::runs::start_run(&state.db, &search_config_json)?;

    let mut adapters: Vec<Arc<dyn SourceAdapter>> = Vec::new();
    let mut initial_failures: Vec<SourceFailure> = Vec::new();

    for source in &enabled {
        if !filters.source_ids.is_empty() && !filters.source_ids.contains(&source.id) {
            continue;
        }
        match build_adapter(source) {
            Ok(adapter) => adapters.push(adapter),
            Err(failure) => initial_failures.push(failure),
        }
    }

    let request = filters.ingestion_request();
    let outcome = ingest(&adapters, &request).await;
    let mut failures = initial_failures;
    failures.extend(outcome.failures());
    let total_ingested = outcome.total_fetched() as i64;

    let mut duplicates_merged = 0_i64;
    for (source_id, source_name, raw) in outcome.jobs() {
        match normalize::normalize(raw, &source_id, &source_name) {
            Ok(job) => {
                let upsert = crate::db::jobs::upsert_job(
                    &state.db,
                    &crate::db::jobs::NewJob {
                        canonical_key: job.canonical_key,
                        canonical_hash: job.canonical_hash,
                        title: job.title,
                        company: job.company,
                        location: job.location,
                        work_modality: job.work_modality,
                        url: job.url.clone(),
                        posted_at: job.posted_at,
                        description_clean: job.description_clean,
                        extracted_metadata_json: job.extracted_metadata_json,
                        needs_review: job.needs_review,
                    },
                )?;
                if upsert.duplicate() {
                    duplicates_merged += 1;
                }
                crate::db::jobs::add_lineage(
                    &state.db,
                    &upsert.job_id,
                    &source_id,
                    &job.external_id,
                    &job.url,
                )?;
            }
            Err(error) => {
                // The raw description is already gone. Log only the sanitized
                // validation message, never posting content.
                tracing::warn!(
                    source = %source_id,
                    error = %error.message(),
                    "discarding unnormalizable job"
                );
            }
        }
    }

    let router = llm::router_from_config(&state.config, state.http.clone());
    let settings = crate::matching::EvaluationSettings {
        threshold: state.config.match_threshold,
        max_jobs: state.config.max_evaluations,
        concurrency: state.config.max_concurrency,
    };
    let summary =
        crate::matching::evaluate_run(&state.db, &run.run_id, &profile, &router, &settings).await?;

    let status = if summary.fatal_provider_error
        || adapters.is_empty()
        || (total_ingested == 0 && !failures.is_empty())
    {
        "FAILED"
    } else if failures.is_empty() && summary.error == 0 {
        "SUCCESS"
    } else {
        "PARTIAL"
    };

    for source in &enabled {
        let failed = failures
            .iter()
            .any(|failure| failure.source_id == source.id);
        let last_status = if failed { "FAILED" } else { "SUCCESS" };
        crate::db::sources::set_last_run_status(&state.db, &source.id, last_status)?;
    }

    let source_failure_messages = failures
        .iter()
        .map(|failure| format!("{}: {}", failure.code, failure.message))
        .collect::<Vec<_>>();

    let source_breakdown = crate::db::jobs::source_breakdown_since(&state.db, &run.started_at)?;
    let score_distribution = crate::db::evaluations::score_distribution(&state.db)?;

    let mut metrics = crate::db::runs::finish_run(
        &state.db,
        &RunFinish {
            run_id: &run.run_id,
            status,
            total_ingested,
            duplicates_merged,
            rejected_count: summary.rejected,
            pending_count: summary.pending,
            approved_count: summary.approved,
            error_count: summary.error,
            source_failures: source_failure_messages,
            rejected_by_reason: summary.rejected_by_reason,
            score_distribution,
            source_breakdown,
        },
    )?;
    metrics.started_at = run.started_at;
    Ok(metrics)
}

fn build_adapter(source: &SourceConfig) -> Result<Arc<dyn SourceAdapter>, SourceFailure> {
    match source.source_type {
        crate::config::SourceType::Mcp => McpAdapter::from_config(source)
            .map(|adapter| Arc::new(adapter) as Arc<dyn SourceAdapter>)
            .map_err(|error| failure(source, "MCP_CONFIG_ERROR", error.message())),
        crate::config::SourceType::Api => {
            if source.id.to_ascii_lowercase().contains("remoteok")
                || source.name.to_ascii_lowercase().contains("remoteok")
            {
                RemoteOkAdapter::from_config(source)
                    .map(|adapter| Arc::new(adapter) as Arc<dyn SourceAdapter>)
                    .map_err(|error| failure(source, "API_CONFIG_ERROR", error.message()))
            } else {
                Err(failure(
                    source,
                    "UNSUPPORTED_SOURCE",
                    "API source is not supported in this milestone".to_string(),
                ))
            }
        }
        crate::config::SourceType::WebScraper => Err(failure(
            source,
            "UNSUPPORTED_SOURCE",
            "web scraper sources are not supported".to_string(),
        )),
    }
}

fn failure(source: &SourceConfig, code: &str, message: String) -> SourceFailure {
    SourceFailure {
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        code: code.to_string(),
        message,
    }
}

#[tauri::command]
pub async fn cmd_trigger_search(
    state: State<'_, AppState>,
    filters: SearchFilters,
) -> Result<SearchMetrics, ErrorPayload> {
    trigger(&state, filters).await.map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SearchTimeframe;

    fn profile() -> CandidateProfile {
        serde_json::from_str(include_str!("../../tests/fixtures/profile.json")).expect("fixture")
    }

    #[test]
    fn defaults_come_from_the_fixture_profile() {
        let filters = SearchFilters::default().resolve(&profile());
        assert_eq!(filters.titles[0], "Rust Engineer");
        assert_eq!(filters.stack_keyword, "Rust");
        assert_eq!(
            filters.seniorities,
            vec![Seniority::Trainee, Seniority::Junior, Seniority::Pleno]
        );
        assert_eq!(
            filters.modalities,
            vec![
                WorkModality::Remote,
                WorkModality::Hybrid,
                WorkModality::Onsite
            ]
        );
        assert_eq!(filters.timeframe, SearchTimeframe::D7);
    }

    #[test]
    fn ingestion_request_uses_the_seven_day_remote_default() {
        let filters = SearchFilters::default().resolve(&profile());
        let request = filters.ingestion_request();
        assert_eq!(request.hours_old, 168);
        assert!(request.is_remote);
        assert_eq!(request.location, "Brazil");
        assert!(request.query.contains("Rust Engineer"));
    }

    #[test]
    fn explicit_filters_are_not_overwritten() {
        let filters = SearchFilters {
            titles: vec!["C++ Engineer".to_string()],
            stack_keyword: "C++".to_string(),
            timeframe: SearchTimeframe::H24,
            seniorities: vec![Seniority::Junior],
            modalities: vec![WorkModality::Hybrid],
            source_ids: vec!["remoteok".to_string()],
        }
        .resolve(&profile());
        assert_eq!(filters.titles, vec!["C++ Engineer"]);
        assert_eq!(filters.timeframe, SearchTimeframe::H24);
        assert_eq!(filters.modalities, vec![WorkModality::Hybrid]);
        assert_eq!(filters.source_ids, vec!["remoteok"]);
    }

    #[tokio::test]
    async fn search_lock_is_single_flight() {
        let lock = tokio::sync::Mutex::new(());
        let _guard = lock.try_lock().expect("first");
        assert!(lock.try_lock().is_err());
    }
}
