use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use crate::llm::prompt::{SYSTEM_PROMPT, user_prompt};
use crate::llm::provider::{EvaluationInput, LlmError, LlmProvider};
use crate::llm::schemas::{EvaluationEvidence, parse_llm_json};
use crate::llm::transport::{api_key_headers, gemini_content, post_json};

pub struct GeminiProvider {
    client: Client,
    base_url: String,
    model: String,
    api_key: String,
    timeout: Duration,
}

impl GeminiProvider {
    pub fn new(
        client: Client,
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            api_key: api_key.into(),
            timeout,
        }
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn model_for_attempt(&self, _attempt: u32) -> String {
        self.model.clone()
    }

    async fn evaluate(
        &self,
        input: &EvaluationInput,
        _attempt: u32,
    ) -> Result<EvaluationEvidence, LlmError> {
        let url = format!("{}/models/{}:generateContent", self.base_url, self.model);
        let headers = api_key_headers("x-goog-api-key", &self.api_key)?;
        let body = serde_json::json!({
            "systemInstruction": {
                "parts": [{ "text": SYSTEM_PROMPT }]
            },
            "contents": [{
                "role": "user",
                "parts": [{ "text": user_prompt(input) }]
            }],
            "generationConfig": {
                "temperature": 0,
                "responseMimeType": "application/json"
            }
        });

        let response = post_json(&self.client, &url, headers, &body, self.timeout).await?;
        let text = gemini_content(&response)?;
        parse_llm_json(&text).map_err(|error| LlmError::parse(error.message()))
    }
}
