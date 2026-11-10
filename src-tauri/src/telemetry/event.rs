use std::collections::HashMap;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use tracing::field::{Field, Visit};

use crate::telemetry::sanitize::{sanitize_identifier, sanitize_message};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Level {
    Info,
    Warn,
    Error,
    Debug,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Debug => "DEBUG",
        }
    }

    pub fn as_filter_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Debug => "debug",
        }
    }

    pub fn parse_lossy(value: &str) -> (Self, bool) {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return (Self::Info, false);
        }

        let token = trimmed
            .rsplit(',')
            .next()
            .unwrap_or(trimmed)
            .rsplit('=')
            .next()
            .unwrap_or(trimmed)
            .trim()
            .to_ascii_lowercase();

        match token.as_str() {
            "debug" | "trace" => (Self::Debug, false),
            "info" => (Self::Info, false),
            "warn" | "warning" => (Self::Warn, false),
            "error" => (Self::Error, false),
            _ => (Self::Info, true),
        }
    }

    pub fn from_tracing(level: tracing::Level) -> Self {
        match level {
            tracing::Level::TRACE | tracing::Level::DEBUG => Self::Debug,
            tracing::Level::INFO => Self::Info,
            tracing::Level::WARN => Self::Warn,
            tracing::Level::ERROR => Self::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    AppStart,
    AppShutdown,
    SearchRunStart,
    SearchRunEnd,
    SourceFailure,
    DedupComplete,
    LlmCall,
    LlmFallback,
    EvaluationComplete,
    ApplicationAction,
}

impl EventType {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "APP_START" => Some(Self::AppStart),
            "APP_SHUTDOWN" => Some(Self::AppShutdown),
            "SEARCH_RUN_START" => Some(Self::SearchRunStart),
            "SEARCH_RUN_END" => Some(Self::SearchRunEnd),
            "SOURCE_FAILURE" => Some(Self::SourceFailure),
            "DEDUP_COMPLETE" => Some(Self::DedupComplete),
            "LLM_CALL" => Some(Self::LlmCall),
            "LLM_FALLBACK" => Some(Self::LlmFallback),
            "EVALUATION_COMPLETE" => Some(Self::EvaluationComplete),
            "APPLICATION_ACTION" => Some(Self::ApplicationAction),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Component {
    App,
    Ingestion,
    Llm,
    Match,
    Db,
    Ui,
    Telemetry,
}

impl Component {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "APP" => Some(Self::App),
            "INGESTION" => Some(Self::Ingestion),
            "LLM" => Some(Self::Llm),
            "MATCH" => Some(Self::Match),
            "DB" => Some(Self::Db),
            "UI" => Some(Self::Ui),
            "TELEMETRY" => Some(Self::Telemetry),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Outcome {
    Success,
    Failure,
    Partial,
    Skipped,
}

impl Outcome {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "SUCCESS" => Some(Self::Success),
            "FAILURE" => Some(Self::Failure),
            "PARTIAL" => Some(Self::Partial),
            "SKIPPED" => Some(Self::Skipped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCategory {
    Timeout,
    RateLimit,
    Auth,
    Network,
    Parse,
    Io,
    Config,
    Unknown,
}

impl ErrorCategory {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "TIMEOUT" => Some(Self::Timeout),
            "RATE_LIMIT" => Some(Self::RateLimit),
            "AUTH" => Some(Self::Auth),
            "NETWORK" => Some(Self::Network),
            "PARSE" => Some(Self::Parse),
            "IO" => Some(Self::Io),
            "CONFIG" => Some(Self::Config),
            "UNKNOWN" => Some(Self::Unknown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LifecycleTransition {
    Discovered,
    Applied,
    Ignored,
}

impl LifecycleTransition {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "DISCOVERED" => Some(Self::Discovered),
            "APPLIED" => Some(Self::Applied),
            "IGNORED" => Some(Self::Ignored),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEvent {
    pub timestamp: String,
    pub level: Level,
    pub event_type: EventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub component: Component,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_from: Option<String>,
    pub outcome: Outcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_category: Option<ErrorCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle_transition: Option<LifecycleTransition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedup_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl LogEvent {
    pub fn new(
        level: Level,
        event_type: EventType,
        component: Component,
        outcome: Outcome,
    ) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            level,
            event_type,
            run_id: None,
            component,
            source: None,
            provider: None,
            model: None,
            fallback_from: None,
            outcome,
            elapsed_ms: None,
            error_category: None,
            job_ref: None,
            lifecycle_transition: None,
            dedup_count: None,
            message: None,
        }
    }

    pub fn with_run_id(mut self, value: &str) -> Self {
        self.run_id = sanitize_identifier(value);
        self
    }

    pub fn with_source(mut self, value: &str) -> Self {
        self.source = sanitize_identifier(value);
        self
    }

    pub fn with_provider(mut self, value: &str) -> Self {
        self.provider = sanitize_identifier(value);
        self
    }

    pub fn with_model(mut self, value: &str) -> Self {
        self.model = sanitize_identifier(value);
        self
    }

    pub fn with_fallback_from(mut self, value: &str) -> Self {
        self.fallback_from = sanitize_identifier(value);
        self
    }

    pub fn with_elapsed_ms(mut self, value: u64) -> Self {
        self.elapsed_ms = Some(value);
        self
    }

    pub fn with_error_category(mut self, value: ErrorCategory) -> Self {
        self.error_category = Some(value);
        self
    }

    pub fn with_job_ref(mut self, value: &str) -> Self {
        self.job_ref = sanitize_identifier(value);
        self
    }

    pub fn with_lifecycle_transition(mut self, value: LifecycleTransition) -> Self {
        self.lifecycle_transition = Some(value);
        self
    }

    pub fn with_dedup_count(mut self, value: u64) -> Self {
        self.dedup_count = Some(value);
        self
    }

    pub fn with_message(mut self, value: &str) -> Self {
        self.message = sanitize_message(value);
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("run_id", &self.run_id),
            ("source", &self.source),
            ("provider", &self.provider),
            ("model", &self.model),
            ("fallback_from", &self.fallback_from),
            ("job_ref", &self.job_ref),
        ] {
            if value
                .as_ref()
                .is_some_and(|value| sanitize_identifier(value).is_none())
            {
                return Err(format!("{name} is not a safe identifier"));
            }
        }

        if let Some(message) = &self.message {
            let count = message.chars().count();
            if count > 512 {
                return Err(format!("message is {count} characters; maximum is 512"));
            }
        }

        Ok(())
    }
}

pub struct LogEventVisitor {
    level: Level,
    fields: HashMap<&'static str, String>,
}

impl LogEventVisitor {
    pub fn new(level: Level) -> Self {
        Self {
            level,
            fields: HashMap::new(),
        }
    }

    fn insert(&mut self, field: &Field, value: String) {
        self.fields.insert(field.name(), value);
    }

    fn take(&mut self, name: &'static str) -> Option<String> {
        self.fields.remove(name)
    }

    pub fn finish(mut self) -> Option<LogEvent> {
        let event_type = EventType::parse(&self.take("event_type")?)?;
        let component = Component::parse(&self.take("component")?)?;
        let outcome = Outcome::parse(&self.take("outcome")?)?;
        let mut event = LogEvent::new(self.level, event_type, component, outcome);

        if let Some(value) = self.take("run_id") {
            event.run_id = sanitize_identifier(&value);
            event.run_id.as_ref()?;
        }
        if let Some(value) = self.take("source") {
            event.source = sanitize_identifier(&value);
            event.source.as_ref()?;
        }
        if let Some(value) = self.take("provider") {
            event.provider = sanitize_identifier(&value);
            event.provider.as_ref()?;
        }
        if let Some(value) = self.take("model") {
            event.model = sanitize_identifier(&value);
            event.model.as_ref()?;
        }
        if let Some(value) = self.take("fallback_from") {
            event.fallback_from = sanitize_identifier(&value);
            event.fallback_from.as_ref()?;
        }
        if let Some(value) = self.take("elapsed_ms") {
            event.elapsed_ms = value.parse().ok();
            event.elapsed_ms?;
        }
        if let Some(value) = self.take("error_category") {
            event.error_category = ErrorCategory::parse(&value);
            event.error_category?;
        }
        if let Some(value) = self.take("job_ref") {
            event.job_ref = sanitize_identifier(&value);
            event.job_ref.as_ref()?;
        }
        if let Some(value) = self.take("lifecycle_transition") {
            event.lifecycle_transition = LifecycleTransition::parse(&value);
            event.lifecycle_transition?;
        }
        if let Some(value) = self.take("dedup_count") {
            event.dedup_count = value.parse().ok();
            event.dedup_count?;
        }
        if let Some(value) = self.take("message") {
            event.message = sanitize_message(&value);
        }

        event.validate().ok()?;
        Some(event)
    }
}

impl Visit for LogEventVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert(field, value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert(field, value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert(field, value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let value = format!("{value:?}");
        self.insert(field, value.trim_matches('"').to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_levels_and_aliases() {
        assert_eq!(Level::parse_lossy("debug").0, Level::Debug);
        assert_eq!(Level::parse_lossy("warn").0, Level::Warn);
        assert_eq!(Level::parse_lossy("jobs_search=error").0, Level::Error);
        assert_eq!(Level::parse_lossy("").0, Level::Info);
        assert!(Level::parse_lossy("verbose").1);
    }

    #[test]
    fn builds_and_validates_event() {
        let event = LogEvent::new(
            Level::Info,
            EventType::SearchRunStart,
            Component::App,
            Outcome::Success,
        )
        .with_run_id("run-1")
        .with_message("search started");

        assert!(event.validate().is_ok());
        assert_eq!(event.run_id.as_deref(), Some("run-1"));
    }

    #[test]
    fn rejects_forbidden_identifiers() {
        let event = LogEvent::new(
            Level::Info,
            EventType::AppStart,
            Component::App,
            Outcome::Success,
        )
        .with_run_id("https://example.com/job");

        assert!(event.run_id.is_none());
        assert!(event.validate().is_ok());
    }
}
