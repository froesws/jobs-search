use std::time::Duration;

use jobs_search_lib::ingestion::adapter::{IngestionRequest, RawJob, SourceAdapter};
use jobs_search_lib::ingestion::api::RemoteOkAdapter;
use jobs_search_lib::ingestion::ingest;
use jobs_search_lib::ingestion::mcp::McpAdapter;
use jobs_search_lib::ingestion::normalize;
use jobs_search_lib::types::WorkModality;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;

struct FailingAdapter;

#[async_trait::async_trait]
impl SourceAdapter for FailingAdapter {
    fn id(&self) -> &str {
        "broken"
    }

    fn name(&self) -> &str {
        "broken"
    }

    fn source_type(&self) -> &'static str {
        "api"
    }

    async fn fetch(
        &self,
        _request: &IngestionRequest,
    ) -> Result<Vec<RawJob>, jobs_search_lib::ingestion::adapter::SourceError> {
        Err(jobs_search_lib::ingestion::adapter::SourceError::permanent(
            "API_ERROR",
            "source is unavailable",
        ))
    }
}

#[tokio::test]
async fn hybrid_ingestion_dedups_and_preserves_lineage_with_one_failure() {
    let db = common::test_db();
    common::ensure_source(
        &db,
        "jobspy",
        "jobspy",
        jobs_search_lib::config::SourceType::Mcp,
    );
    common::ensure_source(
        &db,
        "remoteok",
        "remoteok",
        jobs_search_lib::config::SourceType::Api,
    );
    common::ensure_source(
        &db,
        "broken",
        "broken",
        jobs_search_lib::config::SourceType::Api,
    );

    let directory = tempfile::tempdir().expect("tempdir");
    let script = common::write_fake_mcp_server(directory.path());
    let mcp = McpAdapter::new(
        "jobspy",
        "jobspy",
        "python3",
        vec![script.display().to_string()],
        vec!["linkedin".to_string(), "indeed".to_string()],
        Duration::from_secs(5),
    );

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"legal": "notice"},
            {
                "id": 2,
                "slug": "rust-acme",
                "company": "Acme",
                "position": "Rust Engineer",
                "tags": ["rust"],
                "description": "<p>Rust and C++</p>",
                "location": "Remote - Brazil",
                "epoch": 1762600000,
                "url": "https://remoteok.example/2"
            }
        ])))
        .mount(&server)
        .await;

    let api = RemoteOkAdapter::new(
        "remoteok",
        "remoteok",
        format!("{}/api", server.uri()),
        50,
        1,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .expect("remoteok adapter");

    let adapters: Vec<std::sync::Arc<dyn SourceAdapter>> = vec![
        std::sync::Arc::new(mcp),
        std::sync::Arc::new(api),
        std::sync::Arc::new(FailingAdapter),
    ];
    let request = IngestionRequest {
        query: "Rust Engineer".to_string(),
        stack_keyword: "Rust".to_string(),
        ..IngestionRequest::default()
    };

    let outcome = ingest(&adapters, &request).await;
    assert_eq!(outcome.total_fetched(), 2);
    assert_eq!(outcome.failures().len(), 1);
    assert_eq!(outcome.failures()[0].source_id, "broken");

    let mut duplicates = 0;
    let mut canonical_job_id: Option<String> = None;
    for (source_id, source_name, raw) in outcome.jobs() {
        let job = normalize::normalize(raw, &source_id, &source_name).expect("normalized");
        let new_job = jobs_search_lib::db::jobs::NewJob {
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
        };
        let upsert = jobs_search_lib::db::jobs::upsert_job(&db, &new_job).expect("upsert");
        if upsert.duplicate() {
            duplicates += 1;
        }
        canonical_job_id.get_or_insert_with(|| upsert.job_id.clone());
        jobs_search_lib::db::jobs::add_lineage(
            &db,
            &upsert.job_id,
            &source_id,
            &job.external_id,
            &job.url,
        )
        .expect("lineage");
    }

    assert_eq!(duplicates, 1);
    assert_eq!(
        jobs_search_lib::db::jobs::count_jobs(&db).expect("count"),
        1
    );
    let job_id = canonical_job_id.expect("canonical job");
    assert_eq!(
        jobs_search_lib::db::jobs::lineage(&db, &job_id)
            .expect("lineage")
            .len(),
        2
    );
    assert_eq!(
        jobs_search_lib::db::jobs::lineage_urls(&db, &job_id)
            .expect("urls")
            .len(),
        2
    );

    // Repeat-safe: re-running the same ingestion creates no second job.
    let outcome = ingest(&adapters, &request).await;
    for (source_id, source_name, raw) in outcome.jobs() {
        let job = normalize::normalize(raw, &source_id, &source_name).expect("normalized");
        let new_job = jobs_search_lib::db::jobs::NewJob {
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
        };
        let upsert = jobs_search_lib::db::jobs::upsert_job(&db, &new_job).expect("upsert");
        jobs_search_lib::db::jobs::add_lineage(
            &db,
            &upsert.job_id,
            &source_id,
            &job.external_id,
            &job.url,
        )
        .expect("lineage");
    }
    assert_eq!(
        jobs_search_lib::db::jobs::count_jobs(&db).expect("count"),
        1
    );
    assert_eq!(
        jobs_search_lib::db::jobs::lineage(&db, &job_id)
            .expect("lineage")
            .len(),
        2
    );
    assert_eq!(
        jobs_search_lib::db::jobs::get_detail(&db, &job_id)
            .expect("detail")
            .summary
            .work_modality,
        WorkModality::Remote.as_str()
    );
}
