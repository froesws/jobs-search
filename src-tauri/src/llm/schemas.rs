use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::profile::schema::ExtractedProfile;
use crate::types::WorkModality;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum RequirementDimension {
    CoreTechnical,
    Domain,
    Seniority,
    Secondary,
}

impl RequirementDimension {
    pub fn weight(self) -> i64 {
        match self {
            Self::CoreTechnical => 40,
            Self::Domain => 25,
            Self::Seniority => 20,
            Self::Secondary => 15,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::CoreTechnical => "CORE_TECHNICAL",
            Self::Domain => "DOMAIN",
            Self::Seniority => "SENIORITY",
            Self::Secondary => "SECONDARY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum MatchType {
    Exact,
    Inferred,
    None,
}

impl MatchType {
    pub fn is_match(self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "EXACT",
            Self::Inferred => "INFERRED",
            Self::None => "NONE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub id: String,
    pub text: String,
    pub dimension: RequirementDimension,
    pub match_type: MatchType,
    #[serde(default)]
    pub evidence: String,
}

/// Strict LLM evaluation boundary. The app, not the model, computes the score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationEvidence {
    pub seniority: String,
    pub work_modality: WorkModality,
    pub location: String,
    pub required_skills: Vec<String>,
    pub desirable_skills: Vec<String>,
    pub requirements: Vec<Requirement>,
    pub summary: String,
}

impl EvaluationEvidence {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.seniority.chars().count() > 64 {
            return Err(AppError::validation("seniority exceeds 64 characters"));
        }
        if self.location.chars().count() > 256 {
            return Err(AppError::validation("location exceeds 256 characters"));
        }
        if self.summary.chars().count() > 512 {
            return Err(AppError::validation("summary exceeds 512 characters"));
        }
        validate_skills(&self.required_skills, "required_skills")?;
        validate_skills(&self.desirable_skills, "desirable_skills")?;
        if self.requirements.len() > 200 {
            return Err(AppError::validation(
                "requirements cannot contain more than 200 entries",
            ));
        }

        let mut ids = HashSet::new();
        for requirement in &self.requirements {
            if requirement.id.trim().is_empty() || requirement.id.chars().count() > 64 {
                return Err(AppError::validation(
                    "requirement id must contain 1-64 characters",
                ));
            }
            if !ids.insert(requirement.id.clone()) {
                return Err(AppError::validation(format!(
                    "duplicate requirement id: {}",
                    requirement.id
                )));
            }
            if requirement.text.trim().is_empty() || requirement.text.chars().count() > 512 {
                return Err(AppError::validation(
                    "requirement text must contain 1-512 characters",
                ));
            }
            if requirement.evidence.chars().count() > 512 {
                return Err(AppError::validation(
                    "requirement evidence exceeds 512 characters",
                ));
            }
        }

        Ok(())
    }

    pub fn matched_keywords(&self) -> Vec<String> {
        dedupe(
            self.requirements
                .iter()
                .filter(|requirement| requirement.match_type.is_match())
                .map(|requirement| requirement.text.clone()),
        )
    }

    pub fn missing_keywords(&self) -> Vec<String> {
        dedupe(
            self.requirements
                .iter()
                .filter(|requirement| !requirement.match_type.is_match())
                .map(|requirement| requirement.text.clone()),
        )
    }

    pub fn dimension_counts(&self, dimension: RequirementDimension) -> (i64, i64) {
        let mut matched = 0;
        let mut total = 0;
        for requirement in &self.requirements {
            if requirement.dimension == dimension {
                total += 1;
                if requirement.match_type.is_match() {
                    matched += 1;
                }
            }
        }
        (matched, total)
    }
}

fn validate_skills(skills: &[String], field: &str) -> Result<(), AppError> {
    if skills.len() > 100 {
        return Err(AppError::validation(format!(
            "{field} cannot contain more than 100 entries"
        )));
    }
    let mut seen = HashSet::new();
    for skill in skills {
        let trimmed = skill.trim();
        if trimmed.is_empty() || skill.chars().count() > 128 {
            return Err(AppError::validation(format!(
                "{field} entries must contain 1-128 characters"
            )));
        }
        if !seen.insert(trimmed.to_ascii_lowercase()) {
            return Err(AppError::validation(format!(
                "{field} contains a duplicate entry"
            )));
        }
    }
    Ok(())
}

fn dedupe(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for value in values {
        let key = value.trim().to_ascii_lowercase();
        if !key.is_empty() && seen.insert(key) {
            result.push(value);
        }
    }
    result
}

/// Parse a model response, tolerating markdown code fences and surrounding
/// prose while still rejecting schema-invalid payloads.
pub fn parse_llm_json(text: &str) -> Result<EvaluationEvidence, AppError> {
    let start = text
        .find('{')
        .ok_or_else(|| AppError::validation("LLM response did not contain a JSON object"))?;
    let end = text
        .rfind('}')
        .ok_or_else(|| AppError::validation("LLM response did not contain a JSON object"))?;
    if end <= start {
        return Err(AppError::validation(
            "LLM response contained an invalid JSON object",
        ));
    }
    let evidence: EvaluationEvidence = serde_json::from_str(&text[start..=end])?;
    evidence.validate()?;
    Ok(evidence)
}

pub fn parse_extracted_profile(text: &str) -> Result<ExtractedProfile, AppError> {
    let start = text
        .find('{')
        .ok_or_else(|| AppError::validation("LLM response did not contain a JSON object"))?;
    let end = text
        .rfind('}')
        .ok_or_else(|| AppError::validation("LLM response did not contain a JSON object"))?;
    if end <= start {
        return Err(AppError::validation(
            "LLM response contained an invalid JSON object",
        ));
    }
    let profile: ExtractedProfile = serde_json::from_str(&text[start..=end])?;
    profile.validate()?;
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn evidence() -> serde_json::Value {
        json!({
            "seniority": "Pleno",
            "work_modality": "Remote",
            "location": "Remote - Brazil",
            "required_skills": ["Rust"],
            "desirable_skills": ["Docker"],
            "requirements": [
                {
                    "id": "r1",
                    "text": "Rust",
                    "dimension": "CORE_TECHNICAL",
                    "match_type": "EXACT",
                    "evidence": "five years"
                }
            ],
            "summary": "Strong fit"
        })
    }

    #[test]
    fn parses_fenced_json() {
        let text = format!("```json\n{}\n```", evidence());
        let parsed = parse_llm_json(&text).expect("valid");
        assert_eq!(parsed.requirements.len(), 1);
        assert_eq!(parsed.matched_keywords(), ["Rust"]);
    }

    #[test]
    fn rejects_unknown_fields() {
        let mut value = evidence();
        value["extra"] = json!(true);
        assert!(parse_llm_json(&value.to_string()).is_err());
    }

    #[test]
    fn rejects_duplicate_requirement_ids() {
        let mut value = evidence();
        let requirement = value["requirements"][0].clone();
        value["requirements"]
            .as_array_mut()
            .unwrap()
            .push(requirement);
        assert!(parse_llm_json(&value.to_string()).is_err());
    }
}
