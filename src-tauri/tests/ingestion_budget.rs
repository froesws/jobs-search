use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use jobs_search_lib::ingestion::adapter::{
    FetchOutcome, IngestionRequest, RawJob, SourceAdapter, SourceError,
};
use jobs_search_lib::ingestion::ingest;
use zeroize::Zeroizing;

struct DropGuard(Arc<AtomicBool>);

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

struct NeverAdapter {
    dropped: Arc<AtomicBool>,
    budget: u64,
}

#[async_trait]
impl SourceAdapter for NeverAdapter {
    fn id(&self) -> &str {
        "slow"
    }

    fn name(&self) -> &str {
        "slow"
    }

    fn source_type(&self) -> &'static str {
        "api"
    }

    fn budget_seconds(&self) -> u64 {
        self.budget
    }

    async fn fetch(&self, _request: &IngestionRequest) -> Result<FetchOutcome, SourceError> {
        let _guard = DropGuard(self.dropped.clone());
        std::future::pending().await
    }
}

struct ReadyAdapter;

#[async_trait]
impl SourceAdapter for ReadyAdapter {
    fn id(&self) -> &str {
        "fast"
    }

    fn name(&self) -> &str {
        "fast"
    }

    fn source_type(&self) -> &'static str {
        "api"
    }

    async fn fetch(&self, _request: &IngestionRequest) -> Result<FetchOutcome, SourceError> {
        Ok(FetchOutcome::from_jobs(vec![RawJob {
            external_id: "fast-1".to_string(),
            title: "Rust Engineer".to_string(),
            company: "Acme".to_string(),
            location: "Remote - Brazil".to_string(),
            work_modality: Some("Remote".to_string()),
            url: "https://example.com/fast-1".to_string(),
            posted_at: None,
            description: Zeroizing::new("Rust systems".to_string()),
            source_name: "fast".to_string(),
            tags: Vec::new(),
        }]))
    }
}

#[tokio::test]
async fn a_source_that_exceeds_its_budget_is_reported_and_does_not_block_the_run() {
    let dropped = Arc::new(AtomicBool::new(false));
    let adapters: Vec<Arc<dyn SourceAdapter>> = vec![
        Arc::new(NeverAdapter {
            dropped: dropped.clone(),
            budget: 0,
        }),
        Arc::new(ReadyAdapter),
    ];

    let outcome = ingest(&adapters, &IngestionRequest::default()).await;

    assert_eq!(outcome.sources.len(), 2);
    let slow = outcome
        .sources
        .iter()
        .find(|source| source.source_id == "slow")
        .expect("slow source");
    let failure = slow.failure.as_ref().expect("timeout failure");
    assert_eq!(failure.code, "SOURCE_TIMEOUT");
    assert!(slow.jobs.is_empty());

    let fast = outcome
        .sources
        .iter()
        .find(|source| source.source_id == "fast")
        .expect("fast source");
    assert!(fast.failure.is_none());
    assert_eq!(fast.jobs.len(), 1);
    assert_eq!(outcome.total_fetched(), 1);
    assert_eq!(outcome.failures().len(), 1);

    assert!(
        dropped.load(Ordering::SeqCst),
        "the timed-out future must be dropped"
    );
}
