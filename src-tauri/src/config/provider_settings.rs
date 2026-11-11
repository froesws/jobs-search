//! Effective provider configuration.
//!
//! Resolution order is session override → `app_settings` → `.env`/OS environment
//! → built-in defaults. Credentials resolve as OS credential store → `.env`.

use std::collections::BTreeMap;

use crate::config::AppConfig;
use crate::credentials;
use crate::db::{Db, settings};
use crate::error::AppError;

/// Supported providers, in the default order.
pub const PROVIDERS: [&str; 4] = ["gemini", "openrouter", "deepseek", "ollama"];

/// A session-only provider/model selection. Never persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSelection {
    /// Selected provider.
    pub provider: String,
    /// Ordered models; length one for single-model providers.
    pub models: Vec<String>,
}

/// Effective settings for one provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSetting {
    /// Single model for gemini, deepseek and ollama.
    pub model: Option<String>,
    /// Ordered model list for openrouter.
    pub models: Vec<String>,
    /// Editable base endpoint (ollama only).
    pub base_url: Option<String>,
    /// Attempt budget.
    pub attempts: u32,
    /// Whether the provider participates in the chain (ollama only).
    pub enabled: bool,
}

impl ProviderSetting {
    /// Models in attempt order. Single-model providers repeat their model.
    pub fn attempt_models(&self) -> Vec<String> {
        if !self.models.is_empty() {
            return self.models.clone();
        }
        self.model
            .clone()
            .map(|model| vec![model])
            .unwrap_or_default()
    }
}

/// Effective provider configuration for a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSettings {
    /// Provider order.
    pub order: Vec<String>,
    /// Whether cross-provider fallback is enabled.
    pub fallback_enabled: bool,
    /// Per-provider settings.
    pub providers: BTreeMap<String, ProviderSetting>,
}

impl ProviderSettings {
    /// Resolve the effective settings.
    pub fn resolve(
        db: &Db,
        config: &AppConfig,
        session: Option<&SessionSelection>,
    ) -> Result<Self, AppError> {
        let order = resolve_order(db, config)?;
        let fallback_enabled = resolve_fallback(db, config)?;

        let mut providers = BTreeMap::new();
        for provider in PROVIDERS {
            providers.insert(
                provider.to_string(),
                resolve_provider(db, config, provider)?,
            );
        }

        let mut settings = Self {
            order,
            fallback_enabled,
            providers,
        };
        settings.apply_session(session);
        Ok(settings)
    }

    /// Apply a session-only override on top of the persisted settings.
    pub fn apply_session(&mut self, session: Option<&SessionSelection>) {
        let Some(session) = session else {
            return;
        };
        if !PROVIDERS.contains(&session.provider.as_str()) || session.models.is_empty() {
            return;
        }
        if let Some(setting) = self.providers.get_mut(&session.provider) {
            if session.provider == "openrouter" {
                setting.models = session.models.clone();
            } else {
                setting.model = session.models.first().cloned();
            }
        }
        self.order.retain(|entry| entry != &session.provider);
        self.order.insert(0, session.provider.clone());
    }

    /// Providers that can actually run, in order.
    ///
    /// A cloud provider is eligible when a credential resolves; ollama is
    /// eligible only when it is enabled and has a model.
    pub fn eligible(&self, config: &AppConfig) -> Vec<String> {
        self.order
            .iter()
            .filter(|provider| {
                let Some(setting) = self.providers.get(*provider) else {
                    return false;
                };
                if provider.as_str() == "ollama" {
                    return setting.enabled && !setting.attempt_models().is_empty();
                }
                let (secret, _) = resolve_credential(config, provider);
                secret.is_some()
            })
            .cloned()
            .collect()
    }

    /// The preferred provider, if any.
    pub fn preferred(&self, config: &AppConfig) -> Option<String> {
        self.eligible(config).into_iter().next()
    }
}

fn resolve_order(db: &Db, config: &AppConfig) -> Result<Vec<String>, AppError> {
    if let Some(raw) = settings::get(db, "provider.order")?
        && let Ok(parsed) = serde_json::from_str::<Vec<String>>(&raw)
    {
        let valid: Vec<String> = parsed
            .into_iter()
            .map(|value| value.to_ascii_lowercase())
            .filter(|value| PROVIDERS.contains(&value.as_str()))
            .collect();
        if !valid.is_empty() {
            return Ok(sanitize_order(valid));
        }
    }
    let order = sanitize_order(config.provider_order().to_vec());
    Ok(order)
}

fn sanitize_order(values: Vec<String>) -> Vec<String> {
    let mut order = Vec::new();
    for value in values {
        let value = value.to_ascii_lowercase();
        if PROVIDERS.contains(&value.as_str()) && !order.contains(&value) {
            order.push(value);
        }
    }
    // Every supported provider stays in the chain; ollama is gated by its own
    // enable flag rather than by being absent from the order.
    for provider in PROVIDERS {
        if !order.iter().any(|entry| entry == provider) {
            order.push(provider.to_string());
        }
    }
    order
}

fn resolve_fallback(db: &Db, config: &AppConfig) -> Result<bool, AppError> {
    if let Some(raw) = settings::get(db, "provider.fallback_enabled")? {
        return Ok(raw == "true");
    }
    Ok(config.allow_fallback)
}

fn resolve_provider(
    db: &Db,
    config: &AppConfig,
    provider: &str,
) -> Result<ProviderSetting, AppError> {
    let model = match provider {
        "gemini" => Some(
            settings::get(db, "provider.gemini.model")?
                .unwrap_or_else(|| config.gemini_model.clone()),
        ),
        "deepseek" => Some(
            settings::get(db, "provider.deepseek.model")?
                .unwrap_or_else(|| config.deepseek_model.clone()),
        ),
        "ollama" => {
            let stored = settings::get(db, "provider.ollama.model")?;
            let value = stored.unwrap_or_else(|| config.ollama_model.clone());
            if value.is_empty() { None } else { Some(value) }
        }
        _ => None,
    };

    let models = if provider == "openrouter" {
        match settings::get(db, "provider.openrouter.models")? {
            Some(raw) => serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default(),
            None => config.openrouter_free_models.clone(),
        }
    } else {
        Vec::new()
    };

    let base_url = if provider == "ollama" {
        Some(
            settings::get(db, "provider.ollama.base_url")?
                .unwrap_or_else(|| config.ollama_base_url.clone()),
        )
    } else {
        None
    };

    let attempts = match settings::get(db, &format!("provider.{provider}.attempts"))? {
        Some(raw) => raw
            .parse::<u32>()
            .unwrap_or_else(|_| default_attempts(provider)),
        None => default_attempts(provider),
    };

    let enabled = if provider == "ollama" {
        settings::get(db, "provider.ollama.enabled")?.as_deref() == Some("true")
    } else {
        true
    };

    Ok(ProviderSetting {
        model,
        models,
        base_url,
        attempts: attempts.clamp(1, 10),
        enabled,
    })
}

/// Resolve a provider credential: OS store, then the in-memory configuration
/// loaded from `.env`/OS environment, then the `.env` file itself.
pub fn resolve_credential(
    config: &AppConfig,
    provider: &str,
) -> (Option<String>, credentials::CredentialSource) {
    if let Ok(Some(secret)) = credentials::get_from_store(provider) {
        return (Some(secret), credentials::CredentialSource::CredentialStore);
    }
    let in_memory = match provider {
        "gemini" => config.gemini_api_key.clone(),
        "openrouter" => config.openrouter_api_key.clone(),
        "deepseek" => config.deepseek_api_key.clone(),
        _ => None,
    };
    if let Some(secret) = in_memory {
        return (Some(secret), credentials::CredentialSource::Env);
    }
    if let Some(key) = credentials::env_key(provider)
        && let Some(path) = config.env_file.as_deref()
        && let Some(value) = credentials::read_env_value(path, key)
    {
        return (Some(value), credentials::CredentialSource::Env);
    }
    (None, credentials::CredentialSource::None)
}

/// Built-in attempt budget for a provider.
pub fn default_attempts(provider: &str) -> u32 {
    match provider {
        "gemini" => 3,
        "openrouter" => 3,
        "deepseek" => 2,
        "ollama" => 2,
        _ => 1,
    }
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
    fn defaults_come_from_the_environment() {
        let db = Db::open_in_memory().expect("db");
        let config = config(&[("DEEPSEEK_API_KEY", "secret")]);
        let settings = ProviderSettings::resolve(&db, &config, None).expect("resolve");

        assert_eq!(
            settings.order,
            vec!["gemini", "openrouter", "deepseek", "ollama"]
        );
        assert!(!settings.fallback_enabled);
        assert_eq!(
            settings.providers["gemini"].model.as_deref(),
            Some("gemini-3.5-flash-lite")
        );
        assert_eq!(settings.providers["deepseek"].attempts, 2);
        assert!(!settings.providers["ollama"].enabled);
    }

    #[test]
    fn stored_settings_override_the_environment() {
        let db = Db::open_in_memory().expect("db");
        settings::put(&db, "provider.order", "[\"deepseek\",\"gemini\"]").expect("put");
        settings::put(&db, "provider.fallback_enabled", "true").expect("put");
        settings::put(&db, "provider.gemini.model", "gemini-custom").expect("put");
        settings::put(&db, "provider.gemini.attempts", "5").expect("put");
        settings::put(&db, "provider.ollama.enabled", "true").expect("put");
        settings::put(&db, "provider.ollama.model", "qwen").expect("put");

        let config = config(&[("GEMINI_MODEL", "env-model")]);
        let settings = ProviderSettings::resolve(&db, &config, None).expect("resolve");

        assert_eq!(
            settings.order,
            vec!["deepseek", "gemini", "openrouter", "ollama"]
        );
        assert!(settings.fallback_enabled);
        assert_eq!(
            settings.providers["gemini"].model.as_deref(),
            Some("gemini-custom")
        );
        assert_eq!(settings.providers["gemini"].attempts, 5);
        assert!(settings.providers["ollama"].enabled);
        assert_eq!(settings.providers["ollama"].model.as_deref(), Some("qwen"));
    }

    #[test]
    fn session_override_wins_and_moves_the_provider_first() {
        let db = Db::open_in_memory().expect("db");
        let config = config(&[]);
        let session = SessionSelection {
            provider: "openrouter".to_string(),
            models: vec!["a".to_string(), "b".to_string()],
        };
        let settings = ProviderSettings::resolve(&db, &config, Some(&session)).expect("resolve");

        assert_eq!(
            settings.order.first().map(String::as_str),
            Some("openrouter")
        );
        assert_eq!(settings.providers["openrouter"].models, vec!["a", "b"]);
    }

    #[test]
    fn invalid_stored_order_falls_back_to_the_environment() {
        let db = Db::open_in_memory().expect("db");
        settings::put(&db, "provider.order", "[\"unknown\"]").expect("put");
        let config = config(&[("LLM_PROVIDER_ORDER", "deepseek,gemini")]);
        let settings = ProviderSettings::resolve(&db, &config, None).expect("resolve");
        assert_eq!(
            settings.order,
            vec!["deepseek", "gemini", "openrouter", "ollama"]
        );
    }

    #[test]
    fn attempts_are_clamped() {
        let db = Db::open_in_memory().expect("db");
        settings::put(&db, "provider.gemini.attempts", "99").expect("put");
        let config = config(&[]);
        let settings = ProviderSettings::resolve(&db, &config, None).expect("resolve");
        assert_eq!(settings.providers["gemini"].attempts, 10);
    }

    #[test]
    fn ollama_is_ineligible_until_enabled_with_a_model() {
        let db = Db::open_in_memory().expect("db");
        let config = config(&[]);
        let settings = ProviderSettings::resolve(&db, &config, None).expect("resolve");
        assert!(!settings.eligible(&config).contains(&"ollama".to_string()));

        settings::put(&db, "provider.ollama.enabled", "true").expect("put");
        settings::put(&db, "provider.ollama.model", "qwen").expect("put");
        let settings = ProviderSettings::resolve(&db, &config, None).expect("resolve");
        assert!(settings.eligible(&config).contains(&"ollama".to_string()));
    }
}
