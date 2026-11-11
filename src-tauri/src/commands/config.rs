//! Provider configuration commands.
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::config::{AppConfig, ProviderSettings, SessionSelection};
use crate::credentials;
use crate::db::settings;
use crate::error::{AppError, ErrorPayload};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ActiveProvider {
    pub provider: String,
    pub model: String,
    pub fallback_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_only: Option<bool>,
}

/// Return the first eligible provider in the effective chain.
/// Never returns a key or any connection secret.
pub fn active_provider(
    config: &AppConfig,
    settings: &ProviderSettings,
    session: Option<&SessionSelection>,
) -> Result<ActiveProvider, AppError> {
    let Some(provider) = settings.preferred(config) else {
        return Err(AppError::Config(
            "no LLM provider is configured; open the provider settings to add a key".to_string(),
        ));
    };
    let setting = settings
        .providers
        .get(&provider)
        .ok_or_else(|| AppError::Config("provider settings are missing".to_string()))?;
    let model = setting
        .attempt_models()
        .first()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    Ok(ActiveProvider {
        provider: provider.clone(),
        model,
        fallback_enabled: settings.fallback_enabled,
        session_only: session
            .filter(|selection| selection.provider == provider)
            .map(|_| true),
    })
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettingView {
    pub id: String,
    pub label: String,
    pub requires_api_key: bool,
    pub has_credential: bool,
    pub credential_source: String,
    pub model: Option<String>,
    pub models: Option<Vec<String>>,
    pub base_url: Option<String>,
    pub attempts: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionSelectionView {
    pub provider: String,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigView {
    pub providers: Vec<ProviderSettingView>,
    pub order: Vec<String>,
    pub fallback_enabled: bool,
    pub session_override: Option<SessionSelectionView>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettingInput {
    pub id: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub models: Option<Vec<String>>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub attempts: Option<u32>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigInput {
    pub order: Vec<String>,
    pub fallback_enabled: bool,
    pub providers: Vec<ProviderSettingInput>,
}

fn label(provider: &str) -> &'static str {
    match provider {
        "gemini" => "Gemini",
        "openrouter" => "OpenRouter",
        "deepseek" => "DeepSeek",
        "ollama" => "Ollama (local)",
        _ => "Unknown",
    }
}

fn build_view(state: &AppState) -> Result<ProviderConfigView, AppError> {
    let config = state.config()?;
    let session = state.session_selection()?;
    let settings = ProviderSettings::resolve(&state.db, &config, session.as_ref())?;

    let mut providers = Vec::new();
    for provider in crate::config::provider_settings::PROVIDERS {
        let setting = settings
            .providers
            .get(provider)
            .ok_or_else(|| AppError::Config("provider settings are missing".to_string()))?;
        let (secret, source) = credentials::resolve(provider, config.env_file.as_deref());
        providers.push(ProviderSettingView {
            id: provider.to_string(),
            label: label(provider).to_string(),
            requires_api_key: provider != "ollama",
            has_credential: secret.is_some(),
            credential_source: source.as_str().to_string(),
            model: setting.model.clone(),
            models: if provider == "openrouter" {
                Some(setting.models.clone())
            } else {
                None
            },
            base_url: setting.base_url.clone(),
            attempts: setting.attempts,
            enabled: setting.enabled,
        });
    }

    Ok(ProviderConfigView {
        providers,
        order: settings.order,
        fallback_enabled: settings.fallback_enabled,
        session_override: session.map(|selection| SessionSelectionView {
            provider: selection.provider,
            models: selection.models,
        }),
    })
}

fn validate_provider_id(provider: &str) -> Result<(), AppError> {
    if crate::config::provider_settings::PROVIDERS.contains(&provider) {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "unknown provider: {provider}"
        )))
    }
}

fn validate_order(order: &[String]) -> Result<(), AppError> {
    if order.is_empty() {
        return Err(AppError::validation("the provider order cannot be empty"));
    }
    let mut seen = Vec::new();
    for provider in order {
        validate_provider_id(provider)?;
        if seen.contains(provider) {
            return Err(AppError::validation(format!(
                "duplicate provider in the order: {provider}"
            )));
        }
        seen.push(provider.clone());
    }
    Ok(())
}

fn validate_model(model: &str) -> Result<(), AppError> {
    if model.trim().is_empty() {
        return Err(AppError::validation("the model cannot be empty"));
    }
    if model.chars().count() > 200 {
        return Err(AppError::validation("the model name is too long"));
    }
    Ok(())
}

fn validate_base_url(value: &str) -> Result<(), AppError> {
    let parsed = reqwest::Url::parse(value)
        .map_err(|_| AppError::validation("the base URL is not a valid URL"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::validation("the base URL must use http or https"));
    }
    Ok(())
}

fn validate_input(input: &ProviderConfigInput) -> Result<(), AppError> {
    validate_order(&input.order)?;
    for provider in &input.providers {
        validate_provider_id(&provider.id)?;
        if provider.model.is_some() && provider.models.is_some() {
            return Err(AppError::validation(
                "a provider cannot have both a model and a model list",
            ));
        }
        if let Some(model) = &provider.model {
            validate_model(model)?;
        }
        if let Some(models) = &provider.models {
            if provider.id != "openrouter" {
                return Err(AppError::validation(
                    "only OpenRouter accepts an ordered model list",
                ));
            }
            if models.is_empty() || models.len() > 10 {
                return Err(AppError::validation(
                    "the OpenRouter model list must contain 1 to 10 models",
                ));
            }
            for model in models {
                validate_model(model)?;
            }
        }
        if let Some(attempts) = provider.attempts {
            if provider.id == "openrouter" {
                return Err(AppError::validation(
                    "OpenRouter attempts are derived from the model list",
                ));
            }
            if !(1..=10).contains(&attempts) {
                return Err(AppError::validation("attempts must be between 1 and 10"));
            }
        }
        if let Some(base_url) = &provider.base_url {
            if provider.id != "ollama" {
                return Err(AppError::validation(
                    "only Ollama accepts a custom base URL",
                ));
            }
            validate_base_url(base_url)?;
        }
    }
    Ok(())
}

fn persist_input(state: &AppState, input: &ProviderConfigInput) -> Result<(), AppError> {
    let order = serde_json::to_string(&input.order)?;
    settings::put(&state.db, "provider.order", &order)?;
    settings::put(
        &state.db,
        "provider.fallback_enabled",
        if input.fallback_enabled {
            "true"
        } else {
            "false"
        },
    )?;

    for provider in &input.providers {
        let prefix = format!("provider.{}", provider.id);
        if let Some(model) = &provider.model {
            settings::put(&state.db, &format!("{prefix}.model"), model)?;
        }
        if let Some(models) = &provider.models {
            let value = serde_json::to_string(models)?;
            settings::put(&state.db, &format!("{prefix}.models"), &value)?;
        }
        if let Some(base_url) = &provider.base_url {
            settings::put(&state.db, &format!("{prefix}.base_url"), base_url)?;
        }
        if let Some(attempts) = provider.attempts {
            settings::put(
                &state.db,
                &format!("{prefix}.attempts"),
                &attempts.to_string(),
            )?;
        }
        if let Some(enabled) = provider.enabled {
            settings::put(
                &state.db,
                &format!("{prefix}.enabled"),
                if enabled { "true" } else { "false" },
            )?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn cmd_get_active_provider(state: State<'_, AppState>) -> Result<ActiveProvider, ErrorPayload> {
    let config = state.config()?;
    let session = state.session_selection()?;
    let settings = ProviderSettings::resolve(&state.db, &config, session.as_ref())?;
    active_provider(&config, &settings, session.as_ref()).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_get_provider_config(
    state: State<'_, AppState>,
) -> Result<ProviderConfigView, ErrorPayload> {
    build_view(&state).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_save_provider_config(
    state: State<'_, AppState>,
    config: ProviderConfigInput,
) -> Result<ProviderConfigView, ErrorPayload> {
    validate_input(&config)?;
    persist_input(&state, &config)?;
    state.set_session_selection(None)?;
    build_view(&state).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_save_provider_credential(
    state: State<'_, AppState>,
    provider: String,
    api_key: String,
    allow_env_fallback: Option<bool>,
) -> Result<ProviderConfigView, ErrorPayload> {
    validate_provider_id(&provider)?;
    if provider == "ollama" {
        return Err(AppError::validation("Ollama does not use an API key").into());
    }
    if api_key.trim().is_empty() {
        return Err(AppError::validation("the API key cannot be empty").into());
    }

    match credentials::set_in_store(&provider, &api_key) {
        Ok(()) => {}
        Err(AppError::CredentialStoreUnavailable) if allow_env_fallback == Some(true) => {
            let config = state.config()?;
            let env_file = config
                .env_file
                .clone()
                .ok_or_else(|| AppError::validation("no .env file was found to store the key"))?;
            let key = credentials::env_key(&provider)
                .ok_or_else(|| AppError::validation("unknown provider"))?;
            credentials::write_env_value(&env_file, key, &api_key)?;
        }
        Err(error) => return Err(error.into()),
    }

    build_view(&state).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_remove_provider_credential(
    state: State<'_, AppState>,
    provider: String,
    remove_env_entry: Option<bool>,
) -> Result<ProviderConfigView, ErrorPayload> {
    validate_provider_id(&provider)?;
    credentials::delete_from_store(&provider)?;

    if remove_env_entry == Some(true)
        && let Some(key) = credentials::env_key(&provider)
    {
        let config = state.config()?;
        if let Some(env_file) = config.env_file.as_deref() {
            credentials::remove_env_value(env_file, key)?;
        }
    }

    if let Some(selection) = state.session_selection()?
        && selection.provider == provider
    {
        state.set_session_selection(None)?;
    }

    build_view(&state).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_reset_provider_setting(
    state: State<'_, AppState>,
    provider: String,
    field: String,
) -> Result<ProviderConfigView, ErrorPayload> {
    let key = match field.as_str() {
        "order" => "provider.order".to_string(),
        "fallbackEnabled" => "provider.fallback_enabled".to_string(),
        "model" | "models" | "baseUrl" | "attempts" | "enabled" => {
            validate_provider_id(&provider)?;
            let suffix = match field.as_str() {
                "baseUrl" => "base_url",
                other => other,
            };
            format!("provider.{provider}.{suffix}")
        }
        _ => return Err(AppError::validation("unknown setting field").into()),
    };
    settings::delete(&state.db, &key)?;
    build_view(&state).map_err(Into::into)
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
        let settings =
            ProviderSettings::resolve(&crate::db::Db::open_in_memory().expect("db"), &config, None)
                .expect("settings");
        let active = active_provider(&config, &settings, None).expect("active");
        assert_eq!(active.provider, "deepseek");
        assert_eq!(active.model, "deepseek-flash");
        assert!(active.fallback_enabled);
        assert!(!format!("{active:?}").contains("secret"));
    }

    #[test]
    fn respects_the_selected_provider() {
        let config = config(&[
            ("LLM_PROVIDER", "openrouter"),
            ("OPENROUTER_API_KEY", "openrouter-secret"),
            ("GEMINI_API_KEY", "gemini-secret"),
        ]);
        let settings =
            ProviderSettings::resolve(&crate::db::Db::open_in_memory().expect("db"), &config, None)
                .expect("settings");
        let active = active_provider(&config, &settings, None).expect("active");
        assert_eq!(active.provider, "openrouter");
        assert_eq!(active.model, "openrouter/free");
    }

    #[test]
    fn uses_the_configured_deepseek_model() {
        let config = config(&[
            ("LLM_PROVIDER", "deepseek"),
            ("DEEPSEEK_API_KEY", "secret"),
            ("DEEPSEEK_MODEL", "deepseek-v4-pro"),
        ]);
        let settings =
            ProviderSettings::resolve(&crate::db::Db::open_in_memory().expect("db"), &config, None)
                .expect("settings");
        let active = active_provider(&config, &settings, None).expect("active");
        assert_eq!(active.provider, "deepseek");
        assert_eq!(active.model, "deepseek-v4-pro");
    }

    #[test]
    fn missing_keys_are_a_configuration_error() {
        let config = config(&[]);
        let settings =
            ProviderSettings::resolve(&crate::db::Db::open_in_memory().expect("db"), &config, None)
                .expect("settings");
        let error = active_provider(&config, &settings, None).expect_err("missing keys");
        assert!(error.message().contains("provider settings"));
    }

    #[test]
    fn session_selection_is_reported_as_session_only() {
        let config = config(&[("GEMINI_API_KEY", "secret")]);
        let db = crate::db::Db::open_in_memory().expect("db");
        let session = SessionSelection {
            provider: "gemini".to_string(),
            models: vec!["gemini-custom".to_string()],
        };
        let settings = ProviderSettings::resolve(&db, &config, Some(&session)).expect("settings");
        let active = active_provider(&config, &settings, Some(&session)).expect("active");
        assert_eq!(active.model, "gemini-custom");
        assert_eq!(active.session_only, Some(true));
    }

    #[test]
    fn validation_rejects_bad_input() {
        let base = ProviderConfigInput {
            order: vec!["gemini".to_string()],
            fallback_enabled: false,
            providers: Vec::new(),
        };
        assert!(validate_input(&base).is_ok());

        let empty_order = ProviderConfigInput {
            order: Vec::new(),
            ..base.clone()
        };
        assert!(validate_input(&empty_order).is_err());

        let duplicate = ProviderConfigInput {
            order: vec!["gemini".to_string(), "gemini".to_string()],
            ..base.clone()
        };
        assert!(validate_input(&duplicate).is_err());

        let bad_attempts = ProviderConfigInput {
            providers: vec![ProviderSettingInput {
                id: "gemini".to_string(),
                model: None,
                models: None,
                base_url: None,
                attempts: Some(0),
                enabled: None,
            }],
            ..base.clone()
        };
        assert!(validate_input(&bad_attempts).is_err());

        let openrouter_attempts = ProviderConfigInput {
            providers: vec![ProviderSettingInput {
                id: "openrouter".to_string(),
                model: None,
                models: None,
                base_url: None,
                attempts: Some(3),
                enabled: None,
            }],
            ..base.clone()
        };
        assert!(validate_input(&openrouter_attempts).is_err());

        let bad_url = ProviderConfigInput {
            providers: vec![ProviderSettingInput {
                id: "ollama".to_string(),
                model: None,
                models: None,
                base_url: Some("ftp://x".to_string()),
                attempts: None,
                enabled: None,
            }],
            ..base
        };
        assert!(validate_input(&bad_url).is_err());
    }
}
