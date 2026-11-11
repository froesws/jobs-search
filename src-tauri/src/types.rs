use serde::{Deserialize, Serialize};

/// Controlled work modality used by sources, storage, and matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum WorkModality {
    Remote,
    Hybrid,
    Onsite,
    #[default]
    Unknown,
}

impl WorkModality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Remote => "Remote",
            Self::Hybrid => "Hybrid",
            Self::Onsite => "Onsite",
            Self::Unknown => "Unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "remote" | "remoto" | "remota" => Self::Remote,
            "hybrid" | "híbrido" | "hibrido" | "hybrido" => Self::Hybrid,
            "onsite" | "on-site" | "on site" | "presencial" => Self::Onsite,
            _ => Self::Unknown,
        }
    }
}

/// Seniority values accepted by the operator profile and the hard filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Seniority {
    Trainee,
    Intern,
    Junior,
    Pleno,
    Senior,
}

impl Seniority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trainee => "Trainee",
            Self::Intern => "Intern",
            Self::Junior => "Junior",
            Self::Pleno => "Pleno",
            Self::Senior => "Senior",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "trainee" => Some(Self::Trainee),
            "intern" | "internship" | "estágio" | "estagio" => Some(Self::Intern),
            "junior" | "júnior" | "jr" => Some(Self::Junior),
            "pleno" | "mid" | "mid-level" | "midlevel" => Some(Self::Pleno),
            "senior" | "sênior" | "sr" => Some(Self::Senior),
            _ => None,
        }
    }
}

/// Search timeframe accepted by the command contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SearchTimeframe {
    H24,
    #[default]
    D7,
    D30,
}

impl SearchTimeframe {
    pub fn hours_old(self) -> u32 {
        match self {
            Self::H24 => 24,
            Self::D7 => 168,
            Self::D30 => 720,
        }
    }
}

/// Feed tab selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobTab {
    #[default]
    NewJobs,
    Applications,
    BelowThreshold,
}

/// Evaluation status is separate from the application lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvaluationStatus {
    Pending,
    Approved,
    Rejected,
    Error,
}

impl EvaluationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Approved => "APPROVED",
            Self::Rejected => "REJECTED",
            Self::Error => "ERROR",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "PENDING" => Some(Self::Pending),
            "APPROVED" => Some(Self::Approved),
            "REJECTED" => Some(Self::Rejected),
            "ERROR" => Some(Self::Error),
            _ => None,
        }
    }
}

/// Application lifecycle state, kept independent from evaluation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LifecycleState {
    Discovered,
    Applied,
    Ignored,
}

impl LifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "DISCOVERED",
            Self::Applied => "APPLIED",
            Self::Ignored => "IGNORED",
        }
    }
}

/// Sanitized hard-filter rejection stored with an evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardFilterRejection {
    pub code: String,
    pub message: String,
}

impl HardFilterRejection {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Stable identifier for a job posting. Wrapping the raw string prevents
/// confusing job IDs with run or resume IDs at the type level.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(String);

impl JobId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for JobId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for JobId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for JobId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Stable identifier for one execution run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(String);

impl RunId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for RunId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for RunId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RunId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Stable identifier for a stored resume file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResumeId(String);

impl ResumeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ResumeId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for ResumeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for ResumeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ResumeId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}
