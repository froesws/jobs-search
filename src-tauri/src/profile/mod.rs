pub mod fixture;
pub mod merge;
pub mod schema;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::types::Seniority;

/// The single active candidate profile used for matching.
///
/// The first milestone loads a synthetic fixture. Real PDF extraction and
/// merge/versioning belong to the P1 milestone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateProfile {
    pub profile_id: String,
    pub version: u32,
    pub updated_at: String,
    pub target_roles: Vec<String>,
    pub seniority_range: Vec<String>,
    pub work_modalities: Vec<String>,
    pub accepted_cities: Vec<String>,
    pub acceptable_technologies: Vec<String>,
    pub core_competencies: Vec<String>,
    #[serde(default)]
    pub secondary_competencies: Vec<String>,
    #[serde(default)]
    pub domain_experience: Vec<String>,
    #[serde(default)]
    pub dealbreakers: Vec<String>,
    #[serde(default)]
    pub work_authorization: Option<String>,
    #[serde(default)]
    pub salary_threshold: Option<i64>,
    #[serde(default)]
    pub source_documents: Vec<String>,
}

impl CandidateProfile {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.profile_id.trim().is_empty() {
            return Err(AppError::validation("profile_id is required"));
        }
        if self.version == 0 {
            return Err(AppError::validation("profile version must be >= 1"));
        }
        if self.target_roles.is_empty() {
            return Err(AppError::validation("target_roles cannot be empty"));
        }
        if self.accepted_seniorities().is_empty() {
            return Err(AppError::validation(
                "seniority_range must contain Trainee, Intern, Junior, Pleno, or Senior",
            ));
        }
        if self.accepted_cities.is_empty() {
            return Err(AppError::validation("accepted_cities cannot be empty"));
        }
        if self.technologies().is_empty() {
            return Err(AppError::validation(
                "at least one acceptable or core technology is required",
            ));
        }

        for role in &self.target_roles {
            if role.trim().is_empty() {
                return Err(AppError::validation(
                    "target_roles cannot contain empty values",
                ));
            }
        }

        Ok(())
    }

    pub fn accepted_seniorities(&self) -> Vec<Seniority> {
        let mut values = Vec::new();
        for value in &self.seniority_range {
            if let Some(seniority) = Seniority::parse(value)
                && !values.contains(&seniority)
            {
                values.push(seniority);
            }
        }
        values
    }

    /// Core technologies plus operator-maintained acceptable technologies.
    pub fn technologies(&self) -> Vec<String> {
        dedupe_case_insensitive(
            self.core_competencies
                .iter()
                .chain(self.acceptable_technologies.iter())
                .cloned(),
        )
    }

    pub fn all_competencies(&self) -> Vec<String> {
        dedupe_case_insensitive(
            self.core_competencies
                .iter()
                .chain(self.secondary_competencies.iter())
                .chain(self.acceptable_technologies.iter())
                .cloned(),
        )
    }

    /// Compact profile text used in LLM prompts. No identity fields exist in
    /// the fixture, and raw resume text is never sent.
    pub fn prompt_summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Target roles: {}", self.target_roles.join(", ")));
        lines.push(format!("Seniority: {}", self.seniority_range.join(", ")));
        lines.push(format!(
            "Accepted cities: {}",
            self.accepted_cities.join(", ")
        ));
        lines.push(format!(
            "Core technologies: {}",
            self.core_competencies.join(", ")
        ));
        lines.push(format!(
            "Acceptable technologies: {}",
            self.acceptable_technologies.join(", ")
        ));
        if !self.secondary_competencies.is_empty() {
            lines.push(format!(
                "Secondary competencies: {}",
                self.secondary_competencies.join(", ")
            ));
        }
        if !self.domain_experience.is_empty() {
            lines.push(format!(
                "Domain experience: {}",
                self.domain_experience.join(", ")
            ));
        }
        if !self.dealbreakers.is_empty() {
            lines.push(format!("Dealbreakers: {}", self.dealbreakers.join(", ")));
        }
        lines.join("\n")
    }
}

fn dedupe_case_insensitive(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for value in values {
        let normalized = value.trim().to_ascii_lowercase();
        if !normalized.is_empty() && seen.insert(normalized) {
            result.push(value);
        }
    }
    result
}
