pub mod adapter;
pub mod api;
pub mod mcp;
pub mod normalize;

use std::sync::Arc;

use futures::future::join_all;
use tokio::sync::Semaphore;

use crate::ingestion::adapter::{IngestionRequest, RawJob, SourceAdapter, SourceFailure};

#[derive(Debug, Clone)]
pub struct SourceOutcome {
    pub source_id: String,
    pub source_name: String,
    pub jobs: Vec<RawJob>,
    pub failure: Option<SourceFailure>,
}

#[derive(Debug, Clone)]
pub struct IngestionOutcome {
    pub sources: Vec<SourceOutcome>,
}

impl IngestionOutcome {
    pub fn total_fetched(&self) -> usize {
        self.sources.iter().map(|source| source.jobs.len()).sum()
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

/// Fetch every enabled source concurrently with a small central call limit.
/// A failure is isolated to that source and never aborts the run.
pub async fn ingest(
    adapters: &[Arc<dyn SourceAdapter>],
    request: &IngestionRequest,
) -> IngestionOutcome {
    let concurrency = adapters.len().clamp(1, 4);
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let futures = adapters.iter().map(|adapter| {
        let semaphore = semaphore.clone();
        async move {
            let source_id = adapter.id().to_string();
            let source_name = adapter.name().to_string();
            let _permit = match semaphore.acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    return SourceOutcome {
                        source_id: source_id.clone(),
                        source_name: source_name.clone(),
                        jobs: Vec::new(),
                        failure: Some(SourceFailure {
                            source_id,
                            source_name,
                            code: "SOURCE_CANCELLED".to_string(),
                            message: "source call was cancelled".to_string(),
                        }),
                    };
                }
            };

            match adapter.fetch(request).await {
                Ok(jobs) => SourceOutcome {
                    source_id,
                    source_name,
                    jobs,
                    failure: None,
                },
                Err(error) => SourceOutcome {
                    source_id: source_id.clone(),
                    source_name: source_name.clone(),
                    jobs: Vec::new(),
                    failure: Some(SourceFailure {
                        source_id,
                        source_name,
                        code: error.code,
                        message: error.message,
                    }),
                },
            }
        }
    });

    IngestionOutcome {
        sources: join_all(futures).await,
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
