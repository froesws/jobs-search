use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("telemetry error: {0}")]
    Telemetry(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("source error: {0}")]
    Source(String),
    #[error("llm provider error: {0}")]
    Provider(String),
    #[error("a search is already running")]
    SearchInProgress,
    #[error("a fixture profile is required; set JOBS_SEARCH_FIXTURE_PROFILE")]
    ProfileRequired,
    #[error("upload a resume before starting a new search")]
    ResumeRequired,
}

impl AppError {
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

    pub fn message(&self) -> String {
        self.to_string()
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

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

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: String,
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
