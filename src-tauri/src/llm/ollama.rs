//! Ollama provider adapter.
//!
//! Ollama exposes an OpenAI-compatible chat endpoint at
//! `{base_url}/v1/chat/completions` and needs no API key, so this adapter
//! reuses the shared transport with an empty credential.
#![allow(missing_docs)]

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use crate::llm::prompt::{PROFILE_SYSTEM_PROMPT, SYSTEM_PROMPT, profile_user_prompt, user_prompt};
use crate::llm::provider::{EvaluationInput, LlmError, LlmProvider};
use crate::llm::schemas::{EvaluationEvidence, parse_extracted_profile, parse_llm_json};
use crate::llm::transport::chat_completion;
use crate::profile::schema::ExtractedProfile;

pub struct OllamaProvider {
    client: Client,
    base_url: String,
    model: String,
    timeout: Duration,
}

impl OllamaProvider {
    pub fn new(
        client: Client,
        base_url: impl Into<String>,
        model: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        let base = base_url.into();
        let base = base.trim_end_matches('/');
        let base = if base.ends_with("/v1") {
            base.to_string()
        } else {
            format!("{base}/v1")
        };
        Self {
            client,
            base_url: base,
            model: model.into(),
            timeout,
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn provider(&self) -> &'static str {
        "ollama"
    }

    fn model_for_attempt(&self, _attempt: u32) -> String {
        self.model.clone()
    }

    async fn evaluate(
        &self,
        input: &EvaluationInput,
        _attempt: u32,
    ) -> Result<EvaluationEvidence, LlmError> {
        let text = chat_completion(
            &self.client,
            &self.base_url,
            &self.model,
            "",
            SYSTEM_PROMPT,
            &user_prompt(input),
            self.timeout,
        )
        .await?;
        parse_llm_json(&text).map_err(|error| LlmError::parse(error.message()))
    }

    async fn extract_profile(
        &self,
        resume_text: &str,
        _attempt: u32,
    ) -> Result<ExtractedProfile, LlmError> {
        let text = chat_completion(
            &self.client,
            &self.base_url,
            &self.model,
            "",
            PROFILE_SYSTEM_PROMPT,
            &profile_user_prompt(resume_text),
            self.timeout,
        )
        .await?;
        parse_extracted_profile(&text).map_err(|error| LlmError::parse(error.message()))
    }
}
