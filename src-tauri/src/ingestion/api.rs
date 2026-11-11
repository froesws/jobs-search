use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use zeroize::Zeroizing;

use crate::config::SourceConfig;
use crate::error::AppError;
use crate::ingestion::adapter::{IngestionRequest, RawJob, SourceAdapter, SourceError};

pub struct RemoteOkAdapter {
    id: String,
    name: String,
    base_url: String,
    client: Client,
    timeout: Duration,
    max_retries: u32,
    backoff: Duration,
    results_wanted: u32,
}

impl RemoteOkAdapter {
    pub fn from_config(source: &SourceConfig) -> Result<Self, AppError> {
        let base_url = source
            .config_str("base_url")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::Config(format!("api source {} requires config.base_url", source.id))
            })?;
        let timeout = Duration::from_secs(source.rate_limit.timeout_seconds.clamp(1, 300));
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("jobs-search/0.1")
            .build()
            .map_err(|_| AppError::Config("could not build the HTTP client".to_string()))?;

        Ok(Self {
            id: source.id.clone(),
            name: source.name.clone(),
            base_url: base_url.to_string(),
            client,
            timeout,
            max_retries: source.rate_limit.max_retries.min(5),
            backoff: Duration::from_millis(source.rate_limit.backoff_ms.clamp(10, 5_000)),
            results_wanted: source.config_u32("results_wanted", 50).clamp(1, 200),
        })
    }

    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        base_url: impl Into<String>,
        results_wanted: u32,
        max_retries: u32,
        backoff: Duration,
        timeout: Duration,
    ) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("jobs-search/0.1")
            .build()
            .map_err(|_| AppError::Config("could not build the HTTP client".to_string()))?;
        Ok(Self {
            id: id.into(),
            name: name.into(),
            base_url: base_url.into(),
            client,
            timeout,
            max_retries,
            backoff,
            results_wanted,
        })
    }
}

#[async_trait]
impl SourceAdapter for RemoteOkAdapter {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> &'static str {
        "api"
    }

    async fn fetch(&self, request: &IngestionRequest) -> Result<Vec<RawJob>, SourceError> {
        let mut last_error =
            SourceError::transient("API_REQUEST_FAILED", "RemoteOK request failed");

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                tokio::time::sleep(self.backoff * attempt).await;
            }

            let response = match self
                .client
                .get(&self.base_url)
                .timeout(self.timeout)
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    last_error = if error.is_timeout() {
                        SourceError::transient("API_TIMEOUT", "RemoteOK request timed out")
                    } else {
                        SourceError::transient("API_NETWORK", "RemoteOK request failed")
                    };
                    continue;
                }
            };

            let status = response.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                last_error = SourceError::transient(
                    "API_TRANSIENT",
                    format!("RemoteOK returned status {}", status.as_u16()),
                );
                continue;
            }
            if !status.is_success() {
                return Err(SourceError::permanent(
                    "API_ERROR",
                    format!("RemoteOK returned status {}", status.as_u16()),
                ));
            }

            let payload: Vec<RemoteOkJob> = response.json().await.map_err(|_| {
                SourceError::transient("API_INVALID_RESPONSE", "RemoteOK returned invalid JSON")
            })?;

            let query_terms = query_terms(&request.query, &request.stack_keyword);
            let mut jobs = Vec::new();
            for item in payload {
                let Some(position) = item.position.filter(|value| !value.trim().is_empty()) else {
                    continue;
                };
                let haystack = format!(
                    "{} {} {}",
                    position.to_ascii_lowercase(),
                    item.tags.join(" ").to_ascii_lowercase(),
                    item.description
                        .as_deref()
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                );
                if !query_terms.is_empty()
                    && !query_terms.iter().any(|term| haystack.contains(term))
                {
                    continue;
                }

                let url = item
                    .url
                    .or(item.apply_url)
                    .unwrap_or_else(|| format!("https://remoteok.com/remote-jobs/{}", item.slug));
                let external_id = item
                    .id
                    .and_then(RemoteOkJob::id_to_string)
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| crate::dedup::identity::sha256_hex(&url));
                let location = item
                    .location
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "Remote".to_string());
                let posted_at = item.epoch.and_then(|epoch| {
                    chrono::DateTime::from_timestamp(epoch, 0).map(|value| value.to_rfc3339())
                });

                jobs.push(RawJob {
                    external_id,
                    title: position,
                    company: item.company.unwrap_or_else(|| "Unknown".to_string()),
                    location,
                    work_modality: Some("Remote".to_string()),
                    url,
                    posted_at,
                    description: Zeroizing::new(item.description.unwrap_or_default()),
                    source_name: self.name.clone(),
                    tags: item.tags,
                });

                let cap = request.results_wanted.min(self.results_wanted) as usize;
                if jobs.len() >= cap.max(1) {
                    break;
                }
            }

            return Ok(jobs);
        }

        Err(last_error)
    }
}

fn query_terms(query: &str, stack_keyword: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for value in query.split(" OR ").chain(std::iter::once(stack_keyword)) {
        let term = value.trim().to_ascii_lowercase();
        if term.len() >= 2 && !matches!(term.as_str(), "or" | "and" | "the") {
            terms.push(term);
        }
    }
    terms.sort();
    terms.dedup();
    terms
}

#[derive(Debug, Deserialize)]
struct RemoteOkJob {
    #[serde(default)]
    id: Option<serde_json::Value>,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    company: Option<String>,
    #[serde(default)]
    position: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    epoch: Option<i64>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    apply_url: Option<String>,
}

impl RemoteOkJob {
    fn id_to_string(value: serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::String(value) => Some(value),
            serde_json::Value::Number(value) => Some(value.to_string()),
            _ => None,
        }
    }
}
