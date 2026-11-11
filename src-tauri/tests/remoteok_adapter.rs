use std::time::Duration;

use jobs_search_lib::config::RateLimitConfig;
use jobs_search_lib::ingestion::adapter::{IngestionRequest, SourceAdapter};
use jobs_search_lib::ingestion::api::RemoteOkAdapter;
use jobs_search_lib::types::WorkModality;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn request(query: &str) -> IngestionRequest {
    IngestionRequest {
        query: query.to_string(),
        stack_keyword: "Rust".to_string(),
        ..IngestionRequest::default()
    }
}

fn payload() -> serde_json::Value {
    serde_json::json!([
        {"legal": "RemoteOK terms"},
        {
            "id": 7,
            "slug": "rust-acme",
            "company": "Acme",
            "position": "Rust Engineer",
            "tags": ["rust", "backend"],
            "description": "<p>Rust and C++</p>",
            "location": "Worldwide",
            "epoch": 1762600000,
            "url": "https://remoteok.example/7"
        },
        {
            "id": 8,
            "slug": "sales",
            "company": "Sales Co",
            "position": "Sales Manager",
            "tags": ["sales"],
            "description": "commission only",
            "location": "USA",
            "epoch": 1762600000,
            "url": "https://remoteok.example/8"
        }
    ])
}

#[tokio::test]
async fn fetches_and_filters_remoteok_jobs() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(payload()))
        .mount(&server)
        .await;

    let adapter = RemoteOkAdapter::new(
        "remoteok",
        "remoteok",
        format!("{}/api", server.uri()),
        50,
        1,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .expect("adapter");

    let jobs = adapter.fetch(&request("Rust")).await.expect("jobs");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].title, "Rust Engineer");
    assert_eq!(jobs[0].company, "Acme");
    assert!(jobs[0].url.ends_with("/7"));
    assert_eq!(jobs[0].description.as_str(), "<p>Rust and C++</p>");

    let mut sales_request = request("Kubernetes");
    sales_request.stack_keyword = String::new();
    let jobs = adapter.fetch(&sales_request).await.expect("jobs");
    assert!(jobs.is_empty());
}

#[tokio::test]
async fn retries_transient_statuses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(payload()))
        .mount(&server)
        .await;

    let adapter = RemoteOkAdapter::new(
        "remoteok",
        "remoteok",
        format!("{}/api", server.uri()),
        50,
        2,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .expect("adapter");

    let jobs = adapter.fetch(&request("Rust")).await.expect("jobs");
    assert_eq!(jobs.len(), 1);
}

#[tokio::test]
async fn authentication_errors_are_permanent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let adapter = RemoteOkAdapter::new(
        "remoteok",
        "remoteok",
        format!("{}/api", server.uri()),
        50,
        2,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .expect("adapter");

    let error = adapter.fetch(&request("Rust")).await.expect_err("error");
    assert_eq!(error.code, "API_ERROR");
    assert!(!error.retryable);
}

#[tokio::test]
async fn retries_rate_limits() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(payload()))
        .mount(&server)
        .await;

    let adapter = RemoteOkAdapter::new(
        "remoteok",
        "remoteok",
        format!("{}/api", server.uri()),
        50,
        2,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .expect("adapter");

    let jobs = adapter.fetch(&request("Rust")).await.expect("jobs");
    assert_eq!(jobs.len(), 1);
}

#[tokio::test]
async fn timeout_is_reported_after_exhausted_retries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(300))
                .set_body_json(payload()),
        )
        .mount(&server)
        .await;

    let adapter = RemoteOkAdapter::new(
        "remoteok",
        "remoteok",
        format!("{}/api", server.uri()),
        50,
        0,
        Duration::from_millis(1),
        Duration::from_millis(50),
    )
    .expect("adapter");

    let error = adapter.fetch(&request("Rust")).await.expect_err("timeout");
    assert_eq!(error.code, "API_TIMEOUT");
    assert!(error.retryable);
}

#[tokio::test]
async fn exhausted_server_errors_are_reported() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let adapter = RemoteOkAdapter::new(
        "remoteok",
        "remoteok",
        format!("{}/api", server.uri()),
        50,
        1,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .expect("adapter");

    let error = adapter
        .fetch(&request("Rust"))
        .await
        .expect_err("exhausted");
    assert_eq!(error.code, "API_TRANSIENT");
    assert!(error.retryable);
}

#[test]
fn default_source_timeout_is_fifteen_seconds() {
    assert_eq!(RateLimitConfig::default().timeout_seconds, 15);
}

#[tokio::test]
async fn skips_remoteok_when_remote_is_not_selected() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(payload()))
        .expect(0)
        .mount(&server)
        .await;

    let adapter = RemoteOkAdapter::new(
        "remoteok",
        "remoteok",
        format!("{}/api", server.uri()),
        50,
        1,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .expect("adapter");
    let request = IngestionRequest {
        query: "Rust".to_string(),
        stack_keyword: "Rust".to_string(),
        modalities: vec![WorkModality::Hybrid],
        ..IngestionRequest::default()
    };

    let jobs = adapter.fetch(&request).await.expect("jobs");
    assert!(jobs.is_empty());
    server.verify().await;
}

#[tokio::test]
async fn uses_title_terms_instead_of_full_query_variants() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(payload()))
        .mount(&server)
        .await;

    let adapter = RemoteOkAdapter::new(
        "remoteok",
        "remoteok",
        format!("{}/api", server.uri()),
        50,
        1,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .expect("adapter");
    let request = IngestionRequest {
        query: "Rust Engineer pleno (remoto)".to_string(),
        queries: vec!["Rust Engineer pleno (remoto)".to_string()],
        titles: vec!["Rust Engineer".to_string()],
        stack_keyword: "Rust".to_string(),
        modalities: vec![WorkModality::Remote],
        ..IngestionRequest::default()
    };

    let jobs = adapter.fetch(&request).await.expect("jobs");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].title, "Rust Engineer");
}
