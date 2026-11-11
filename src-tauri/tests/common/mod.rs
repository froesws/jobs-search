#![allow(dead_code)]

use std::path::{Path, PathBuf};

use jobs_search_lib::config::{RateLimitConfig, SourceConfig, SourceType};
use jobs_search_lib::db::Db;
use jobs_search_lib::db::jobs::{NewJob, add_lineage, upsert_job};
use jobs_search_lib::dedup::canonical_identity;
use jobs_search_lib::profile::CandidateProfile;
use jobs_search_lib::telemetry::TelemetryLayer;
use jobs_search_lib::telemetry::health::HealthState;
use jobs_search_lib::telemetry::queue::BoundedQueue;
use jobs_search_lib::types::WorkModality;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;

pub struct TestSink {
    pub queue: BoundedQueue,
    pub health: HealthState,
}

pub fn test_sink() -> TestSink {
    let queue = BoundedQueue::new(1_000);
    TestSink {
        health: HealthState::new(queue.clone()),
        queue,
    }
}

pub fn with_telemetry<F: FnOnce()>(level: &str, sink: &TestSink, action: F) {
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new(level))
        .with(TelemetryLayer::new(sink.queue.clone(), sink.health.clone()));
    let dispatch = tracing::Dispatch::new(subscriber);
    tracing::dispatcher::with_default(&dispatch, action);
}

pub fn events(sink: &TestSink) -> Vec<serde_json::Value> {
    sink.queue
        .try_pop_batch(10_000)
        .into_iter()
        .map(|line| serde_json::from_slice(&line).expect("valid JSON event"))
        .collect()
}

pub fn test_db() -> Db {
    Db::open_in_memory().expect("in-memory database")
}

pub fn fixture_profile() -> CandidateProfile {
    serde_json::from_str(include_str!("../fixtures/profile.json")).expect("fixture profile")
}

pub fn ensure_source(db: &Db, id: &str, name: &str, source_type: SourceType) {
    let config = match source_type {
        SourceType::Mcp => serde_yaml::from_str("command: python3").expect("mcp config"),
        SourceType::Api => {
            serde_yaml::from_str("base_url: https://example.com/api").expect("api config")
        }
        SourceType::WebScraper => serde_yaml::from_str("{}").expect("scraper config"),
    };
    let source = SourceConfig {
        id: id.to_string(),
        name: name.to_string(),
        source_type,
        enabled: true,
        config,
        rate_limit: RateLimitConfig::default(),
        credential_ref: None,
    };
    jobs_search_lib::db::sources::upsert_source(db, &source).expect("source");
}

pub fn new_job(
    title: &str,
    company: &str,
    location: &str,
    modality: WorkModality,
    url: &str,
    description: &str,
) -> NewJob {
    let identity = canonical_identity(company, title, location, url);
    NewJob {
        canonical_key: identity.key,
        canonical_hash: identity.hash,
        title: title.to_string(),
        company: company.to_string(),
        location: location.to_string(),
        work_modality: modality,
        url: url.to_string(),
        posted_at: None,
        description_clean: description.to_string(),
        extracted_metadata_json: None,
    }
}

pub fn seed_job(db: &Db, source_id: &str, external_id: &str, job: &NewJob) -> String {
    let outcome = upsert_job(db, job).expect("upsert job");
    add_lineage(db, &outcome.job_id, source_id, external_id, &job.url).expect("lineage");
    outcome.job_id
}

pub fn write_fake_mcp_server(directory: &Path) -> PathBuf {
    let script = directory.join("fake_mcp.py");
    std::fs::write(
        &script,
        r#"
import json
import sys

for raw in sys.stdin:
    line = raw.strip()
    if not line:
        continue
    request = json.loads(line)
    if "id" not in request:
        continue
    if request["method"] == "initialize":
        response = {
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "serverInfo": {"name": "fake", "version": "0"},
            },
        }
    elif request["method"] == "tools/call":
        response = {
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": {
                "jobs": [
                    {
                        "title": "Rust Engineer",
                        "company": "Acme",
                        "location": "Remote - Brazil",
                        "work_modality": "Remote",
                        "url": "https://linkedin.example/1",
                        "posted_at": "2026-11-08T12:00:00Z",
                        "description": "<p>Rust systems</p>",
                        "source_name": "jobspy",
                        "external_id": "li-1",
                    }
                ]
            },
        }
    else:
        response = {
            "jsonrpc": "2.0",
            "id": request["id"],
            "error": {"code": -32601, "message": "unknown method"},
        }
    print(json.dumps(response), flush=True)
"#,
    )
    .expect("write fake MCP server");
    script
}
