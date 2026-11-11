//! Application error type, wire payload, and conversion impls.

use serde::Serialize;

/// Application error with a stable wire code for the frontend.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Filesystem or stream failure.
    #[error("io error: {0}")]
    Io(String),
    /// Configuration load or shape failure.
    #[error("configuration error: {0}")]
    Config(String),
    /// Local log store failure.
    #[error("telemetry error: {0}")]
    Telemetry(String),
    /// SQLite failure.
    #[error("database error: {0}")]
    Database(String),
    /// JSON or wire-format failure.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// Operator input rejected.
    #[error("validation error: {0}")]
    Validation(String),
    /// Referenced entity does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// Job source failure.
    #[error("source error: {0}")]
    Source(String),
    /// LLM provider failure.
    #[error("llm provider error: {0}")]
    Provider(String),
    /// A search run is already active.
    #[error("a search is already running")]
    SearchInProgress,
    /// No profile is available and the fixture is not enabled.
    #[error("a fixture profile is required; set JOBS_SEARCH_FIXTURE_PROFILE")]
    ProfileRequired,
    /// No resume has been uploaded.
    #[error("upload a resume before starting a new search")]
    ResumeRequired,
}

impl AppError {
    /// Stable machine-readable code for the frontend.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "IO_ERROR",
            Self::Config(_) => "CONFIG_ERROR",
            Self::Telemetry(_) => "TELEMETRY_ERROR",
            Self::Database(_) => "DB_ERROR",
            Self::Serialization(_) => "SERIALIZATION_ERROR",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Source(_) => "SOURCE_ERROR",
            Self::Provider(_) => "PROVIDER_ERROR",
            Self::SearchInProgress => "SEARCH_IN_PROGRESS",
            Self::ProfileRequired => "PROFILE_REQUIRED",
            Self::ResumeRequired => "RESUME_REQUIRED",
        }
    }

    /// Human-readable message.
    pub fn message(&self) -> String {
        self.to_string()
    }

    /// Build a validation error from an operator input message.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    /// Build a not-found error for a named entity.
    pub fn not_found(entity: impl Into<String>) -> Self {
        Self::NotFound(entity.into())
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error.to_string())
    }
}

impl From<serde_yaml::Error> for AppError {
    fn from(error: serde_yaml::Error) -> Self {
        Self::Config(error.to_string())
    }
}

/// Wire payload sent to the frontend.
#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    /// Stable machine-readable code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

impl From<AppError> for ErrorPayload {
    fn from(error: AppError) -> Self {
        Self {
            code: error.code().to_string(),
            message: error.message(),
        }
    }
}
