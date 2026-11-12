//! Concurrent multi-source ingestion.
#![allow(missing_docs)]

pub mod adapter;
pub mod api;
pub mod linkedin_mcp;
pub mod mcp;
pub mod normalize;

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use futures::future::join_all;

use crate::ingestion::adapter::{
    FetchOutcome, IngestionRequest, KnownPosting, RawJob, SourceAdapter, SourceError, SourceFailure,
};

#[derive(Debug, Clone, Default)]
pub struct SourceOutcome {
    pub source_id: String,
    pub source_name: String,
    pub jobs: Vec<RawJob>,
    /// Postings the source listed, including known ones whose detail was
    /// skipped.
    pub listed: u32,
    /// Known postings whose detail request was skipped.
    pub known_skipped: Vec<KnownPosting>,
    /// True for a configured fallback that did not need to run.
    pub fallback_skipped: bool,
    pub failure: Option<SourceFailure>,
}

impl SourceOutcome {
    /// Postings the source actually delivered: fetched jobs plus known
    /// postings it listed. This is the fallback trigger definition.
    pub fn delivered(&self) -> u32 {
        self.jobs.len() as u32 + self.known_skipped.len() as u32
    }
}

#[derive(Debug, Clone, Default)]
pub struct IngestionOutcome {
    pub sources: Vec<SourceOutcome>,
}

impl IngestionOutcome {
    /// New postings fetched in this run. Known skipped postings are not new
    /// ingestion.
    pub fn total_fetched(&self) -> usize {
        self.sources.iter().map(|source| source.jobs.len()).sum()
    }

    /// All postings listed by the sources, including known skipped ones.
    pub fn total_listed(&self) -> u32 {
        self.sources.iter().map(|source| source.listed).sum()
    }

    pub fn failures(&self) -> Vec<SourceFailure> {
        self.sources
            .iter()
            .filter_map(|source| source.failure.clone())
            .collect()
    }

    pub fn jobs(&self) -> Vec<(String, String, RawJob)> {
        let mut values = Vec::new();
        for source in &self.sources {
            for job in &source.jobs {
                values.push((
                    source.source_id.clone(),
                    source.source_name.clone(),
                    job.clone(),
                ));
            }
        }
        values
    }
}

/// Aggregate collection progress. Values are cumulative for the run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IngestionProgress {
    pub found: u32,
    pub failed: u32,
}

/// Receives one update each time a source finishes.
pub type IngestionProgressSink = Arc<dyn Fn(IngestionProgress) + Send + Sync>;

/// Fetch every enabled source concurrently with a small central call limit.
/// A failure is isolated to that source and never aborts the run.
#[tracing::instrument(skip(adapters), fields(sources = adapters.len()))]
pub async fn ingest(
    adapters: &[Arc<dyn SourceAdapter>],
    request: &IngestionRequest,
) -> IngestionOutcome {
    ingest_with_progress(adapters, request, None).await
}

/// Same as [`ingest`], reporting cumulative counts as each source finishes.
#[tracing::instrument(skip(adapters, progress), fields(sources = adapters.len()))]
pub async fn ingest_with_progress(
    adapters: &[Arc<dyn SourceAdapter>],
    request: &IngestionRequest,
    progress: Option<&IngestionProgressSink>,
) -> IngestionOutcome {
    let groups = fallback_groups(adapters);
    let found = Arc::new(AtomicU32::new(0));
    let failed = Arc::new(AtomicU32::new(0));

    let futures = groups.into_iter().map(|group| {
        let found = found.clone();
        let failed = failed.clone();
        async move {
            let mut outcomes = Vec::new();
            let primary = run_adapter(&adapters[group.primary], request).await;
            report_progress(progress, &found, &failed, &primary);
            let delivered = primary.delivered();
            let primary_failed = primary.failure.is_some();
            outcomes.push((group.primary, primary));

            for index in group.fallbacks {
                let adapter = &adapters[index];
                let required = adapter.fallback_min_results();
                if !primary_failed && delivered >= required {
                    // The primary delivered enough; the fallback stays idle
                    // and performs no request.
                    let outcome = SourceOutcome {
                        source_id: adapter.id().to_string(),
                        source_name: adapter.name().to_string(),
                        fallback_skipped: true,
                        ..SourceOutcome::default()
                    };
                    report_progress(progress, &found, &failed, &outcome);
                    outcomes.push((index, outcome));
                    continue;
                }
                // A failed fallback is never retried within the same run.
                let outcome = run_adapter(adapter, request).await;
                report_progress(progress, &found, &failed, &outcome);
                outcomes.push((index, outcome));
            }

            outcomes
        }
    });

    let mut indexed: Vec<(usize, SourceOutcome)> =
        join_all(futures).await.into_iter().flatten().collect();
    indexed.sort_by_key(|(index, _)| *index);
    IngestionOutcome {
        sources: indexed.into_iter().map(|(_, outcome)| outcome).collect(),
    }
}

struct FallbackGroup {
    primary: usize,
    fallbacks: Vec<usize>,
}

/// Split adapters into one group per primary, preserving input order.
///
/// A fallback joins the group of an adapter that is actually present. When its
/// configured primary is missing or disabled, the fallback runs as a normal
/// source and a configuration warning is logged instead of failing the run.
fn fallback_groups(adapters: &[Arc<dyn SourceAdapter>]) -> Vec<FallbackGroup> {
    let ids: Vec<&str> = adapters.iter().map(|adapter| adapter.id()).collect();
    let mut groups: Vec<FallbackGroup> = Vec::new();
    let mut primary_positions: Vec<Option<usize>> = vec![None; adapters.len()];

    for (index, adapter) in adapters.iter().enumerate() {
        let is_orphan_fallback = adapter.fallback_for().is_some_and(|target| {
            let present = ids.contains(&target);
            if !present {
                tracing::warn!(
                    source = %adapter.id(),
                    "fallback source has no enabled primary; running it as a normal source"
                );
            }
            !present
        });
        if adapter.fallback_for().is_none() || is_orphan_fallback {
            primary_positions[index] = Some(groups.len());
            groups.push(FallbackGroup {
                primary: index,
                fallbacks: Vec::new(),
            });
        }
    }

    for (index, adapter) in adapters.iter().enumerate() {
        let Some(target) = adapter.fallback_for() else {
            continue;
        };
        let Some(position) = ids
            .iter()
            .position(|id| *id == target)
            .and_then(|target_index| primary_positions[target_index])
        else {
            continue;
        };
        groups[position].fallbacks.push(index);
    }

    groups
}

async fn run_adapter(
    adapter: &Arc<dyn SourceAdapter>,
    request: &IngestionRequest,
) -> SourceOutcome {
    let source_id = adapter.id().to_string();
    let source_name = adapter.name().to_string();
    match fetch_with_budget(adapter.as_ref(), request, adapter.budget_seconds()).await {
        Ok(fetched) => {
            let known_skipped = fetched
                .known_skipped
                .iter()
                .filter_map(|external_id| {
                    request
                        .known_postings
                        .iter()
                        .find(|posting| &posting.external_id == external_id)
                        .cloned()
                })
                .collect();
            SourceOutcome {
                source_id,
                source_name,
                jobs: fetched.jobs,
                listed: fetched.listed,
                known_skipped,
                fallback_skipped: false,
                failure: None,
            }
        }
        Err(error) => SourceOutcome {
            source_id: source_id.clone(),
            source_name: source_name.clone(),
            failure: Some(SourceFailure {
                source_id,
                source_name,
                code: error.code,
                message: error.message,
            }),
            ..SourceOutcome::default()
        },
    }
}

fn report_progress(
    progress: Option<&IngestionProgressSink>,
    found: &AtomicU32,
    failed: &AtomicU32,
    outcome: &SourceOutcome,
) {
    let Some(sink) = progress else {
        return;
    };
    if outcome.failure.is_some() {
        failed.fetch_add(1, Ordering::Relaxed);
    } else {
        found.fetch_add(outcome.listed, Ordering::Relaxed);
    }
    sink(IngestionProgress {
        found: found.load(Ordering::Relaxed),
        failed: failed.load(Ordering::Relaxed),
    });
}

/// Fetch one source under its time budget. An elapsed budget becomes a
/// structured `SOURCE_TIMEOUT` failure and drops the in-flight future.
pub async fn fetch_with_budget(
    adapter: &dyn SourceAdapter,
    request: &IngestionRequest,
    seconds: u64,
) -> Result<FetchOutcome, SourceError> {
    match tokio::time::timeout(Duration::from_secs(seconds), adapter.fetch(request)).await {
        Ok(result) => result,
        Err(_) => Err(SourceError::transient(
            "SOURCE_TIMEOUT",
            "source exceeded its time budget",
        )),
    }
}

/// A minimal per-source pacer used by adapters that issue more than one call.
pub struct SourcePacer {
    interval: std::time::Duration,
    last: tokio::sync::Mutex<Option<tokio::time::Instant>>,
}

impl SourcePacer {
    pub fn new(requests_per_minute: u32) -> Self {
        let per_minute = requests_per_minute.max(1) as u64;
        Self {
            interval: std::time::Duration::from_millis(60_000 / per_minute),
            last: tokio::sync::Mutex::new(None),
        }
    }

    pub async fn wait(&self) {
        let mut last = self.last.lock().await;
        if let Some(previous) = *last {
            let elapsed = previous.elapsed();
            if elapsed < self.interval {
                tokio::time::sleep(self.interval - elapsed).await;
            }
        }
        *last = Some(tokio::time::Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    fn raw_job(external_id: &str) -> RawJob {
        RawJob {
            external_id: external_id.to_string(),
            title: "Rust Engineer".to_string(),
            company: "Acme".to_string(),
            location: "Remote".to_string(),
            work_modality: Some("Remote".to_string()),
            url: format!("https://example.com/{external_id}"),
            posted_at: None,
            description: zeroize::Zeroizing::new("Rust".to_string()),
            source_name: "test".to_string(),
            tags: Vec::new(),
        }
    }

    fn known(external_id: &str) -> KnownPosting {
        KnownPosting {
            external_id: external_id.to_string(),
            job_id: "job-1".to_string(),
            url: "https://example.com/1".to_string(),
        }
    }

    struct NeverAdapter;

    #[async_trait]
    impl SourceAdapter for NeverAdapter {
        fn id(&self) -> &str {
            "never"
        }

        fn name(&self) -> &str {
            "never"
        }

        fn source_type(&self) -> &'static str {
            "api"
        }

        async fn fetch(&self, _request: &IngestionRequest) -> Result<FetchOutcome, SourceError> {
            std::future::pending().await
        }
    }

    struct ReadyAdapter;

    #[async_trait]
    impl SourceAdapter for ReadyAdapter {
        fn id(&self) -> &str {
            "ready"
        }

        fn name(&self) -> &str {
            "ready"
        }

        fn source_type(&self) -> &'static str {
            "api"
        }

        async fn fetch(&self, _request: &IngestionRequest) -> Result<FetchOutcome, SourceError> {
            Ok(FetchOutcome::from_jobs(vec![raw_job("1")]))
        }
    }

    #[tokio::test]
    async fn a_never_resolving_source_times_out() {
        let error = fetch_with_budget(&NeverAdapter, &IngestionRequest::default(), 0)
            .await
            .expect_err("timeout");
        assert_eq!(error.code, "SOURCE_TIMEOUT");
        assert!(error.retryable);
    }

    #[tokio::test]
    async fn a_source_that_finishes_within_the_budget_returns_its_jobs() {
        let outcome = fetch_with_budget(&ReadyAdapter, &IngestionRequest::default(), 5)
            .await
            .expect("jobs");
        assert_eq!(outcome.jobs.len(), 1);
        assert_eq!(outcome.listed, 1);
    }

    #[test]
    fn source_outcome_counts_delivered_postings() {
        let outcome = SourceOutcome {
            source_id: "linkedin".to_string(),
            source_name: "linkedin".to_string(),
            jobs: vec![raw_job("1")],
            listed: 3,
            known_skipped: vec![known("linkedin-2"), known("linkedin-3")],
            fallback_skipped: false,
            failure: None,
        };
        assert_eq!(outcome.delivered(), 3);

        let ingestion = IngestionOutcome {
            sources: vec![
                outcome,
                SourceOutcome {
                    source_id: "scraper".to_string(),
                    source_name: "scraper".to_string(),
                    fallback_skipped: true,
                    ..SourceOutcome::default()
                },
            ],
        };
        assert_eq!(ingestion.total_fetched(), 1);
        assert_eq!(ingestion.total_listed(), 3);
        assert!(ingestion.failures().is_empty());
    }
}
