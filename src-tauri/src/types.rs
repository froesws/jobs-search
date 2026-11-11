//! Core domain types shared across the Tauri command boundary.

use serde::{Deserialize, Serialize};

/// Controlled work modality used by sources, storage, and matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub enum WorkModality {
    /// Fully remote work.
    Remote,
    /// Remote plus office days.
    Hybrid,
    /// On-site presence required.
    Onsite,
    /// Modality could not be determined.
    #[default]
    Unknown,
}

impl WorkModality {
    /// Stable display string used for storage and matching.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Remote => "Remote",
            Self::Hybrid => "Hybrid",
            Self::Onsite => "Onsite",
            Self::Unknown => "Unknown",
        }
    }

    /// Parse from a raw source value, falling back to Unknown.
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "remote" | "remoto" | "remota" => Self::Remote,
            "hybrid" | "híbrido" | "hibrido" | "hybrido" => Self::Hybrid,
            "onsite" | "on-site" | "on site" | "presencial" => Self::Onsite,
            _ => Self::Unknown,
        }
    }

    /// Portuguese term used in external source queries. This is separate from
    /// the stable storage/display value returned by `as_str`.
    pub fn search_term_pt(self) -> Option<&'static str> {
        match self {
            Self::Remote => Some("remoto"),
            Self::Hybrid => Some("híbrido"),
            Self::Onsite => Some("presencial"),
            Self::Unknown => None,
        }
    }
}

/// Seniority values accepted by the operator profile and the hard filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub enum Seniority {
    /// Trainee level.
    Trainee,
    /// Internship/estágio.
    Intern,
    /// Junior level.
    Junior,
    /// Mid level (pleno).
    Pleno,
    /// Senior level.
    Senior,
}

impl Seniority {
    /// Stable display string used for storage and matching.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trainee => "Trainee",
            Self::Intern => "Intern",
            Self::Junior => "Junior",
            Self::Pleno => "Pleno",
            Self::Senior => "Senior",
        }
    }

    /// Parse from a raw source value, returning None when unrecognized.
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

    /// Portuguese term used in external source queries. This is separate from
    /// the stable storage/display value returned by `as_str`.
    pub fn search_term_pt(self) -> &'static str {
        match self {
            Self::Trainee => "trainee",
            Self::Intern => "estágio",
            Self::Junior => "júnior",
            Self::Pleno => "pleno",
            Self::Senior => "sênior",
        }
    }
}

/// Search timeframe accepted by the command contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum SearchTimeframe {
    /// Last 24 hours.
    H24,
    /// Last 7 days (default).
    #[default]
    D7,
    /// Last 30 days.
    D30,
}

impl SearchTimeframe {
    /// The window length in hours.
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
#[non_exhaustive]
pub enum JobTab {
    /// Approved matches feed.
    #[default]
    NewJobs,
    /// Applied jobs.
    Applications,
    /// Pending evaluations below the threshold.
    BelowThreshold,
}

/// Evaluation status is separate from the application lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum EvaluationStatus {
    /// Scored below the approval threshold.
    Pending,
    /// Passed hard filters and the threshold.
    Approved,
    /// Failed a hard filter.
    Rejected,
    /// Evaluation could not complete.
    Error,
}

impl EvaluationStatus {
    /// Wire representation stored in the database.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Approved => "APPROVED",
            Self::Rejected => "REJECTED",
            Self::Error => "ERROR",
        }
    }

    /// Parse the wire representation, returning None when unrecognized.
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
#[non_exhaustive]
pub enum LifecycleState {
    /// Approved and awaiting a decision.
    Discovered,
    /// Marked as applied.
    Applied,
    /// Hidden by the operator.
    Ignored,
}

impl LifecycleState {
    /// Wire representation stored in the database.
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
    /// Machine-readable reason code.
    pub code: String,
    /// Sanitized human-readable reason.
    pub message: String,
}

impl HardFilterRejection {
    /// Build a rejection from a reason code and a sanitized message.
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
    /// Create from the raw string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the raw string.
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
    /// Create from the raw string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the raw string.
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
    /// Create from the raw string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the raw string.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_portuguese_search_terms_without_changing_display_values() {
        assert_eq!(Seniority::Trainee.search_term_pt(), "trainee");
        assert_eq!(Seniority::Intern.search_term_pt(), "estágio");
        assert_eq!(Seniority::Junior.search_term_pt(), "júnior");
        assert_eq!(Seniority::Pleno.search_term_pt(), "pleno");
        assert_eq!(Seniority::Senior.search_term_pt(), "sênior");
        assert_eq!(WorkModality::Remote.search_term_pt(), Some("remoto"));
        assert_eq!(WorkModality::Hybrid.search_term_pt(), Some("híbrido"));
        assert_eq!(WorkModality::Onsite.search_term_pt(), Some("presencial"));
        assert_eq!(WorkModality::Unknown.search_term_pt(), None);

        assert_eq!(Seniority::Senior.as_str(), "Senior");
        assert_eq!(WorkModality::Remote.as_str(), "Remote");
    }
}
