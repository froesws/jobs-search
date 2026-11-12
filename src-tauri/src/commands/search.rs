//! Search trigger and filter resolution.
#![allow(missing_docs)]

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use crate::config::SourceConfig;
use crate::db::runs::{RunFinish, SearchMetrics};
use crate::error::{AppError, ErrorPayload};
use crate::ingestion::adapter::{IngestionRequest, SourceAdapter, SourceFailure};
use crate::ingestion::{
    IngestionProgressSink, api::RemoteOkAdapter, ingest_with_progress,
    linkedin_mcp::LinkedInMcpAdapter, mcp::McpAdapter, normalize,
};
use crate::llm;
use crate::matching::ProgressSink;
use crate::matching::filters::SearchCriteria;
use crate::profile::CandidateProfile;
use crate::state::AppState;
use crate::types::{SearchTimeframe, Seniority, WorkModality};

/// Tauri event emitted while a search run is active.
pub const SEARCH_PROGRESS_EVENT: &str = "search://progress";

const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

/// Stage of an active search run. It only moves forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchStage {
    Collecting,
    Evaluating,
}

/// Aggregate progress for an active run. Counters only, never content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchProgress {
    pub stage: SearchStage,
    pub found: u32,
    pub evaluated: u32,
    pub total: u32,
    pub failed: u32,
}

struct ProgressState {
    progress: SearchProgress,
    last_emit: Option<Instant>,
    dirty: bool,
}

/// Emits coalesced progress events for one run. Interior mutability keeps the
/// counters owned by the run instead of the database.
pub struct SearchProgressTracker {
    emit: Box<dyn Fn(SearchProgress) + Send + Sync>,
    interval: Duration,
    state: Mutex<ProgressState>,
    finished: AtomicBool,
}

impl SearchProgressTracker {
    /// A tracker that emits at most one event per 250 ms.
    pub fn new(emit: impl Fn(SearchProgress) + Send + Sync + 'static) -> Self {
        Self::with_interval(emit, PROGRESS_INTERVAL)
    }

    /// Same as [`Self::new`] with an explicit coalescing interval. Tests use
    /// `Duration::ZERO` to observe every update.
    pub fn with_interval(
        emit: impl Fn(SearchProgress) + Send + Sync + 'static,
        interval: Duration,
    ) -> Self {
        let tracker = Self {
            emit: Box::new(emit),
            interval,
            state: Mutex::new(ProgressState {
                progress: SearchProgress {
                    stage: SearchStage::Collecting,
                    found: 0,
                    evaluated: 0,
                    total: 0,
                    failed: 0,
                },
                last_emit: None,
                dirty: false,
            }),
            finished: AtomicBool::new(false),
        };
        // The first event must reach the UI within one second of the command.
        tracker.update(|_| {}, true);
        tracker
    }

    /// Records collection counters. `found` counts listed postings, including
    /// known postings whose detail request was skipped.
    pub fn collecting(&self, found: u32, failed: u32) {
        self.update(
            |progress| {
                if progress.stage == SearchStage::Collecting {
                    progress.found = progress.found.max(found);
                    progress.failed = progress.failed.max(failed);
                }
            },
            false,
        );
    }

    /// Moves to the evaluation stage and fixes the total for the run.
    pub fn begin_evaluating(&self, total: u32) {
        self.update(
            |progress| {
                if progress.stage == SearchStage::Collecting {
                    progress.stage = SearchStage::Evaluating;
                    progress.total = total;
                    progress.evaluated = 0;
                }
            },
            true,
        );
    }

    /// Records evaluated jobs. The value never decreases or exceeds `total`.
    pub fn evaluated(&self, evaluated: u32) {
        self.update(
            |progress| {
                if progress.stage == SearchStage::Evaluating {
                    progress.evaluated = evaluated.min(progress.total).max(progress.evaluated);
                }
            },
            false,
        );
    }

    /// Emits the final event. Safe to call more than once.
    pub fn finish(&self) {
        if self.finished.swap(true, Ordering::SeqCst) {
            return;
        }
        self.update(|_| {}, true);
    }

    fn update(&self, apply: impl FnOnce(&mut SearchProgress), force: bool) {
        let snapshot = {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            apply(&mut state.progress);
            let due = force
                || state
                    .last_emit
                    .is_none_or(|last| last.elapsed() >= self.interval);
            if !due {
                state.dirty = true;
                return;
            }
            state.last_emit = Some(Instant::now());
            state.dirty = false;
            state.progress
        };
        (self.emit)(snapshot);
    }
}

impl Drop for SearchProgressTracker {
    fn drop(&mut self) {
        // Guarantee a final event even when the run returns early or fails.
        self.finish();
    }
}

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
    #[must_use]
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

    pub fn criteria(&self) -> SearchCriteria {
        SearchCriteria {
            seniorities: self.seniorities.clone(),
            modalities: self.modalities.clone(),
        }
    }

    pub fn ingestion_queries(&self) -> Vec<String> {
        let base = if self.titles.is_empty() {
            self.stack_keyword.trim().to_string()
        } else {
            self.titles.join(" OR ")
        };
        let base = if base.trim().is_empty() {
            "jobs".to_string()
        } else {
            base
        };

        let mut modalities = Vec::new();
        for modality in &self.modalities {
            if !modalities.contains(modality) {
                modalities.push(*modality);
            }
        }
        let modality_terms = modalities
            .iter()
            .filter_map(|modality| modality.search_term_pt())
            .collect::<Vec<_>>();
        let suffix = if modalities.is_empty() || modalities.len() >= 3 || modality_terms.is_empty()
        {
            String::new()
        } else {
            format!(" ({})", modality_terms.join(" OR "))
        };

        let seniorities = if self.seniorities.is_empty() {
            vec![
                Seniority::Trainee,
                Seniority::Intern,
                Seniority::Junior,
                Seniority::Pleno,
                Seniority::Senior,
            ]
        } else {
            let mut values = Vec::new();
            for seniority in &self.seniorities {
                if !values.contains(seniority) {
                    values.push(*seniority);
                }
            }
            values
        };

        let mut queries = Vec::new();
        for seniority in seniorities {
            let query = format!("{base} {}{suffix}", seniority.search_term_pt())
                .trim()
                .to_string();
            if !queries.contains(&query) {
                queries.push(query);
            }
        }
        if queries.is_empty() {
            queries.push(base);
        }
        queries
    }

    pub fn ingestion_request(&self) -> IngestionRequest {
        let queries = self.ingestion_queries();
        IngestionRequest {
            query: queries.first().cloned().unwrap_or_default(),
            queries,
            titles: self.titles.clone(),
            stack_keyword: self.stack_keyword.clone(),
            location: "Brazil".to_string(),
            sites: Vec::new(),
            results_wanted: 50,
            hours_old: self.timeframe.hours_old(),
            is_remote: self.modalities.len() == 1
                && self.modalities.first() == Some(&WorkModality::Remote),
            offset: 0,
            seniorities: self.seniorities.clone(),
            modalities: self.modalities.clone(),
            known_postings: Vec::new(),
        }
    }
}

pub async fn trigger(
    state: &AppState,
    filters: SearchFilters,
    progress: Option<Arc<SearchProgressTracker>>,
) -> Result<SearchMetrics, AppError> {
    let _guard = state
        .search_lock
        .try_lock()
        .map_err(|_| AppError::SearchInProgress)?;

    let config = state.config()?;
    let profile = crate::commands::profile::load_profile_for_search(state)?;
    let sources = crate::config::SourcesConfig::load_or_default(&config.sources_path)?;
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
    let criteria = filters.criteria();
    let search_config_json = serde_json::to_string(&filters)?;
    let run = crate::db::runs::start_run(&state.db, &search_config_json)?;

    // Read the known set once, before any network call, so the database lock
    // is never held across an `.await`.
    let known = crate::db::jobs::known_postings(&state.db)?;

    // Resolve the provider chain at the start of the run so the evaluation
    // concurrency and the router come from one consistent snapshot.
    let session = state.session_selection()?;
    let provider_settings =
        crate::config::ProviderSettings::resolve(&state.db, &config, session.as_ref())?;
    let router = llm::router_from_config(&config, &provider_settings, state.http.clone());
    let settings = crate::matching::EvaluationSettings {
        threshold: config.match_threshold,
        max_jobs: config.max_evaluations,
        concurrency: provider_settings.evaluation_concurrency,
        criteria,
    };

    let ingestion = ingest_jobs(state, &enabled, &filters, known, progress.as_ref()).await?;
    let total =
        crate::db::jobs::count_for_evaluation(&state.db, settings.max_jobs.clamp(1, 1_000))?;
    if let Some(tracker) = &progress {
        tracker.begin_evaluating(u32::try_from(total).unwrap_or(u32::MAX));
    }
    let evaluation_sink: Option<ProgressSink> = progress.as_ref().map(|tracker| {
        let tracker = tracker.clone();
        Arc::new(move |update: crate::matching::EvaluationProgress| {
            tracker.evaluated(u32::try_from(update.evaluated).unwrap_or(u32::MAX));
        }) as ProgressSink
    });
    let summary = crate::matching::evaluate_run(
        &state.db,
        &run.run_id,
        &profile,
        &router,
        &settings,
        evaluation_sink.as_ref(),
    )
    .await?;
    if let Some(tracker) = &progress {
        tracker.finish();
    }

    let status = if summary.fatal_provider_error
        || !ingestion.adapters_built
        || (ingestion.total_ingested == 0 && !ingestion.failures.is_empty())
    {
        "FAILED"
    } else if ingestion.failures.is_empty() && summary.error == 0 {
        "SUCCESS"
    } else {
        "PARTIAL"
    };

    for source in &enabled {
        let Some(last_status) = source_status(&ingestion, &source.id) else {
            // A fallback that never ran keeps its previous status.
            continue;
        };
        crate::db::sources::set_last_run_status(&state.db, &source.id, last_status)?;
    }

    let source_failure_messages = ingestion
        .failures
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
            total_ingested: ingestion.total_ingested,
            duplicates_merged: ingestion.duplicates_merged,
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

struct IngestionResult {
    total_ingested: i64,
    duplicates_merged: i64,
    failures: Vec<SourceFailure>,
    fallback_skipped: Vec<String>,
    adapters_built: bool,
}

/// The status to persist for a source, or `None` when its outcome must not
/// touch `job_sources.last_run_status` because it was skipped as a fallback.
fn source_status(ingestion: &IngestionResult, source_id: &str) -> Option<&'static str> {
    if ingestion
        .fallback_skipped
        .iter()
        .any(|skipped| skipped == source_id)
    {
        return None;
    }
    let failed = ingestion
        .failures
        .iter()
        .any(|failure| failure.source_id == source_id);
    Some(if failed { "FAILED" } else { "SUCCESS" })
}

async fn ingest_jobs(
    state: &AppState,
    enabled: &[&SourceConfig],
    filters: &SearchFilters,
    known_postings: Vec<crate::ingestion::adapter::KnownPosting>,
    progress: Option<&Arc<SearchProgressTracker>>,
) -> Result<IngestionResult, AppError> {
    let mut adapters: Vec<Arc<dyn SourceAdapter>> = Vec::new();
    let mut failures: Vec<SourceFailure> = Vec::new();
    let config = state.config()?;
    let sources_base = config
        .sources_path
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    for source in enabled {
        if !filters.source_ids.is_empty() && !filters.source_ids.contains(&source.id) {
            continue;
        }
        match build_adapter(source, sources_base) {
            Ok(adapter) => adapters.push(adapter),
            Err(failure) => failures.push(failure),
        }
    }

    let adapters_built = !adapters.is_empty();
    let mut request = filters.ingestion_request();
    request.known_postings = known_postings;
    let sink: Option<IngestionProgressSink> = progress.map(|tracker| {
        let tracker = tracker.clone();
        Arc::new(move |update: crate::ingestion::IngestionProgress| {
            tracker.collecting(update.found, update.failed);
        }) as IngestionProgressSink
    });
    let outcome = ingest_with_progress(&adapters, &request, sink.as_ref()).await;
    failures.extend(outcome.failures());
    let total_ingested = outcome.total_fetched() as i64;
    let fallback_skipped = outcome
        .sources
        .iter()
        .filter(|source| source.fallback_skipped)
        .map(|source| source.source_id.clone())
        .collect::<Vec<_>>();

    // A known posting was listed again but its detail was reused. Refresh the
    // lineage timestamp so the audit shows the sighting without re-downloading
    // the posting body.
    let mut skipped_known = 0_usize;
    for source in &outcome.sources {
        for posting in &source.known_skipped {
            skipped_known += 1;
            crate::db::jobs::touch_source_lineage(
                &state.db,
                &crate::types::JobId::from(posting.job_id.clone()),
                &source.source_id,
                &posting.external_id,
                &posting.url,
            )?;
        }
    }
    if skipped_known > 0 {
        tracing::info!(
            skipped_known,
            "reused known postings instead of re-fetching details"
        );
    }

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

    Ok(IngestionResult {
        total_ingested,
        duplicates_merged,
        failures,
        fallback_skipped,
        adapters_built,
    })
}

fn build_adapter(
    source: &SourceConfig,
    sources_base: &Path,
) -> Result<Arc<dyn SourceAdapter>, SourceFailure> {
    match source.source_type {
        crate::config::SourceType::Mcp => {
            if source.config_str("adapter") == Some("linkedin") {
                LinkedInMcpAdapter::from_config(source, sources_base)
                    .map(|adapter| Arc::new(adapter) as Arc<dyn SourceAdapter>)
                    .map_err(|error| failure(source, "MCP_CONFIG_ERROR", error.message()))
            } else {
                McpAdapter::from_config(source, sources_base)
                    .map(|adapter| Arc::new(adapter) as Arc<dyn SourceAdapter>)
                    .map_err(|error| failure(source, "MCP_CONFIG_ERROR", error.message()))
            }
        }
        crate::config::SourceType::Api => {
            let adapter_kind = source.config_str("adapter").unwrap_or_default();
            let is_remoteok = adapter_kind == "remoteok"
                || source.id.to_ascii_lowercase().contains("remoteok")
                || source.name.to_ascii_lowercase().contains("remoteok");
            if is_remoteok {
                RemoteOkAdapter::from_config(source)
                    .map(|adapter| Arc::new(adapter) as Arc<dyn SourceAdapter>)
                    .map_err(|error| failure(source, "API_CONFIG_ERROR", error.message()))
            } else {
                Err(failure(
                    source,
                    "UNSUPPORTED_SOURCE",
                    format!("API adapter '{adapter_kind}' is not supported"),
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
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    filters: SearchFilters,
) -> Result<SearchMetrics, ErrorPayload> {
    let tracker = Arc::new(SearchProgressTracker::new(move |progress| {
        let _ = app.emit(SEARCH_PROGRESS_EVENT, progress);
    }));
    trigger(&state, filters, Some(tracker))
        .await
        .map_err(Into::into)
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
    fn ingestion_request_uses_the_seven_day_default() {
        let filters = SearchFilters::default().resolve(&profile());
        let request = filters.ingestion_request();
        assert_eq!(request.hours_old, 168);
        assert!(!request.is_remote);
        assert_eq!(request.location, "Brazil");
        assert!(request.query.contains("Rust Engineer"));
        assert_eq!(request.seniorities, filters.seniorities);
        assert_eq!(request.modalities, filters.modalities);
    }

    #[test]
    fn builds_one_portuguese_query_per_selected_seniority() {
        let filters = SearchFilters {
            titles: vec!["Rust Engineer".into()],
            seniorities: vec![Seniority::Trainee, Seniority::Junior, Seniority::Pleno],
            modalities: vec![WorkModality::Remote],
            ..SearchFilters::default()
        }
        .resolve(&profile());

        assert_eq!(
            filters.ingestion_queries(),
            [
                "Rust Engineer trainee (remoto)",
                "Rust Engineer júnior (remoto)",
                "Rust Engineer pleno (remoto)"
            ]
        );

        let request = filters.ingestion_request();
        assert_eq!(request.queries.len(), 3);
        assert_eq!(request.query, request.queries[0]);
    }

    #[test]
    fn is_remote_is_true_only_for_remote_only_selections() {
        let remote = SearchFilters {
            seniorities: vec![Seniority::Pleno],
            modalities: vec![WorkModality::Remote],
            ..SearchFilters::default()
        }
        .resolve(&profile());
        assert!(remote.ingestion_request().is_remote);

        let mixed = SearchFilters {
            seniorities: vec![Seniority::Pleno],
            modalities: vec![WorkModality::Remote, WorkModality::Hybrid],
            ..SearchFilters::default()
        }
        .resolve(&profile());
        assert!(!mixed.ingestion_request().is_remote);

        let onsite = SearchFilters {
            seniorities: vec![Seniority::Pleno],
            modalities: vec![WorkModality::Onsite],
            ..SearchFilters::default()
        }
        .resolve(&profile());
        let request = onsite.ingestion_request();
        assert!(!request.is_remote);
        assert!(
            request
                .queries
                .iter()
                .all(|query| query.contains("presencial"))
        );
    }

    #[test]
    fn appends_portuguese_work_type_terms_only_for_a_subset() {
        let mixed = SearchFilters {
            titles: vec!["Rust Engineer".into()],
            seniorities: vec![Seniority::Pleno],
            modalities: vec![WorkModality::Remote, WorkModality::Hybrid],
            ..SearchFilters::default()
        };
        assert_eq!(
            mixed.ingestion_queries(),
            ["Rust Engineer pleno (remoto OR híbrido)"]
        );

        let all = SearchFilters {
            titles: vec!["Rust Engineer".into()],
            seniorities: vec![Seniority::Pleno],
            modalities: vec![
                WorkModality::Remote,
                WorkModality::Hybrid,
                WorkModality::Onsite,
            ],
            ..SearchFilters::default()
        };
        assert_eq!(all.ingestion_queries(), ["Rust Engineer pleno"]);
    }

    #[test]
    fn criteria_mirrors_resolved_filters() {
        let filters = SearchFilters {
            seniorities: vec![Seniority::Trainee, Seniority::Pleno],
            modalities: vec![WorkModality::Remote, WorkModality::Hybrid],
            ..SearchFilters::default()
        }
        .resolve(&profile());

        let criteria = filters.criteria();
        assert_eq!(criteria.seniorities, filters.seniorities);
        assert_eq!(criteria.modalities, filters.modalities);
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

    #[test]
    fn a_skipped_fallback_keeps_its_last_run_status() {
        let ingestion = IngestionResult {
            total_ingested: 0,
            duplicates_merged: 0,
            failures: vec![SourceFailure {
                source_id: "linkedin".to_string(),
                source_name: "linkedin".to_string(),
                code: "SOURCE_TIMEOUT".to_string(),
                message: "source exceeded its time budget".to_string(),
            }],
            fallback_skipped: vec!["scraper".to_string()],
            adapters_built: true,
        };

        assert_eq!(source_status(&ingestion, "scraper"), None);
        assert_eq!(source_status(&ingestion, "linkedin"), Some("FAILED"));
        assert_eq!(source_status(&ingestion, "remoteok"), Some("SUCCESS"));
    }
}
