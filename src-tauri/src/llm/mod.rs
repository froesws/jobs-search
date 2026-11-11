pub mod deepseek;
pub mod gemini;
pub mod openrouter;
pub mod prompt;
pub mod provider;
pub mod schemas;
pub mod transport;

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;

use crate::config::AppConfig;
use crate::llm::deepseek::DeepSeekProvider;
use crate::llm::gemini::GeminiProvider;
use crate::llm::openrouter::OpenRouterProvider;
use crate::llm::provider::{LlmRouter, ProviderPlan};

/// Build the configured provider chain with per-provider attempt budgets:
/// Gemini 3, OpenRouter 3 (free model list), DeepSeek 2.
pub fn router_from_config(config: &AppConfig, client: Client) -> LlmRouter {
    let timeout = Duration::from_secs(config.llm_timeout_seconds);
    let mut plans = Vec::new();

    for provider in config.provider_order() {
        match provider.as_str() {
            "gemini" => {
                if let Some(key) = &config.gemini_api_key {
                    plans.push(ProviderPlan {
                        provider: Arc::new(GeminiProvider::new(
                            client.clone(),
                            config.gemini_base_url.clone(),
                            config.gemini_model.clone(),
                            key.clone(),
                            timeout,
                        )),
                        attempts: 3,
                    });
                }
            }
            "openrouter" => {
                if let Some(key) = &config.openrouter_api_key {
                    plans.push(ProviderPlan {
                        provider: Arc::new(OpenRouterProvider::new(
                            client.clone(),
                            config.openrouter_base_url.clone(),
                            config.openrouter_free_models.clone(),
                            key.clone(),
                            timeout,
                        )),
                        attempts: 3,
                    });
                }
            }
            "deepseek" => {
                if let Some(key) = &config.deepseek_api_key {
                    plans.push(ProviderPlan {
                        provider: Arc::new(DeepSeekProvider::new(
                            client.clone(),
                            config.deepseek_base_url.clone(),
                            "deepseek-chat",
                            key.clone(),
                            timeout,
                        )),
                        attempts: 2,
                    });
                }
            }
            _ => {}
        }
    }

    LlmRouter::new(plans, Duration::from_secs(config.llm_timeout_seconds + 5))
}
