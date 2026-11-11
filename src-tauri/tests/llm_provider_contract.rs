use std::sync::Arc;
use std::time::Duration;

use jobs_search_lib::llm::deepseek::DeepSeekProvider;
use jobs_search_lib::llm::gemini::GeminiProvider;
use jobs_search_lib::llm::ollama::OllamaProvider;
use jobs_search_lib::llm::openrouter::OpenRouterProvider;
use jobs_search_lib::llm::provider::{
    EvaluationInput, EvaluationJob, LlmProvider, LlmRouter, ProviderPlan,
};
use jobs_search_lib::profile::CandidateProfile;
use jobs_search_lib::types::WorkModality;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn profile() -> CandidateProfile {
    serde_json::from_str(include_str!("fixtures/profile.json")).expect("fixture")
}

fn input() -> EvaluationInput {
    EvaluationInput {
        profile: profile(),
        job: EvaluationJob {
            title: "Rust Engineer".to_string(),
            company: "Acme".to_string(),
            location: "Remote - Brazil".to_string(),
            work_modality: WorkModality::Remote,
            description_clean: "Rust and C++".to_string(),
        },
    }
}

fn evidence_json() -> serde_json::Value {
    json!({
        "seniority": "Pleno",
        "work_modality": "Remote",
        "location": "Remote - Brazil",
        "required_skills": ["Rust"],
        "desirable_skills": ["C++"],
        "requirements": [
            {
                "id": "r1",
                "text": "Rust",
                "dimension": "CORE_TECHNICAL",
                "match_type": "EXACT",
                "evidence": "projects"
            }
        ],
        "summary": "Strong fit"
    })
}

fn gemini_success() -> serde_json::Value {
    json!({
        "candidates": [{
            "content": { "parts": [{ "text": evidence_json().to_string() }] }
        }]
    })
}

fn openrouter_success() -> serde_json::Value {
    json!({
        "choices": [{
            "message": { "content": evidence_json().to_string() }
        }]
    })
}

fn test_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client")
}

fn router(server: &MockServer, gemini_timeout: Duration) -> LlmRouter {
    let client = test_client();
    LlmRouter::from_plans(vec![
        ProviderPlan {
            provider: Arc::new(GeminiProvider::new(
                client.clone(),
                server.uri(),
                "gemini-test",
                "gemini-key",
                gemini_timeout,
            )),
            attempts: 1,
        },
        ProviderPlan {
            provider: Arc::new(OpenRouterProvider::new(
                client,
                server.uri(),
                vec!["free/model:free".to_string()],
                "openrouter-key",
                Duration::from_secs(5),
            )),
            attempts: 1,
        },
    ])
}

#[tokio::test]
async fn rate_limit_falls_back_to_openrouter() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-test:generateContent"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openrouter_success()))
        .mount(&server)
        .await;

    let outcome = router(&server, Duration::from_secs(2))
        .evaluate(&input())
        .await
        .expect("fallback");
    assert_eq!(outcome.provider, "openrouter");
    assert_eq!(outcome.attempts.len(), 2);
    assert_eq!(
        outcome.attempts[0].error_category.as_deref(),
        Some("RATE_LIMIT")
    );
    assert_eq!(
        outcome.attempts[1].fallback_from.as_deref(),
        Some("gemini/gemini-test")
    );
}

#[tokio::test]
async fn malformed_output_falls_back() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-test:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{ "content": { "parts": [{ "text": "not json" }] } }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openrouter_success()))
        .mount(&server)
        .await;

    let outcome = router(&server, Duration::from_secs(2))
        .evaluate(&input())
        .await
        .expect("fallback");
    assert_eq!(outcome.provider, "openrouter");
    assert_eq!(outcome.attempts[0].error_category.as_deref(), Some("PARSE"));
}

#[tokio::test]
async fn timeout_falls_back() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-test:generateContent"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(300))
                .set_body_json(gemini_success()),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openrouter_success()))
        .mount(&server)
        .await;

    let outcome = router(&server, Duration::from_millis(50))
        .evaluate(&input())
        .await
        .expect("fallback");
    assert_eq!(outcome.provider, "openrouter");
    assert_eq!(
        outcome.attempts[0].error_category.as_deref(),
        Some("TIMEOUT")
    );
}

#[tokio::test]
async fn authentication_error_stops_without_fallback() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-test:generateContent"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openrouter_success()))
        .expect(0)
        .mount(&server)
        .await;

    let error = router(&server, Duration::from_secs(2))
        .evaluate(&input())
        .await
        .expect_err("fatal auth error");
    assert!(error.fatal);
    assert_eq!(error.category.as_str(), "AUTH");
    server.verify().await;
}

#[tokio::test]
async fn not_found_falls_back_to_the_next_provider() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-test:generateContent"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "free/model:free" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(openrouter_success()))
        .mount(&server)
        .await;

    let outcome = router(&server, Duration::from_secs(2))
        .evaluate(&input())
        .await
        .expect("fallback");
    assert_eq!(outcome.provider, "openrouter");
    assert_eq!(outcome.attempts.len(), 2);
    assert_eq!(
        outcome.attempts[0].error_category.as_deref(),
        Some("CONFIG")
    );
}

#[tokio::test]
async fn gemini_uses_the_versioned_path_and_omits_temperature() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-test:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(gemini_success()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(
        test_client(),
        format!("{}/", server.uri()),
        "models/gemini-test",
        "gemini-key",
        Duration::from_secs(5),
    );
    let outcome = provider.evaluate(&input(), 1).await.expect("evaluate");
    assert_eq!(outcome.seniority, "Pleno");

    let requests = server.received_requests().await.expect("requests");
    let body: serde_json::Value = requests[0].body_json().expect("body");
    assert!(body.pointer("/generationConfig/temperature").is_none());
    server.verify().await;
}

#[tokio::test]
async fn openrouter_uses_chat_completions_with_the_configured_model() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "free/model:free" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(openrouter_success()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenRouterProvider::new(
        test_client(),
        server.uri(),
        vec!["free/model:free".to_string()],
        "openrouter-key",
        Duration::from_secs(5),
    );
    let outcome = provider.evaluate(&input(), 1).await.expect("evaluate");
    assert_eq!(outcome.seniority, "Pleno");
    server.verify().await;
}

#[tokio::test]
async fn ollama_uses_the_openai_compatible_endpoint_without_a_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_partial_json(json!({ "model": "qwen2.5-coder" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(openrouter_success()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = OllamaProvider::new(
        test_client(),
        server.uri(),
        "qwen2.5-coder",
        Duration::from_secs(5),
    );
    let outcome = provider.evaluate(&input(), 1).await.expect("evaluate");
    assert_eq!(outcome.seniority, "Pleno");
    server.verify().await;
}

#[tokio::test]
async fn deepseek_uses_chat_completions_with_the_configured_model() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "deepseek-flash" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(openrouter_success()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = DeepSeekProvider::new(
        test_client(),
        server.uri(),
        "deepseek-flash",
        "deepseek-key",
        Duration::from_secs(5),
    );
    let outcome = provider.evaluate(&input(), 1).await.expect("evaluate");
    assert_eq!(outcome.seniority, "Pleno");
    server.verify().await;
}
