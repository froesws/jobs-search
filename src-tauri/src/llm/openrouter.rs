use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use crate::llm::prompt::{SYSTEM_PROMPT, user_prompt};
use crate::llm::provider::{EvaluationInput, LlmError, LlmProvider};
use crate::llm::schemas::{EvaluationEvidence, parse_llm_json};
use crate::llm::transport::{bearer_headers, chat_content, post_json};

pub struct OpenRouterProvider {
    client: Client,
    base_url: String,
    models: Vec<String>,
    api_key: String,
    timeout: Duration,
}

impl OpenRouterProvider {
    pub fn new(
        client: Client,
        base_url: impl Into<String>,
        models: Vec<String>,
        api_key: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            models,
            api_key: api_key.into(),
            timeout,
        }
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    fn provider(&self) -> &'static str {
        "openrouter"
    }

    fn model_for_attempt(&self, attempt: u32) -> String {
        if self.models.is_empty() {
            return "openrouter/auto".to_string();
        }
        let index = (attempt.saturating_sub(1) as usize) % self.models.len();
        self.models[index].clone()
    }

    async fn evaluate(
        &self,
        input: &EvaluationInput,
        attempt: u32,
    ) -> Result<EvaluationEvidence, LlmError> {
        let model = self.model_for_attempt(attempt);
        let url = format!("{}/chat/completions", self.base_url);
        let headers = bearer_headers(&self.api_key)?;
        let body = serde_json::json!({
            "model": model,
            "messages": [
                { "role": "system", "content": SYSTEM_PROMPT },
                { "role": "user", "content": user_prompt(input) }
            ],
            "temperature": 0,
            "response_format": { "type": "json_object" }
        });

        let response = post_json(&self.client, &url, headers, &body, self.timeout).await?;
        let text = chat_content(&response)?;
        parse_llm_json(&text).map_err(|error| LlmError::parse(error.message()))
    }
}
