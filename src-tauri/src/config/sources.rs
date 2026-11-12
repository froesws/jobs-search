//! Job source configuration from `sources.yaml`.
#![allow(missing_docs)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Mcp,
    Api,
    WebScraper,
}

impl SourceType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mcp => "mcp",
            Self::Api => "api",
            Self::WebScraper => "web_scraper",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimitConfig {
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_backoff_ms")]
    pub backoff_ms: u64,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    /// Maximum time the whole source may consume in one run.
    #[serde(default = "default_budget_seconds")]
    pub budget_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: default_requests_per_minute(),
            max_retries: default_max_retries(),
            backoff_ms: default_backoff_ms(),
            timeout_seconds: default_timeout_seconds(),
            budget_seconds: default_budget_seconds(),
        }
    }
}

fn default_requests_per_minute() -> u32 {
    20
}

fn default_max_retries() -> u32 {
    2
}

fn default_backoff_ms() -> u64 {
    500
}

fn default_timeout_seconds() -> u64 {
    15
}

fn default_budget_seconds() -> u64 {
    180
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceConfig {
    pub id: String,
    pub name: String,
    pub source_type: SourceType,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub config: serde_yaml::Value,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub credential_ref: Option<String>,
    /// When set, this source only runs if the referenced primary source fails
    /// or delivers fewer postings than [`Self::fallback_min_results`].
    #[serde(default)]
    pub fallback_for: Option<String>,
}

fn default_enabled() -> bool {
    true
}

impl SourceConfig {
    pub fn config_str(&self, key: &str) -> Option<&str> {
        self.config.get(key).and_then(serde_yaml::Value::as_str)
    }

    pub fn config_string_list(&self, key: &str) -> Vec<String> {
        self.config
            .get(key)
            .and_then(serde_yaml::Value::as_sequence)
            .map(|items| {
                items
                    .iter()
                    .filter_map(serde_yaml::Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn config_u32(&self, key: &str, default: u32) -> u32 {
        self.config
            .get(key)
            .and_then(serde_yaml::Value::as_u64)
            .map(|value| u32::try_from(value).unwrap_or(u32::MAX))
            .unwrap_or(default)
    }

    pub fn config_bool(&self, key: &str, default: bool) -> bool {
        self.config
            .get(key)
            .and_then(serde_yaml::Value::as_bool)
            .unwrap_or(default)
    }

    /// Minimum number of postings the primary must deliver before this
    /// fallback stays idle. Values outside `1..=50` are clamped.
    pub fn fallback_min_results(&self) -> u32 {
        self.config_u32("fallback_min_results", 1).clamp(1, 50)
    }

    /// Maximum time this source may consume in one run. Values outside
    /// `30..=900` are clamped.
    pub fn budget_seconds(&self) -> u64 {
        self.rate_limit.budget_seconds.clamp(30, 900)
    }

    pub fn config_json(&self) -> Result<String, AppError> {
        serde_json::to_string(&self.config).map_err(AppError::from)
    }

    /// Resolves the optional `cwd` entry against the directory containing the
    /// sources file. Relative paths in a config file are relative to that file,
    /// not to the process working directory (which differs when the desktop
    /// shell or `tauri dev` starts the app).
    pub fn resolved_cwd(&self, base_dir: &Path) -> Option<PathBuf> {
        let cwd = self
            .config_str("cwd")
            .map(str::trim)
            .filter(|value| !value.is_empty())?;
        let path = PathBuf::from(cwd);
        if path.is_absolute() {
            Some(path)
        } else {
            Some(base_dir.join(path))
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcesConfig {
    #[serde(default)]
    pub sources: Vec<SourceConfig>,
}

impl SourcesConfig {
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let contents = std::fs::read_to_string(path).map_err(|error| {
            AppError::Config(format!(
                "could not read sources file {}: {error}",
                path.display()
            ))
        })?;
        let config: Self = serde_yaml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    pub fn load_or_default(path: &Path) -> Result<Self, AppError> {
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::default_sources())
        }
    }

    pub fn default_sources() -> Self {
        Self {
            sources: vec![
                SourceConfig {
                    id: "jobspy".to_string(),
                    name: "jobspy".to_string(),
                    source_type: SourceType::Mcp,
                    enabled: true,
                    config: serde_yaml::from_str(
                        r#"
command: uv
args: ["run", "--project", "sidecar/jobspy_mcp", "jobspy-mcp-server"]
cwd: "."
sites:
  - linkedin
  - indeed
  - glassdoor
  - zip_recruiter
  - google
  - bayt
  - naukri
  - bdjobs
  - hellowork
results_wanted: 10
"#,
                    )
                    .expect("static jobspy source config"),
                    rate_limit: RateLimitConfig {
                        requests_per_minute: 10,
                        max_retries: 2,
                        backoff_ms: 500,
                        timeout_seconds: 180,
                        budget_seconds: 300,
                    },
                    credential_ref: None,
                    fallback_for: None,
                },
                SourceConfig {
                    id: "linkedin".to_string(),
                    name: "linkedin".to_string(),
                    source_type: SourceType::Mcp,
                    enabled: true,
                    config: serde_yaml::from_str(
                        r#"
command: uvx
args: ["mcp-server-linkedin@latest"]
adapter: linkedin
results_wanted: 10
max_pages: 1
"#,
                    )
                    .expect("static linkedin source config"),
                    rate_limit: RateLimitConfig {
                        requests_per_minute: 4,
                        max_retries: 1,
                        backoff_ms: 1000,
                        timeout_seconds: 300,
                        budget_seconds: 180,
                    },
                    credential_ref: None,
                    fallback_for: None,
                },
                SourceConfig {
                    id: "linkedin_scraper".to_string(),
                    name: "linkedin_scraper".to_string(),
                    source_type: SourceType::Mcp,
                    enabled: true,
                    fallback_for: Some("linkedin".to_string()),
                    config: serde_yaml::from_str(
                        r#"
command: uv
args: ["run", "--project", "sidecar/linkedin_scraper_mcp", "linkedin-scraper-mcp"]
cwd: "."
sites: ["linkedin"]
results_wanted: 10
"#,
                    )
                    .expect("static linkedin scraper source config"),
                    rate_limit: RateLimitConfig {
                        requests_per_minute: 4,
                        max_retries: 1,
                        backoff_ms: 1000,
                        timeout_seconds: 300,
                        budget_seconds: 180,
                    },
                    credential_ref: None,
                },
                SourceConfig {
                    id: "remoteok".to_string(),
                    name: "remoteok".to_string(),
                    source_type: SourceType::Api,
                    enabled: true,
                    config: serde_yaml::from_str(
                        r#"
base_url: "https://remoteok.com/api"
results_wanted: 50
"#,
                    )
                    .expect("static remoteok source config"),
                    rate_limit: RateLimitConfig {
                        requests_per_minute: 20,
                        max_retries: 2,
                        backoff_ms: 500,
                        timeout_seconds: 15,
                        budget_seconds: 30,
                    },
                    credential_ref: None,
                    fallback_for: None,
                },
            ],
        }
    }

    pub fn enabled(&self) -> Vec<&SourceConfig> {
        self.sources
            .iter()
            .filter(|source| source.enabled)
            .collect()
    }

    pub fn validate(&self) -> Result<(), AppError> {
        let mut ids = HashSet::new();
        let mut names = HashSet::new();

        for source in &self.sources {
            if source.id.trim().is_empty() {
                return Err(AppError::Config("source id cannot be empty".to_string()));
            }
            if source.name.trim().is_empty() {
                return Err(AppError::Config(format!(
                    "source {} has an empty name",
                    source.id
                )));
            }
            if !ids.insert(source.id.clone()) {
                return Err(AppError::Config(format!(
                    "duplicate source id: {}",
                    source.id
                )));
            }
            if !names.insert(source.name.clone()) {
                return Err(AppError::Config(format!(
                    "duplicate source name: {}",
                    source.name
                )));
            }
            if source.rate_limit.requests_per_minute == 0 {
                return Err(AppError::Config(format!(
                    "source {} has requests_per_minute = 0",
                    source.id
                )));
            }

            match source.source_type {
                SourceType::Mcp => {
                    if source
                        .config_str("command")
                        .map(str::trim)
                        .unwrap_or_default()
                        .is_empty()
                    {
                        return Err(AppError::Config(format!(
                            "mcp source {} requires config.command",
                            source.id
                        )));
                    }
                }
                SourceType::Api => {
                    let base_url = source
                        .config_str("base_url")
                        .map(str::trim)
                        .unwrap_or_default();
                    if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
                        return Err(AppError::Config(format!(
                            "api source {} requires an http(s) config.base_url",
                            source.id
                        )));
                    }
                }
                SourceType::WebScraper => {}
            }
        }

        let mut fallback_targets = HashSet::new();
        for source in &self.sources {
            let Some(target) = source.fallback_for.as_deref() else {
                continue;
            };
            let target = target.trim();
            if target.is_empty() {
                return Err(AppError::Config(format!(
                    "source {} has an empty fallback_for",
                    source.id
                )));
            }
            if target == source.id {
                return Err(AppError::Config(format!(
                    "source {} cannot be its own fallback",
                    source.id
                )));
            }
            if !ids.contains(target) {
                return Err(AppError::Config(format!(
                    "source {} references unknown fallback target {target}",
                    source.id
                )));
            }
            if !fallback_targets.insert(target.to_string()) {
                return Err(AppError::Config(format!(
                    "duplicate fallback target: {target}"
                )));
            }
        }

        for source in &self.sources {
            if source.fallback_for.is_some() && fallback_targets.contains(&source.id) {
                return Err(AppError::Config(format!(
                    "source {} is a fallback and cannot be the primary of another fallback",
                    source.id
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
sources:
  - id: jobspy
    name: jobspy
    source_type: mcp
    config:
      command: python3
      args: ["-m", "jobspy_mcp_server.server"]
      sites: ["linkedin", "indeed"]
    rate_limit:
      requests_per_minute: 10
      max_retries: 1
      backoff_ms: 10
      timeout_seconds: 20
  - id: remoteok
    name: remoteok
    source_type: api
    enabled: false
    config:
      base_url: "https://remoteok.com/api"
"#;

    #[test]
    fn parses_sources_and_rate_limits() {
        let config: SourcesConfig = serde_yaml::from_str(VALID).expect("valid yaml");
        config.validate().expect("valid sources");
        assert_eq!(config.sources.len(), 2);
        assert_eq!(config.enabled().len(), 1);
        assert_eq!(config.sources[0].config_string_list("sites").len(), 2);
        assert_eq!(config.sources[0].rate_limit.requests_per_minute, 10);
        assert_eq!(
            config.sources[1].config_str("base_url"),
            Some("https://remoteok.com/api")
        );
    }

    #[test]
    fn relative_cwd_resolves_against_the_sources_directory() {
        let config: SourcesConfig = serde_yaml::from_str(
            r#"
sources:
  - id: jobspy
    name: jobspy
    source_type: mcp
    config:
      command: uv
      cwd: "."
  - id: nested
    name: nested
    source_type: mcp
    config:
      command: uv
      cwd: "sidecar/jobspy_mcp"
  - id: absolute
    name: absolute
    source_type: mcp
    config:
      command: uv
      cwd: "/opt/sidecars/jobspy"
  - id: unset
    name: unset
    source_type: mcp
    config:
      command: uv
"#,
        )
        .expect("valid yaml");

        let base = Path::new("/home/dev/project");
        assert_eq!(
            config.sources[0].resolved_cwd(base),
            Some(PathBuf::from("/home/dev/project"))
        );
        assert_eq!(
            config.sources[1].resolved_cwd(base),
            Some(PathBuf::from("/home/dev/project/sidecar/jobspy_mcp"))
        );
        assert_eq!(
            config.sources[2].resolved_cwd(base),
            Some(PathBuf::from("/opt/sidecars/jobspy"))
        );
        assert_eq!(config.sources[3].resolved_cwd(base), None);
    }

    #[test]
    fn parses_boolean_config_values() {
        let config: SourcesConfig = serde_yaml::from_str(
            r#"
sources:
  - id: jobspy
    name: jobspy
    source_type: mcp
    config:
      command: uv
      fetch_descriptions: false
"#,
        )
        .expect("valid yaml");

        assert!(!config.sources[0].config_bool("fetch_descriptions", true));
        assert!(config.sources[0].config_bool("missing", true));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let config: SourcesConfig = serde_yaml::from_str(
            r#"
sources:
  - { id: a, name: one, source_type: api, config: { base_url: "https://example.com" } }
  - { id: a, name: two, source_type: api, config: { base_url: "https://example.com" } }
"#,
        )
        .expect("valid yaml");
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_mcp_without_command() {
        let config: SourcesConfig = serde_yaml::from_str(
            r#"
sources:
  - { id: jobspy, name: jobspy, source_type: mcp, config: {} }
"#,
        )
        .expect("valid yaml");
        assert!(config.validate().is_err());
    }

    #[test]
    fn default_sources_are_valid() {
        let config = SourcesConfig::default_sources();
        config.validate().expect("defaults are valid");
        assert_eq!(config.enabled().len(), 4);
        let linkedin = config
            .sources
            .iter()
            .find(|source| source.id == "linkedin")
            .expect("linkedin");
        assert_eq!(linkedin.budget_seconds(), 180);
        let scraper = config
            .sources
            .iter()
            .find(|source| source.id == "linkedin_scraper")
            .expect("scraper");
        assert_eq!(scraper.fallback_for.as_deref(), Some("linkedin"));
        assert_eq!(scraper.fallback_min_results(), 1);
    }

    #[test]
    fn accepts_a_valid_fallback_pair() {
        let config: SourcesConfig = serde_yaml::from_str(
            r#"
sources:
  - { id: linkedin, name: linkedin, source_type: mcp, config: { command: uvx } }
  - id: scraper
    name: scraper
    source_type: mcp
    fallback_for: linkedin
    config: { command: uv }
"#,
        )
        .expect("valid yaml");
        config.validate().expect("valid pair");
    }

    #[test]
    fn rejects_a_source_as_its_own_fallback() {
        let config: SourcesConfig = serde_yaml::from_str(
            r#"
sources:
  - id: linkedin
    name: linkedin
    source_type: mcp
    fallback_for: linkedin
    config: { command: uvx }
"#,
        )
        .expect("valid yaml");
        let error = config.validate().expect_err("self reference");
        assert!(error.message().contains("own fallback"));
    }

    #[test]
    fn rejects_an_unknown_fallback_target() {
        let config: SourcesConfig = serde_yaml::from_str(
            r#"
sources:
  - id: scraper
    name: scraper
    source_type: mcp
    fallback_for: linkedin
    config: { command: uv }
"#,
        )
        .expect("valid yaml");
        let error = config.validate().expect_err("unknown target");
        assert!(error.message().contains("unknown fallback target"));
    }

    #[test]
    fn rejects_two_sources_targeting_the_same_primary() {
        let config: SourcesConfig = serde_yaml::from_str(
            r#"
sources:
  - { id: linkedin, name: linkedin, source_type: mcp, config: { command: uvx } }
  - id: one
    name: one
    source_type: mcp
    fallback_for: linkedin
    config: { command: uv }
  - id: two
    name: two
    source_type: mcp
    fallback_for: linkedin
    config: { command: uv }
"#,
        )
        .expect("valid yaml");
        let error = config.validate().expect_err("duplicate target");
        assert!(error.message().contains("duplicate fallback target"));
    }

    #[test]
    fn rejects_a_fallback_chain() {
        let config: SourcesConfig = serde_yaml::from_str(
            r#"
sources:
  - { id: linkedin, name: linkedin, source_type: mcp, config: { command: uvx } }
  - id: one
    name: one
    source_type: mcp
    fallback_for: linkedin
    config: { command: uv }
  - id: two
    name: two
    source_type: mcp
    fallback_for: one
    config: { command: uv }
"#,
        )
        .expect("valid yaml");
        let error = config.validate().expect_err("chain");
        assert!(error.message().contains("cannot be the primary"));
    }

    #[test]
    fn budget_seconds_defaults_and_clamps() {
        let source: SourceConfig =
            serde_yaml::from_str("{ id: a, name: a, source_type: mcp, config: { command: uv } }")
                .expect("source");
        assert_eq!(source.rate_limit.budget_seconds, 180);
        assert_eq!(source.budget_seconds(), 180);

        let mut clamped_low = source.clone();
        clamped_low.rate_limit.budget_seconds = 1;
        assert_eq!(clamped_low.budget_seconds(), 30);

        let mut clamped_high = source.clone();
        clamped_high.rate_limit.budget_seconds = 5_000;
        assert_eq!(clamped_high.budget_seconds(), 900);

        let overridden: SourceConfig = serde_yaml::from_str(
            r#"
id: a
name: a
source_type: mcp
config: { command: uv }
rate_limit: { budget_seconds: 45 }
"#,
        )
        .expect("source");
        assert_eq!(overridden.budget_seconds(), 45);
    }

    #[test]
    fn fallback_min_results_clamps() {
        let source: SourceConfig = serde_yaml::from_str(
            r#"
id: a
name: a
source_type: mcp
config: { command: uv, fallback_min_results: 999 }
"#,
        )
        .expect("source");
        assert_eq!(source.fallback_min_results(), 50);

        let low: SourceConfig = serde_yaml::from_str(
            r#"
id: a
name: a
source_type: mcp
config: { command: uv, fallback_min_results: 0 }
"#,
        )
        .expect("source");
        assert_eq!(low.fallback_min_results(), 1);
    }

    #[test]
    fn the_repository_sources_file_is_valid() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repository root")
            .join("sources.yaml");
        let config = SourcesConfig::load(&path).expect("valid sources.yaml");
        let scraper = config
            .sources
            .iter()
            .find(|source| source.id == "linkedin_scraper")
            .expect("scraper");
        assert_eq!(scraper.fallback_for.as_deref(), Some("linkedin"));
        assert_eq!(scraper.budget_seconds(), 180);
        assert_eq!(scraper.fallback_min_results(), 1);
    }
}
