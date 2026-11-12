//! Source adapter contract and normalized payloads.
#![allow(missing_docs)]

use async_trait::async_trait;
use zeroize::Zeroizing;

use crate::types::{Seniority, WorkModality};

/// A source-native posting. `description` is memory-only and zeroized after
/// cleaning; it is never written to SQLite.
#[derive(Debug, Clone)]
pub struct RawJob {
    pub external_id: String,
    pub title: String,
    pub company: String,
    pub location: String,
    pub work_modality: Option<String>,
    pub url: String,
    pub posted_at: Option<String>,
    pub description: Zeroizing<String>,
    pub source_name: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl SourceError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }

    pub fn transient(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(code, message, true)
    }

    pub fn permanent(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(code, message, false)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SourceFailure {
    pub source_id: String,
    pub source_name: String,
    pub code: String,
    pub message: String,
}

/// A posting already stored with a description. The LinkedIn adapter uses the
/// set to skip detail requests; ingestion uses it to touch lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownPosting {
    pub external_id: String,
    pub job_id: String,
    pub url: String,
}

/// What one adapter fetch produced.
#[derive(Debug, Clone, Default)]
pub struct FetchOutcome {
    pub jobs: Vec<RawJob>,
    /// Number of postings the source listed, including known ones whose detail
    /// request was skipped.
    pub listed: u32,
    /// Known external IDs whose detail request was skipped.
    pub known_skipped: Vec<String>,
}

impl FetchOutcome {
    pub fn from_jobs(jobs: Vec<RawJob>) -> Self {
        Self {
            listed: jobs.len() as u32,
            jobs,
            known_skipped: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestionRequest {
    pub query: String,
    pub queries: Vec<String>,
    pub titles: Vec<String>,
    pub stack_keyword: String,
    pub location: String,
    pub sites: Vec<String>,
    pub results_wanted: u32,
    pub hours_old: u32,
    pub is_remote: bool,
    pub offset: u32,
    pub seniorities: Vec<Seniority>,
    pub modalities: Vec<WorkModality>,
    pub known_postings: Vec<KnownPosting>,
}

impl Default for IngestionRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            queries: Vec::new(),
            titles: Vec::new(),
            stack_keyword: String::new(),
            location: "Brazil".to_string(),
            sites: vec!["linkedin".to_string(), "indeed".to_string()],
            results_wanted: 50,
            hours_old: 168,
            is_remote: true,
            offset: 0,
            seniorities: Vec::new(),
            modalities: Vec::new(),
            known_postings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NormalizedJob {
    pub external_id: String,
    pub title: String,
    pub company: String,
    pub location: String,
    pub work_modality: WorkModality,
    pub url: String,
    pub posted_at: Option<String>,
    pub description_clean: String,
    pub extracted_metadata_json: Option<String>,
    pub source_id: String,
    pub source_name: String,
    pub canonical_key: String,
    pub canonical_hash: String,
    pub needs_review: bool,
}

#[async_trait]
pub trait SourceAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn source_type(&self) -> &'static str;

    /// Primary source this adapter falls back for, when configured.
    fn fallback_for(&self) -> Option<&str> {
        None
    }

    /// Minimum delivered postings before this fallback stays idle.
    fn fallback_min_results(&self) -> u32 {
        1
    }

    /// Maximum time this source may consume in one run.
    fn budget_seconds(&self) -> u64 {
        180
    }

    async fn fetch(&self, request: &IngestionRequest) -> Result<FetchOutcome, SourceError>;
}
