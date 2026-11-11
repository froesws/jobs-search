//! Gemini provider adapter.
#![allow(missing_docs)]

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use crate::llm::prompt::{PROFILE_SYSTEM_PROMPT, SYSTEM_PROMPT, profile_user_prompt, user_prompt};
use crate::llm::provider::{EvaluationInput, LlmError, LlmProvider};
use crate::llm::schemas::{EvaluationEvidence, parse_extracted_profile, parse_llm_json};
use crate::llm::transport::{api_key_headers, gemini_content, post_json};
use crate::profile::schema::ExtractedProfile;

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

    async fn extract_profile(
        &self,
        resume_text: &str,
        _attempt: u32,
    ) -> Result<ExtractedProfile, LlmError> {
        let url = format!("{}/models/{}:generateContent", self.base_url, self.model);
        let headers = api_key_headers("x-goog-api-key", &self.api_key)?;
        let body = serde_json::json!({
            "systemInstruction": {
                "parts": [{ "text": PROFILE_SYSTEM_PROMPT }]
            },
            "contents": [{
                "role": "user",
                "parts": [{ "text": profile_user_prompt(resume_text) }]
            }],
            "generationConfig": {
                "temperature": 0,
                "responseMimeType": "application/json"
            }
        });

        let response = post_json(&self.client, &url, headers, &body, self.timeout).await?;
        let text = gemini_content(&response)?;
        parse_extracted_profile(&text).map_err(|error| LlmError::parse(error.message()))
    }
}
