//! DeepSeek provider adapter.
#![allow(missing_docs)]

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use crate::llm::prompt::{PROFILE_SYSTEM_PROMPT, SYSTEM_PROMPT, profile_user_prompt, user_prompt};
use crate::llm::provider::{EvaluationInput, LlmError, LlmProvider};
use crate::llm::schemas::{EvaluationEvidence, parse_extracted_profile, parse_llm_json};
use crate::llm::transport::chat_completion;
use crate::profile::schema::ExtractedProfile;

pub struct DeepSeekProvider {
    client: Client,
    base_url: String,
    model: String,
    api_key: String,
    timeout: Duration,
}

impl DeepSeekProvider {
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
impl LlmProvider for DeepSeekProvider {
    fn provider(&self) -> &'static str {
        "deepseek"
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
            &self.api_key,
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
            &self.api_key,
            PROFILE_SYSTEM_PROMPT,
            &profile_user_prompt(resume_text),
            self.timeout,
        )
        .await?;
        parse_extracted_profile(&text).map_err(|error| LlmError::parse(error.message()))
    }
}
