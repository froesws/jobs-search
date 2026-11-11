use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Runtime configuration loaded from the OS environment and `.env`.
///
/// Precedence is OS environment first, then `.env`, then defaults. API keys are
/// never rendered by `Debug` and are never persisted in SQLite.
#[derive(Clone)]
pub struct AppConfig {
    pub app_data_dir: PathBuf,
    pub database_path: PathBuf,
    pub sources_path: PathBuf,
    pub fixture_profile_path: Option<PathBuf>,
    pub provider_order: Vec<String>,
    pub openrouter_free_models: Vec<String>,
    pub gemini_model: String,
    pub openrouter_base_url: String,
    pub deepseek_base_url: String,
    pub gemini_base_url: String,
    pub gemini_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub deepseek_api_key: Option<String>,
    pub llm_timeout_seconds: u64,
    pub allow_fallback: bool,
    pub match_threshold: i64,
    pub max_evaluations: usize,
    pub max_concurrency: usize,
    pub rust_log: String,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppConfig")
            .field("app_data_dir", &self.app_data_dir)
            .field("database_path", &self.database_path)
            .field("sources_path", &self.sources_path)
            .field("fixture_profile_path", &self.fixture_profile_path)
            .field("provider_order", &self.provider_order)
            .field("openrouter_free_models", &self.openrouter_free_models)
            .field("gemini_model", &self.gemini_model)
            .field("llm_timeout_seconds", &self.llm_timeout_seconds)
            .field("allow_fallback", &self.allow_fallback)
            .field("match_threshold", &self.match_threshold)
            .field("max_evaluations", &self.max_evaluations)
            .field("max_concurrency", &self.max_concurrency)
            .finish_non_exhaustive()
    }
}

impl AppConfig {
    /// Load configuration from the process environment after loading `.env`.
    pub fn from_env(default_app_data: PathBuf) -> Result<Self, AppError> {
        // `.env` is optional and never overrides variables already exported by
        // the OS. Values are not logged.
        let _ = dotenvy::dotenv();

        let cwd = std::env::current_dir()?;
        Self::from_lookup(
            |key| std::env::var(key).ok(),
            default_app_data,
            cwd.as_path(),
        )
    }

    /// Deterministic constructor used by tests.
    pub fn from_lookup(
        lookup: impl Fn(&str) -> Option<String>,
        default_app_data: PathBuf,
        working_dir: &Path,
    ) -> Result<Self, AppError> {
        let read = |key: &str| lookup(key).map(|value| value.trim().to_string());

        let app_data_dir = read("JOBS_SEARCH_APP_DATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or(default_app_data);

        let database_path = read("JOBS_SEARCH_DB_PATH")
            .filter(|value| !value.is_empty())
            .map(|value| resolve(working_dir, &value))
            .unwrap_or_else(|| app_data_dir.join("jobs-search.sqlite3"));

        let sources_path = read("JOBS_SEARCH_SOURCES")
            .filter(|value| !value.is_empty())
            .map(|value| resolve(working_dir, &value))
            .unwrap_or_else(|| working_dir.join("sources.yaml"));

        let fixture_profile_path = read("JOBS_SEARCH_FIXTURE_PROFILE")
            .filter(|value| !value.is_empty())
            .map(|value| resolve(working_dir, &value));

        let provider_order = parse_provider_order(
            read("LLM_PROVIDER_ORDER")
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "gemini,openrouter,deepseek".to_string()),
        )?;

        let openrouter_free_models = parse_list(
            read("OPENROUTER_FREE_MODELS")
                .filter(|value| !value.is_empty())
                .unwrap_or_else(default_openrouter_models),
        );
        if openrouter_free_models.is_empty() {
            return Err(AppError::Config(
                "OPENROUTER_FREE_MODELS must contain at least one model".to_string(),
            ));
        }

        let gemini_model = read("GEMINI_MODEL")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "gemini-2.5-flash".to_string());
        let gemini_base_url = read("GEMINI_BASE_URL")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string());
        let openrouter_base_url = read("OPENROUTER_BASE_URL")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
        let deepseek_base_url = read("DEEPSEEK_BASE_URL")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "https://api.deepseek.com/v1".to_string());

        let gemini_api_key = non_empty(read("GEMINI_API_KEY"));
        let openrouter_api_key = non_empty(read("OPENROUTER_API_KEY"));
        let deepseek_api_key = non_empty(read("DEEPSEEK_API_KEY"));

        let llm_timeout_seconds = parse_u64(read("LLM_TIMEOUT_SECONDS"), 45, 1, 300)?;
        let allow_fallback = parse_bool(read("LLM_ALLOW_FALLBACK"), false)?;
        let match_threshold = parse_i64(read("MATCH_THRESHOLD"), 70, 0, 100)?;
        let max_evaluations = parse_usize(read("JOBS_SEARCH_MAX_EVALUATIONS"), 100, 1, 1_000)?;
        let max_concurrency = parse_usize(read("JOBS_SEARCH_LLM_CONCURRENCY"), 2, 1, 4)?;

        Ok(Self {
            app_data_dir,
            database_path,
            sources_path,
            fixture_profile_path,
            provider_order,
            openrouter_free_models,
            gemini_model,
            openrouter_base_url,
            deepseek_base_url,
            gemini_base_url,
            gemini_api_key,
            openrouter_api_key,
            deepseek_api_key,
            llm_timeout_seconds,
            allow_fallback,
            match_threshold,
            max_evaluations,
            max_concurrency,
            rust_log: read("RUST_LOG").unwrap_or_else(|| "info".to_string()),
        })
    }

    /// Provider configuration keyed by name, in configured order.
    pub fn provider_order(&self) -> &[String] {
        &self.provider_order
    }

    /// Returns a redacted summary safe for diagnostics.
    pub fn redacted_summary(&self) -> BTreeMap<&'static str, String> {
        BTreeMap::from([
            ("database", self.database_path.display().to_string()),
            ("sources", self.sources_path.display().to_string()),
            ("threshold", self.match_threshold.to_string()),
            (
                "fixture_profile",
                self.fixture_profile_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "unset".to_string()),
            ),
            (
                "providers",
                self.provider_order
                    .iter()
                    .filter(|provider| self.has_key(provider))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ])
    }

    pub fn has_key(&self, provider: &str) -> bool {
        match provider {
            "gemini" => self.gemini_api_key.is_some(),
            "openrouter" => self.openrouter_api_key.is_some(),
            "deepseek" => self.deepseek_api_key.is_some(),
            _ => false,
        }
    }
}

fn resolve(working_dir: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        working_dir.join(path)
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|item| !item.is_empty())
}

fn parse_list(value: String) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn default_openrouter_models() -> String {
    [
        "deepseek/deepseek-chat-v3-0324:free",
        "meta-llama/llama-3.3-70b-instruct:free",
        "qwen/qwen-2.5-72b-instruct:free",
    ]
    .join(",")
}

fn parse_provider_order(value: String) -> Result<Vec<String>, AppError> {
    let mut order = Vec::new();
    for provider in parse_list(value) {
        let provider = provider.to_ascii_lowercase();
        if !matches!(provider.as_str(), "gemini" | "openrouter" | "deepseek") {
            return Err(AppError::Config(format!(
                "unknown LLM provider in LLM_PROVIDER_ORDER: {provider}"
            )));
        }
        if !order.contains(&provider) {
            order.push(provider);
        }
    }

    if order.is_empty() {
        return Err(AppError::Config(
            "LLM_PROVIDER_ORDER must contain at least one provider".to_string(),
        ));
    }

    Ok(order)
}

fn parse_bool(value: Option<String>, default: bool) -> Result<bool, AppError> {
    match value {
        Some(raw) => match raw.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(AppError::Config(format!(
                "invalid boolean value for LLM_ALLOW_FALLBACK: {raw}"
            ))),
        },
        None => Ok(default),
    }
}

fn parse_u64(value: Option<String>, default: u64, min: u64, max: u64) -> Result<u64, AppError> {
    match value {
        Some(raw) => raw
            .parse::<u64>()
            .map(|parsed| parsed.clamp(min, max))
            .map_err(|_| AppError::Config(format!("invalid integer value: {raw}"))),
        None => Ok(default),
    }
}

fn parse_i64(value: Option<String>, default: i64, min: i64, max: i64) -> Result<i64, AppError> {
    match value {
        Some(raw) => raw
            .parse::<i64>()
            .map(|parsed| parsed.clamp(min, max))
            .map_err(|_| AppError::Config(format!("invalid integer value: {raw}"))),
        None => Ok(default),
    }
}

fn parse_usize(
    value: Option<String>,
    default: usize,
    min: usize,
    max: usize,
) -> Result<usize, AppError> {
    match value {
        Some(raw) => raw
            .parse::<usize>()
            .map(|parsed| parsed.clamp(min, max))
            .map_err(|_| AppError::Config(format!("invalid integer value: {raw}"))),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(values: &[(&str, &str)]) -> Result<AppConfig, AppError> {
        let map: HashMap<String, String> = values
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();
        AppConfig::from_lookup(
            move |key| map.get(key).cloned(),
            PathBuf::from("/tmp/app-data"),
            Path::new("/tmp/work"),
        )
    }

    #[test]
    fn defaults_are_bounded_and_redacted() {
        let config = config(&[]).expect("valid config");
        assert_eq!(config.match_threshold, 70);
        assert_eq!(config.max_evaluations, 100);
        assert_eq!(config.max_concurrency, 2);
        assert!(!config.allow_fallback);
        assert_eq!(config.provider_order, ["gemini", "openrouter", "deepseek"]);
        assert!(!format!("{config:?}").contains("api_key"));
    }

    #[test]
    fn os_values_override_defaults_and_empty_keys_are_ignored() {
        let config = config(&[
            ("GEMINI_API_KEY", ""),
            ("DEEPSEEK_API_KEY", "secret-value"),
            ("MATCH_THRESHOLD", "85"),
            ("LLM_PROVIDER_ORDER", "deepseek,gemini,deepseek"),
            ("JOBS_SEARCH_FIXTURE_PROFILE", "fixtures/profile.json"),
        ])
        .expect("valid config");

        assert!(!config.has_key("gemini"));
        assert!(config.has_key("deepseek"));
        assert_eq!(config.match_threshold, 85);
        assert_eq!(config.provider_order, ["deepseek", "gemini"]);
        assert_eq!(
            config.fixture_profile_path,
            Some(PathBuf::from("/tmp/work/fixtures/profile.json"))
        );
        assert!(!format!("{:?}", config.redacted_summary()).contains("secret-value"));
    }

    #[test]
    fn invalid_provider_order_is_rejected() {
        let error = config(&[("LLM_PROVIDER_ORDER", "gemini,unknown")]).expect_err("invalid");
        assert!(matches!(error, AppError::Config(_)));
    }

    #[test]
    fn threshold_is_clamped() {
        let config = config(&[("MATCH_THRESHOLD", "170")]).expect("valid");
        assert_eq!(config.match_threshold, 100);
    }

    #[test]
    fn fallback_is_opt_in_and_strictly_parsed() {
        let enabled = config(&[("LLM_ALLOW_FALLBACK", "true")]).expect("valid");
        assert!(enabled.allow_fallback);

        let disabled = config(&[("LLM_ALLOW_FALLBACK", "off")]).expect("valid");
        assert!(!disabled.allow_fallback);

        let error = config(&[("LLM_ALLOW_FALLBACK", "sometimes")]).expect_err("invalid");
        assert!(matches!(error, AppError::Config(_)));
    }
}
