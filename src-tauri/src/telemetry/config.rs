//! Telemetry configuration from the environment.
#![allow(missing_docs)]

use std::path::PathBuf;

use crate::telemetry::event::Level;

#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub level: Level,
    pub log_dir: PathBuf,
    pub retention_days: u64,
    pub max_total_bytes: u64,
    pub queue_capacity: usize,
}

impl TelemetryConfig {
    pub fn new(log_dir: PathBuf, level: Level) -> Self {
        Self {
            level,
            log_dir,
            retention_days: 7,
            max_total_bytes: 52_428_800,
            queue_capacity: 10_000,
        }
    }

    pub fn from_env(log_dir: PathBuf) -> (Self, Option<String>) {
        let raw = std::env::var("RUST_LOG").unwrap_or_default();
        Self::from_env_value(&raw, log_dir)
    }

    pub fn from_env_value(raw: &str, log_dir: PathBuf) -> (Self, Option<String>) {
        let (level, invalid) = Level::parse_lossy(raw);
        let warning =
            invalid.then(|| format!("invalid RUST_LOG value `{raw}`; defaulting to INFO"));
        (Self::new(log_dir, level), warning)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_bounded() {
        let config = TelemetryConfig::new(PathBuf::from("/tmp/logs"), Level::Info);
        assert_eq!(config.level, Level::Info);
        assert_eq!(config.retention_days, 7);
        assert_eq!(config.max_total_bytes, 52_428_800);
        assert_eq!(config.queue_capacity, 10_000);
    }

    #[test]
    fn env_value_parses_levels() {
        let (config, warning) =
            TelemetryConfig::from_env_value("debug", PathBuf::from("/tmp/logs"));
        assert_eq!(config.level, Level::Debug);
        assert!(warning.is_none());

        let (config, warning) =
            TelemetryConfig::from_env_value("jobs_search=warn", PathBuf::from("/tmp/logs"));
        assert_eq!(config.level, Level::Warn);
        assert!(warning.is_none());
    }

    #[test]
    fn invalid_env_value_falls_back_to_info() {
        let (config, warning) =
            TelemetryConfig::from_env_value("verbose", PathBuf::from("/tmp/logs"));
        assert_eq!(config.level, Level::Info);
        assert!(warning.is_some());
    }
}
