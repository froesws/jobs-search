use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Strict LLM profile-extraction response from
/// `contracts/llm-profile-extraction.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractedProfile {
    pub core_competencies: Vec<String>,
    pub secondary_competencies: Vec<String>,
    #[serde(default)]
    pub inferred_competencies: Vec<String>,
    pub domain_experience: Vec<String>,
    pub work_history: Vec<WorkHistoryEntry>,
    pub education: Vec<EducationEntry>,
    pub certifications: Vec<String>,
    #[serde(default)]
    pub source_evidence: Vec<SourceEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkHistoryEntry {
    pub title: String,
    pub company: String,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EducationEntry {
    pub institution: String,
    pub degree: String,
    #[serde(default)]
    pub year: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEvidence {
    pub attribute: String,
    pub evidence: String,
}

impl ExtractedProfile {
    pub fn validate(&self) -> Result<(), AppError> {
        validate_list(&self.core_competencies, "core_competencies", 128)?;
        validate_list(&self.secondary_competencies, "secondary_competencies", 128)?;
        validate_list(&self.inferred_competencies, "inferred_competencies", 128)?;
        validate_list(&self.domain_experience, "domain_experience", 256)?;
        validate_list(&self.certifications, "certifications", 256)?;

        if self.work_history.len() > 50 {
            return Err(AppError::validation(
                "work_history cannot contain more than 50 entries",
            ));
        }
        for entry in &self.work_history {
            validate_text(&entry.title, "work_history.title", 256)?;
            validate_text(&entry.company, "work_history.company", 256)?;
            if let Some(end) = &entry.end {
                validate_text(end, "work_history.end", 32)?;
            }
            if entry.highlights.len() > 50 {
                return Err(AppError::validation(
                    "work_history.highlights cannot contain more than 50 entries",
                ));
            }
        }

        if self.education.len() > 20 {
            return Err(AppError::validation(
                "education cannot contain more than 20 entries",
            ));
        }
        for entry in &self.education {
            validate_text(&entry.institution, "education.institution", 256)?;
            validate_text(&entry.degree, "education.degree", 256)?;
        }

        if self.source_evidence.len() > 200 {
            return Err(AppError::validation(
                "source_evidence cannot contain more than 200 entries",
            ));
        }
        for evidence in &self.source_evidence {
            validate_text(&evidence.attribute, "source_evidence.attribute", 128)?;
            validate_text(&evidence.evidence, "source_evidence.evidence", 512)?;
        }

        Ok(())
    }
}

fn validate_list(values: &[String], field: &str, max: usize) -> Result<(), AppError> {
    if values.len() > 200 {
        return Err(AppError::validation(format!(
            "{field} cannot contain more than 200 entries"
        )));
    }
    let mut seen = HashSet::new();
    for value in values {
        validate_text(value, field, max)?;
        if !seen.insert(value.trim().to_ascii_lowercase()) {
            return Err(AppError::validation(format!(
                "{field} contains a duplicate value: {value}"
            )));
        }
    }
    Ok(())
}

fn validate_text(value: &str, field: &str, max: usize) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::validation(format!("{field} cannot be empty")));
    }
    if value.chars().count() > max {
        return Err(AppError::validation(format!(
            "{field} exceeds {max} characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extracted() -> ExtractedProfile {
        ExtractedProfile {
            core_competencies: vec!["Rust".into()],
            secondary_competencies: vec![],
            inferred_competencies: vec![],
            domain_experience: vec![],
            work_history: vec![],
            education: vec![],
            certifications: vec![],
            source_evidence: vec![],
        }
    }

    #[test]
    fn rejects_duplicates_and_empty_values() {
        let mut value = extracted();
        value.core_competencies = vec!["Rust".into(), "rust".into()];
        assert!(value.validate().is_err());

        let mut value = extracted();
        value.core_competencies = vec!["  ".into()];
        assert!(value.validate().is_err());
    }
}
