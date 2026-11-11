//! Active LLM provider command.
#![allow(missing_docs)]

use serde::Serialize;
use tauri::State;

use crate::config::AppConfig;
use crate::error::{AppError, ErrorPayload};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ActiveProvider {
    pub provider: String,
    pub model: String,
    pub fallback_enabled: bool,
}

/// Return the first configured provider in `LLM_PROVIDER_ORDER` that has a key.
/// Never returns a key or any connection secret.
pub fn active_provider(config: &AppConfig) -> Result<ActiveProvider, AppError> {
    for provider in config.provider_order() {
        if !config.has_key(provider) {
            continue;
        }
        let model = match provider.as_str() {
            "gemini" => config.gemini_model.clone(),
            "openrouter" => config
                .openrouter_free_models
                .first()
                .cloned()
                .unwrap_or_else(|| "openrouter/auto".to_string()),
            "deepseek" => "deepseek-chat".to_string(),
            _ => continue,
        };
        return Ok(ActiveProvider {
            provider: provider.clone(),
            model,
            fallback_enabled: config.allow_fallback,
        });
    }

    Err(AppError::Config(
        "no LLM provider key is configured; set it in the local .env".to_string(),
    ))
}

#[tauri::command]
pub fn cmd_get_active_provider(state: State<'_, AppState>) -> Result<ActiveProvider, ErrorPayload> {
    active_provider(&state.config).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn config(values: &[(&str, &str)]) -> AppConfig {
        let map = values
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<std::collections::HashMap<_, _>>();
        AppConfig::from_lookup(
            move |key| map.get(key).cloned(),
            PathBuf::from("/tmp/app"),
            std::path::Path::new("/tmp"),
        )
        .expect("config")
    }

    #[test]
    fn returns_the_first_configured_provider_without_secrets() {
        let config = config(&[
            ("DEEPSEEK_API_KEY", "secret"),
            ("LLM_ALLOW_FALLBACK", "true"),
        ]);
        let active = active_provider(&config).expect("active");
        assert_eq!(active.provider, "deepseek");
        assert_eq!(active.model, "deepseek-chat");
        assert!(active.fallback_enabled);
        assert!(!format!("{active:?}").contains("secret"));
    }

    #[test]
    fn missing_keys_are_a_configuration_error() {
        let config = config(&[]);
        assert!(active_provider(&config).is_err());
    }
}
