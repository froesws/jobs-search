//! Provider model catalogs.
//!
//! Fetches the chat-capable models a provider currently exposes and filters out
//! models that cannot serve chat completions. The built-in catalog is the
//! offline fallback used before the first successful refresh.
#![allow(missing_docs)]

use std::time::Duration;

use reqwest::Client;
use reqwest::header::HeaderMap;
use serde_json::Value;

use crate::llm::provider::LlmError;
use crate::llm::transport::{bearer_headers, get_json};

/// A selectable model and its display price.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelOption {
    pub id: String,
    pub price: Option<String>,
}

/// Built-in catalog used before the first successful refresh.
pub fn builtin_models(provider: &str) -> Vec<String> {
    match provider {
        "gemini" => vec![
            "gemini-3.5-flash-lite".to_string(),
            "gemini-3.8-flash".to_string(),
        ],
        "openrouter" => vec!["openrouter/free".to_string()],
        "deepseek" => vec!["deepseek-flash".to_string()],
        "ollama" => Vec::new(),
        _ => Vec::new(),
    }
}

/// Fetch the chat-capable models for a provider.
pub async fn fetch_models(
    client: &Client,
    provider: &str,
    base_url: &str,
    api_key: Option<&str>,
    include_paid: bool,
    timeout: Duration,
) -> Result<Vec<ModelOption>, LlmError> {
    let base = base_url.trim_end_matches('/');
    match provider {
        "gemini" => {
            let key = api_key.ok_or_else(|| LlmError::auth("provider authentication failed"))?;
            let url = format!("{base}/models?key={key}");
            let value = get_json(client, &url, HeaderMap::new(), timeout).await?;
            Ok(gemini_models(&value))
        }
        "openrouter" => {
            let url = format!("{base}/models");
            let value = get_json(client, &url, HeaderMap::new(), timeout).await?;
            Ok(openrouter_models(&value, include_paid))
        }
        "deepseek" => {
            let key = api_key.ok_or_else(|| LlmError::auth("provider authentication failed"))?;
            let url = format!("{base}/models");
            let value = get_json(client, &url, bearer_headers(key)?, timeout).await?;
            Ok(openai_models(&value))
        }
        "ollama" => {
            let url = format!("{base}/api/tags");
            let value = get_json(client, &url, HeaderMap::new(), timeout).await?;
            Ok(ollama_models(&value))
        }
        _ => Err(LlmError::config("unknown provider")),
    }
}

/// Probe a provider with the same request used for model listing.
pub async fn probe(
    client: &Client,
    provider: &str,
    base_url: &str,
    api_key: Option<&str>,
    timeout: Duration,
) -> Result<(), LlmError> {
    let models = fetch_models(client, provider, base_url, api_key, true, timeout).await?;
    if models.is_empty() {
        return Err(LlmError::config("provider returned no chat models"));
    }
    Ok(())
}

fn gemini_models(value: &Value) -> Vec<ModelOption> {
    let mut models = Vec::new();
    let Some(entries) = value.get("models").and_then(Value::as_array) else {
        return models;
    };
    for entry in entries {
        let supports_chat = entry
            .get("supportedGenerationMethods")
            .and_then(Value::as_array)
            .map(|methods| {
                methods
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|method| method == "generateContent")
            })
            .unwrap_or(false);
        if !supports_chat {
            continue;
        }
        if let Some(name) = entry.get("name").and_then(Value::as_str) {
            let id = name.strip_prefix("models/").unwrap_or(name);
            models.push(ModelOption {
                id: id.to_string(),
                price: None,
            });
        }
    }
    dedup(models)
}

fn openrouter_models(value: &Value, include_paid: bool) -> Vec<ModelOption> {
    let mut models = Vec::new();
    let Some(entries) = value.get("data").and_then(Value::as_array) else {
        return models;
    };
    for entry in entries {
        let Some(id) = entry.get("id").and_then(Value::as_str) else {
            continue;
        };
        let prompt = entry
            .get("pricing")
            .and_then(|pricing| pricing.get("prompt"))
            .and_then(Value::as_str)
            .unwrap_or("0");
        let completion = entry
            .get("pricing")
            .and_then(|pricing| pricing.get("completion"))
            .and_then(Value::as_str)
            .unwrap_or("0");
        let is_free = is_zero(prompt) && is_zero(completion);
        if !include_paid && !is_free {
            continue;
        }
        models.push(ModelOption {
            id: id.to_string(),
            price: Some(format_price(prompt, completion)),
        });
    }
    dedup(models)
}

fn openai_models(value: &Value) -> Vec<ModelOption> {
    let mut models = Vec::new();
    let Some(entries) = value.get("data").and_then(Value::as_array) else {
        return models;
    };
    for entry in entries {
        if let Some(id) = entry.get("id").and_then(Value::as_str) {
            models.push(ModelOption {
                id: id.to_string(),
                price: None,
            });
        }
    }
    dedup(models)
}

fn ollama_models(value: &Value) -> Vec<ModelOption> {
    let mut models = Vec::new();
    let Some(entries) = value.get("models").and_then(Value::as_array) else {
        return models;
    };
    for entry in entries {
        if let Some(name) = entry.get("name").and_then(Value::as_str) {
            models.push(ModelOption {
                id: name.to_string(),
                price: None,
            });
        }
    }
    dedup(models)
}

fn dedup(mut models: Vec<ModelOption>) -> Vec<ModelOption> {
    models.sort_by(|left, right| left.id.cmp(&right.id));
    models.dedup_by(|left, right| left.id == right.id);
    models
}

fn is_zero(value: &str) -> bool {
    value
        .parse::<f64>()
        .map(|number| number == 0.0)
        .unwrap_or(false)
}

fn format_price(prompt: &str, completion: &str) -> String {
    if is_zero(prompt) && is_zero(completion) {
        return "free".to_string();
    }
    let per_million = |value: &str| {
        value
            .parse::<f64>()
            .map(|number| number * 1_000_000.0)
            .unwrap_or(0.0)
    };
    format!(
        "${:.2}/M in · ${:.2}/M out",
        per_million(prompt),
        per_million(completion)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn gemini_keeps_only_generate_content_models() {
        let value = json!({
            "models": [
                { "name": "models/gemini-3.5-flash-lite", "supportedGenerationMethods": ["generateContent"] },
                { "name": "models/text-embedding-004", "supportedGenerationMethods": ["embedContent"] }
            ]
        });
        let models = gemini_models(&value);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gemini-3.5-flash-lite");
    }

    #[test]
    fn openrouter_filters_paid_models_by_default() {
        let value = json!({
            "data": [
                { "id": "free/model", "pricing": { "prompt": "0", "completion": "0" } },
                { "id": "paid/model", "pricing": { "prompt": "0.000001", "completion": "0.000002" } }
            ]
        });
        let free = openrouter_models(&value, false);
        assert_eq!(free.len(), 1);
        assert_eq!(free[0].id, "free/model");
        assert_eq!(free[0].price.as_deref(), Some("free"));

        let all = openrouter_models(&value, true);
        assert_eq!(all.len(), 2);
        assert!(all[1].price.as_deref().unwrap().contains("/M"));
    }

    #[test]
    fn ollama_reads_tag_names() {
        let value = json!({ "models": [{ "name": "qwen2.5-coder:latest" }] });
        let models = ollama_models(&value);
        assert_eq!(models[0].id, "qwen2.5-coder:latest");
    }

    #[tokio::test]
    async fn gemini_fetch_uses_the_key_query_parameter() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .and(query_param("key", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": [{ "name": "models/gemini-x", "supportedGenerationMethods": ["generateContent"] }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let models = fetch_models(
            &Client::new(),
            "gemini",
            &server.uri(),
            Some("secret"),
            false,
            Duration::from_secs(5),
        )
        .await
        .expect("fetch");
        assert_eq!(models[0].id, "gemini-x");
        server.verify().await;
    }

    #[tokio::test]
    async fn deepseek_fetch_uses_a_bearer_token() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [{ "id": "deepseek-flash" }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let models = fetch_models(
            &Client::new(),
            "deepseek",
            &server.uri(),
            Some("secret"),
            false,
            Duration::from_secs(5),
        )
        .await
        .expect("fetch");
        assert_eq!(models[0].id, "deepseek-flash");
        server.verify().await;
    }

    #[tokio::test]
    async fn ollama_fetch_reads_api_tags() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": [{ "name": "llama3.2:latest" }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let models = fetch_models(
            &Client::new(),
            "ollama",
            &server.uri(),
            None,
            false,
            Duration::from_secs(5),
        )
        .await
        .expect("fetch");
        assert_eq!(models[0].id, "llama3.2:latest");
        server.verify().await;
    }

    #[tokio::test]
    async fn an_empty_filtered_catalog_is_a_failure() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": [] })))
            .mount(&server)
            .await;

        let error = probe(
            &Client::new(),
            "deepseek",
            &server.uri(),
            Some("secret"),
            Duration::from_secs(5),
        )
        .await
        .expect_err("empty");
        assert!(error.message.contains("no chat models"));
    }
}
