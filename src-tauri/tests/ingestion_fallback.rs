use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use jobs_search_lib::ingestion::adapter::{
    FetchOutcome, IngestionRequest, KnownPosting, RawJob, SourceAdapter, SourceError,
};
use jobs_search_lib::ingestion::ingest;
use tokio::sync::{Notify, mpsc};
use zeroize::Zeroizing;

struct FakeAdapter {
    id: &'static str,
    fallback_for: Option<&'static str>,
    min_results: u32,
    jobs: u32,
    known: u32,
    fail: bool,
    calls: Arc<AtomicUsize>,
}

impl FakeAdapter {
    fn new(id: &'static str, jobs: u32) -> Self {
        Self {
            id,
            fallback_for: None,
            min_results: 1,
            jobs,
            known: 0,
            fail: false,
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn fallback_for(mut self, target: &'static str) -> Self {
        self.fallback_for = Some(target);
        self
    }

    fn min_results(mut self, min: u32) -> Self {
        self.min_results = min;
        self
    }

    fn failing(mut self) -> Self {
        self.fail = true;
        self
    }
}

fn raw_job(source: &str, index: u32) -> RawJob {
    RawJob {
        external_id: format!("{source}-{index}"),
        title: format!("Rust Engineer {index}"),
        company: "Acme".to_string(),
        location: "Remote - Brazil".to_string(),
        work_modality: Some("Remote".to_string()),
        url: format!("https://example.com/{source}/{index}"),
        posted_at: None,
        description: Zeroizing::new("Rust systems".to_string()),
        source_name: source.to_string(),
        tags: Vec::new(),
    }
}

#[async_trait]
impl SourceAdapter for FakeAdapter {
    fn id(&self) -> &str {
        self.id
    }

    fn name(&self) -> &str {
        self.id
    }

    fn source_type(&self) -> &'static str {
        "api"
    }

    fn fallback_for(&self) -> Option<&str> {
        self.fallback_for
    }

    fn fallback_min_results(&self) -> u32 {
        self.min_results
    }

    async fn fetch(&self, _request: &IngestionRequest) -> Result<FetchOutcome, SourceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(SourceError::permanent("SOURCE_FAILED", "primary failed"));
        }
        Ok(FetchOutcome {
            jobs: (0..self.jobs)
                .map(|index| raw_job(self.id, index))
                .collect(),
            listed: self.jobs + self.known,
            known_skipped: (0..self.known)
                .map(|index| format!("linkedin-{index}"))
                .collect(),
        })
    }
}

fn boxed(adapter: FakeAdapter) -> (Arc<dyn SourceAdapter>, Arc<AtomicUsize>) {
    let calls = adapter.calls.clone();
    (Arc::new(adapter), calls)
}

fn request_with_known(count: u32) -> IngestionRequest {
    IngestionRequest {
        known_postings: (0..count)
            .map(|index| KnownPosting {
                external_id: format!("linkedin-{index}"),
                job_id: format!("job-{index}"),
                url: format!("https://example.com/linkedin/{index}"),
            })
            .collect(),
        ..IngestionRequest::default()
    }
}

fn outcome_for<'a>(
    outcome: &'a jobs_search_lib::ingestion::IngestionOutcome,
    id: &str,
) -> &'a jobs_search_lib::ingestion::SourceOutcome {
    outcome
        .sources
        .iter()
        .find(|source| source.source_id == id)
        .expect("source outcome")
}

#[tokio::test]
async fn a_primary_that_delivers_enough_skips_its_fallback() {
    let (primary, primary_calls) = boxed(FakeAdapter::new("linkedin", 2));
    let (fallback, fallback_calls) = boxed(FakeAdapter::new("scraper", 3).fallback_for("linkedin"));
    let adapters = vec![primary, fallback];

    let outcome = ingest(&adapters, &IngestionRequest::default()).await;

    assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
    assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);
    assert!(outcome_for(&outcome, "scraper").fallback_skipped);
    assert!(outcome_for(&outcome, "scraper").failure.is_none());
    assert_eq!(outcome.total_listed(), 2);
}

#[tokio::test]
async fn an_empty_primary_triggers_the_fallback() {
    let (primary, _) = boxed(FakeAdapter::new("linkedin", 0));
    let (fallback, fallback_calls) = boxed(FakeAdapter::new("scraper", 2).fallback_for("linkedin"));

    let outcome = ingest(&[primary, fallback], &IngestionRequest::default()).await;

    assert_eq!(fallback_calls.load(Ordering::SeqCst), 1);
    assert!(!outcome_for(&outcome, "scraper").fallback_skipped);
    assert_eq!(outcome_for(&outcome, "scraper").jobs.len(), 2);
}

#[tokio::test]
async fn a_primary_below_the_minimum_merges_the_fallback() {
    let (primary, _) = boxed(FakeAdapter::new("linkedin", 1));
    let (fallback, fallback_calls) = boxed(
        FakeAdapter::new("scraper", 2)
            .fallback_for("linkedin")
            .min_results(2),
    );

    let outcome = ingest(&[primary, fallback], &IngestionRequest::default()).await;

    assert_eq!(fallback_calls.load(Ordering::SeqCst), 1);
    assert_eq!(outcome_for(&outcome, "linkedin").jobs.len(), 1);
    assert_eq!(outcome_for(&outcome, "scraper").jobs.len(), 2);
    assert_eq!(outcome.total_fetched(), 3);
}

#[tokio::test]
async fn a_failed_primary_triggers_the_fallback_and_both_failures_are_reported() {
    let (primary, _) = boxed(FakeAdapter::new("linkedin", 0).failing());
    let (fallback, fallback_calls) = boxed(FakeAdapter::new("scraper", 2).fallback_for("linkedin"));

    let outcome = ingest(&[primary, fallback], &IngestionRequest::default()).await;

    assert_eq!(fallback_calls.load(Ordering::SeqCst), 1);
    assert_eq!(outcome.failures().len(), 1);
    assert_eq!(outcome_for(&outcome, "scraper").jobs.len(), 2);

    let (primary, _) = boxed(FakeAdapter::new("linkedin", 0).failing());
    let (fallback, _) = boxed(
        FakeAdapter::new("scraper", 0)
            .fallback_for("linkedin")
            .failing(),
    );
    let outcome = ingest(&[primary, fallback], &IngestionRequest::default()).await;
    let failures = outcome.failures();
    assert_eq!(failures.len(), 2);
    assert!(
        failures
            .iter()
            .any(|failure| failure.source_id == "linkedin")
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.source_id == "scraper")
    );
}

#[tokio::test]
async fn a_fallback_with_a_disabled_primary_runs_as_a_normal_source() {
    let (fallback, fallback_calls) = boxed(FakeAdapter::new("scraper", 2).fallback_for("linkedin"));

    let outcome = ingest(&[fallback], &IngestionRequest::default()).await;

    assert_eq!(fallback_calls.load(Ordering::SeqCst), 1);
    assert!(!outcome_for(&outcome, "scraper").fallback_skipped);
    assert_eq!(outcome.total_fetched(), 2);
}

#[tokio::test]
async fn known_skipped_postings_count_as_delivered() {
    let mut primary = FakeAdapter::new("linkedin", 0);
    primary.known = 2;
    let (primary, _) = boxed(primary);
    let (fallback, fallback_calls) = boxed(FakeAdapter::new("scraper", 3).fallback_for("linkedin"));

    let outcome = ingest(&[primary, fallback], &request_with_known(2)).await;

    assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);
    assert!(outcome_for(&outcome, "scraper").fallback_skipped);
    assert_eq!(outcome_for(&outcome, "linkedin").known_skipped.len(), 2);
    assert_eq!(outcome_for(&outcome, "linkedin").delivered(), 2);
}

struct SlowAdapter {
    gate: Arc<Notify>,
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl SourceAdapter for SlowAdapter {
    fn id(&self) -> &str {
        "slow"
    }

    fn name(&self) -> &str {
        "slow"
    }

    fn source_type(&self) -> &'static str {
        "api"
    }

    async fn fetch(&self, _request: &IngestionRequest) -> Result<FetchOutcome, SourceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.gate.notified().await;
        Ok(FetchOutcome::from_jobs(vec![raw_job("slow", 0)]))
    }
}

struct SignallingFallback {
    started: mpsc::UnboundedSender<()>,
}

#[async_trait]
impl SourceAdapter for SignallingFallback {
    fn id(&self) -> &str {
        "scraper"
    }

    fn name(&self) -> &str {
        "scraper"
    }

    fn source_type(&self) -> &'static str {
        "api"
    }

    fn fallback_for(&self) -> Option<&str> {
        Some("linkedin")
    }

    async fn fetch(&self, _request: &IngestionRequest) -> Result<FetchOutcome, SourceError> {
        let _ = self.started.send(());
        Ok(FetchOutcome::from_jobs(vec![raw_job("scraper", 0)]))
    }
}

#[tokio::test]
async fn a_fallback_starts_before_an_unrelated_slow_primary_finishes() {
    let gate = Arc::new(Notify::new());
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let slow_calls = Arc::new(AtomicUsize::new(0));

    let adapters: Vec<Arc<dyn SourceAdapter>> = vec![
        Arc::new(FakeAdapter::new("linkedin", 0)),
        Arc::new(SignallingFallback {
            started: started_tx,
        }),
        Arc::new(SlowAdapter {
            gate: gate.clone(),
            calls: slow_calls.clone(),
        }),
    ];
    let request = IngestionRequest::default();

    let handle = tokio::spawn(async move { ingest(&adapters, &request).await });

    let started = tokio::time::timeout(Duration::from_secs(5), started_rx.recv()).await;
    assert!(
        started.is_ok_and(|value| value.is_some()),
        "the fallback must start while the unrelated primary is still running"
    );
    gate.notify_one();
    let outcome = handle.await.expect("join");
    assert_eq!(slow_calls.load(Ordering::SeqCst), 1);
    assert_eq!(outcome.total_fetched(), 2);
}
