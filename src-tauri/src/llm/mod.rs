//! LLM provider assembly and routing.
#![allow(missing_docs)]

pub mod deepseek;
pub mod gemini;
pub mod models;
pub mod ollama;
pub mod openrouter;
pub mod prompt;
pub mod provider;
pub mod schemas;
pub mod transport;

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;

use crate::config::{AppConfig, ProviderSettings};
use crate::llm::deepseek::DeepSeekProvider;
use crate::llm::gemini::GeminiProvider;
use crate::llm::ollama::OllamaProvider;
use crate::llm::openrouter::OpenRouterProvider;
use crate::llm::provider::{LlmRouter, ProviderPlan};

/// Build the provider chain from the effective settings.
///
/// Providers without a resolvable credential are skipped. Ollama joins only
/// when it is enabled and has a model. Each plan keeps its configured attempt
/// budget; OpenRouter's budget is the length of its ordered model list.
pub fn router_from_config(
    config: &AppConfig,
    settings: &ProviderSettings,
    client: Client,
) -> LlmRouter {
    let timeout = Duration::from_secs(config.llm_timeout_seconds);
    let mut plans = Vec::new();

    for provider in &settings.order {
        let Some(setting) = settings.providers.get(provider) else {
            continue;
        };

        match provider.as_str() {
            "gemini" => {
                let (secret, _) =
                    crate::config::provider_settings::resolve_credential(config, "gemini");
                if let (Some(key), Some(model)) = (secret, setting.model.clone()) {
                    plans.push(ProviderPlan {
                        provider: Arc::new(GeminiProvider::new(
                            client.clone(),
                            config.gemini_base_url.clone(),
                            model,
                            key,
                            timeout,
                        )),
                        attempts: setting.attempts,
                    });
                }
            }
            "openrouter" => {
                let (secret, _) =
                    crate::config::provider_settings::resolve_credential(config, "openrouter");
                if let Some(key) = secret
                    && !setting.models.is_empty()
                {
                    plans.push(ProviderPlan {
                        provider: Arc::new(OpenRouterProvider::new(
                            client.clone(),
                            config.openrouter_base_url.clone(),
                            setting.models.clone(),
                            key,
                            timeout,
                        )),
                        attempts: setting.models.len() as u32,
                    });
                }
            }
            "deepseek" => {
                let (secret, _) =
                    crate::config::provider_settings::resolve_credential(config, "deepseek");
                if let (Some(key), Some(model)) = (secret, setting.model.clone()) {
                    plans.push(ProviderPlan {
                        provider: Arc::new(DeepSeekProvider::new(
                            client.clone(),
                            config.deepseek_base_url.clone(),
                            model,
                            key,
                            timeout,
                        )),
                        attempts: setting.attempts,
                    });
                }
            }
            "ollama" => {
                if !setting.enabled {
                    continue;
                }
                if let (Some(base_url), Some(model)) =
                    (setting.base_url.clone(), setting.model.clone())
                {
                    plans.push(ProviderPlan {
                        provider: Arc::new(OllamaProvider::new(
                            client.clone(),
                            base_url,
                            model,
                            timeout,
                        )),
                        attempts: setting.attempts,
                    });
                }
            }
            _ => {}
        }
    }

    let router = LlmRouter::new(plans, Duration::from_secs(config.llm_timeout_seconds + 5));
    if settings.fallback_enabled {
        router
    } else {
        router.without_fallback()
    }
}
