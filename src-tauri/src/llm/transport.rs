//! HTTP transport shared by the LLM providers.
#![allow(missing_docs)]

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, StatusCode};

use crate::llm::provider::{ErrorCategory, LlmError};

/// POST a JSON payload and return a JSON response. Error messages never contain
/// the request URL (which may carry credentials) or the response body.
pub async fn post_json(
    client: &Client,
    url: &str,
    headers: HeaderMap,
    body: &serde_json::Value,
    timeout: Duration,
) -> Result<serde_json::Value, LlmError> {
    let response = client
        .post(url)
        .headers(headers)
        .timeout(timeout)
        .json(body)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                LlmError::timeout("provider request timed out")
            } else {
                LlmError::network("provider request failed")
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(classify_status(status));
    }

    response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| LlmError::parse("provider returned invalid JSON"))
}

/// GET a JSON document. Error messages never contain the request URL (which may
/// carry credentials) or the response body.
pub async fn get_json(
    client: &Client,
    url: &str,
    headers: HeaderMap,
    timeout: Duration,
) -> Result<serde_json::Value, LlmError> {
    let response = client
        .get(url)
        .headers(headers)
        .timeout(timeout)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                LlmError::timeout("provider request timed out")
            } else {
                LlmError::network("provider request failed")
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(classify_status(status));
    }

    response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| LlmError::parse("provider returned invalid JSON"))
}

pub fn classify_status(status: StatusCode) -> LlmError {
    match status.as_u16() {
        401 | 403 => LlmError::auth("provider authentication failed"),
        404 => LlmError::new(
            ErrorCategory::Config,
            true,
            "provider endpoint or model not found; check the model ID and base URL",
        ),
        408 => LlmError::timeout("provider request timed out"),
        429 => LlmError::rate_limit("provider rate limit reached"),
        500..=599 => LlmError::network("provider is temporarily unavailable"),
        _ => LlmError::config(format!(
            "provider rejected the request with status {}",
            status.as_u16()
        )),
    }
}

pub fn bearer_headers(key: &str) -> Result<HeaderMap, LlmError> {
    let mut headers = HeaderMap::new();
    let value = HeaderValue::from_str(&format!("Bearer {key}"))
        .map_err(|_| LlmError::config("provider key contains invalid characters"))?;
    headers.insert(reqwest::header::AUTHORIZATION, value);
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    Ok(headers)
}

pub fn api_key_headers(name: &'static str, key: &str) -> Result<HeaderMap, LlmError> {
    let mut headers = HeaderMap::new();
    let value = HeaderValue::from_str(key)
        .map_err(|_| LlmError::config("provider key contains invalid characters"))?;
    headers.insert(name, value);
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    Ok(headers)
}

/// POST an OpenAI-compatible chat completion and return its message content.
pub async fn chat_completion(
    client: &Client,
    base_url: &str,
    model: &str,
    api_key: &str,
    system: &str,
    user: &str,
    timeout: Duration,
) -> Result<String, LlmError> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let headers = bearer_headers(api_key)?;
    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "temperature": 0,
        "response_format": { "type": "json_object" }
    });
    let response = post_json(client, &url, headers, &body, timeout).await?;
    chat_content(&response)
}

pub fn chat_content(value: &serde_json::Value) -> Result<String, LlmError> {
    value
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| LlmError::parse("provider response did not contain message content"))
}

pub fn gemini_content(value: &serde_json::Value) -> Result<String, LlmError> {
    value
        .get("candidates")
        .and_then(serde_json::Value::as_array)
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(serde_json::Value::as_array)
        .and_then(|parts| parts.first())
        .and_then(|part| part.get("text"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| LlmError::parse("provider response did not contain generated text"))
}

pub fn category_label(category: ErrorCategory) -> &'static str {
    category.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_transient_and_fatal_statuses() {
        assert!(classify_status(StatusCode::TOO_MANY_REQUESTS).retryable);
        assert!(classify_status(StatusCode::INTERNAL_SERVER_ERROR).retryable);
        assert!(!classify_status(StatusCode::UNAUTHORIZED).retryable);
        assert!(!classify_status(StatusCode::BAD_REQUEST).retryable);
    }

    #[test]
    fn not_found_is_a_retryable_configuration_error() {
        let error = classify_status(StatusCode::NOT_FOUND);
        assert_eq!(error.category, ErrorCategory::Config);
        assert!(error.retryable);
        assert!(error.message.contains("model"));
        assert!(error.message.contains("endpoint"));
    }

    #[tokio::test]
    async fn get_json_maps_errors_without_leaking_the_url() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = Client::new();
        let error = get_json(
            &client,
            &format!("{}/models?key=secret", server.uri()),
            HeaderMap::new(),
            Duration::from_secs(5),
        )
        .await
        .expect_err("unauthorized");

        assert_eq!(error.category, ErrorCategory::Auth);
        assert!(!error.message.contains("secret"));
        assert!(!error.message.contains(&server.uri()));
    }
}
