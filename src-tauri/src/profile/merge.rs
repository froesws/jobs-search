//! Multi-resume merge with conflicts.
#![allow(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::db::now_rfc3339;
use crate::error::AppError;
use crate::profile::CandidateProfile;
use crate::profile::schema::{EducationEntry, ExtractedProfile, WorkHistoryEntry};

/// Extra profile data that is extracted from resumes but not used by the
/// hard-filter/scoring contract. Stored inside the profile JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileExtras {
    pub work_history: Vec<WorkHistoryEntry>,
    pub education: Vec<EducationEntry>,
    pub certifications: Vec<String>,
    pub inferred_competencies: Vec<String>,
    pub competency_sources: BTreeMap<String, Vec<String>>,
    pub conflicts: Vec<ProfileConflict>,
    /// False after the last stored resume is deleted. Searches are blocked
    /// until a new resume is uploaded or the operator re-verifies the profile.
    pub resumes_available: bool,
}

impl Default for ProfileExtras {
    fn default() -> Self {
        Self {
            work_history: Vec::new(),
            education: Vec::new(),
            certifications: Vec::new(),
            inferred_competencies: Vec::new(),
            competency_sources: BTreeMap::new(),
            conflicts: Vec::new(),
            resumes_available: true,
        }
    }
}

/// A scalar conflict that must be reviewed by the operator instead of being
/// silently overwritten.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileConflict {
    pub field: String,
    pub values: Vec<String>,
    pub sources: Vec<String>,
}

/// Merge an extracted resume into the single active profile.
///
/// List-valued fields are unioned case-insensitively. Existing operator fields
/// (manual technologies, cities, dealbreakers, target roles, authorization)
/// are preserved. Conflicting current-title values are recorded, never
/// overwritten.
#[must_use = "the merged profile must be persisted by the caller"]
pub fn merge_extracted(
    existing: Option<&CandidateProfile>,
    extracted: &ExtractedProfile,
    source_document: &str,
) -> Result<CandidateProfile, AppError> {
    extracted.validate()?;
    let mut profile = match existing {
        Some(profile) => profile.clone(),
        None => bootstrap(extracted),
    };

    profile.core_competencies = union(&profile.core_competencies, &extracted.core_competencies);
    profile.secondary_competencies = union(
        &profile.secondary_competencies,
        &extracted.secondary_competencies,
    );
    profile.domain_experience = union(&profile.domain_experience, &extracted.domain_experience);
    profile.extras.inferred_competencies = union(
        &profile.extras.inferred_competencies,
        &extracted.inferred_competencies,
    );
    profile.extras.certifications =
        union(&profile.extras.certifications, &extracted.certifications);
    record_current_title_conflict(&mut profile, extracted, source_document);
    profile.extras.work_history = extracted.work_history.clone();
    profile.extras.education = extracted.education.clone();

    if !profile
        .source_documents
        .iter()
        .any(|doc| doc == source_document)
    {
        profile.source_documents.push(source_document.to_string());
    }

    for competency in extracted
        .core_competencies
        .iter()
        .chain(extracted.secondary_competencies.iter())
        .chain(extracted.inferred_competencies.iter())
    {
        let sources = profile
            .extras
            .competency_sources
            .entry(competency.clone())
            .or_default();
        if !sources.iter().any(|doc| doc == source_document) {
            sources.push(source_document.to_string());
        }
    }

    profile.extras.resumes_available = true;
    profile.updated_at = now_rfc3339();
    profile.validate()?;
    Ok(profile)
}

fn bootstrap(extracted: &ExtractedProfile) -> CandidateProfile {
    let target_roles = extracted
        .work_history
        .first()
        .map(|entry| vec![entry.title.clone()])
        .unwrap_or_else(|| vec!["Software Engineer".to_string()]);

    CandidateProfile {
        profile_id: uuid::Uuid::new_v4().to_string(),
        version: 1,
        updated_at: now_rfc3339(),
        target_roles,
        seniority_range: vec!["Trainee".into(), "Junior".into(), "Pleno".into()],
        work_modalities: vec!["Remote".into(), "Hybrid".into(), "Onsite".into()],
        accepted_cities: vec!["Porto Alegre".into(), "Curitiba".into()],
        acceptable_technologies: Vec::new(),
        core_competencies: extracted.core_competencies.clone(),
        secondary_competencies: extracted.secondary_competencies.clone(),
        domain_experience: extracted.domain_experience.clone(),
        dealbreakers: Vec::new(),
        mandatory_criteria: vec!["location".into(), "modality".into()],
        work_authorization: None,
        salary_threshold: None,
        source_documents: vec![],
        extras: ProfileExtras {
            work_history: extracted.work_history.clone(),
            education: extracted.education.clone(),
            certifications: extracted.certifications.clone(),
            inferred_competencies: extracted.inferred_competencies.clone(),
            competency_sources: BTreeMap::new(),
            conflicts: Vec::new(),
            resumes_available: true,
        },
    }
}

fn record_current_title_conflict(
    profile: &mut CandidateProfile,
    extracted: &ExtractedProfile,
    source_document: &str,
) {
    let existing_title = profile
        .extras
        .work_history
        .first()
        .map(|entry| entry.title.clone());
    let extracted_title = extracted
        .work_history
        .first()
        .map(|entry| entry.title.clone());
    let (Some(old), Some(new)) = (existing_title, extracted_title) else {
        return;
    };
    if old.trim().eq_ignore_ascii_case(new.trim()) {
        return;
    }
    if profile
        .extras
        .conflicts
        .iter()
        .any(|conflict| conflict.field == "current_title" && conflict.values.contains(&new))
    {
        return;
    }
    profile.extras.conflicts.push(ProfileConflict {
        field: "current_title".to_string(),
        values: vec![old, new],
        sources: profile
            .source_documents
            .clone()
            .into_iter()
            .chain([source_document.to_string()])
            .collect(),
    });
}

fn union(left: &[String], right: &[String]) -> Vec<String> {
    let mut values = left.to_vec();
    let mut seen = values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect::<std::collections::HashSet<_>>();
    for value in right {
        let key = value.trim().to_ascii_lowercase();
        if !key.is_empty() && seen.insert(key) {
            values.push(value.clone());
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::schema::{ExtractedProfile, WorkHistoryEntry};

    fn extracted(title: &str, competencies: &[&str]) -> ExtractedProfile {
        ExtractedProfile {
            core_competencies: competencies.iter().map(|value| value.to_string()).collect(),
            secondary_competencies: vec!["EF SET C1".into()],
            inferred_competencies: vec!["Embedded C".into()],
            domain_experience: vec!["systems programming".into()],
            work_history: vec![WorkHistoryEntry {
                title: title.to_string(),
                company: "Acme".into(),
                start: "2024".into(),
                end: None,
                highlights: vec![],
            }],
            education: vec![],
            certifications: vec!["EF SET C1".into()],
            source_evidence: vec![],
        }
    }

    #[test]
    fn merges_lists_and_tracks_sources() {
        let first = merge_extracted(
            None,
            &extracted("Rust Engineer", &["Rust", "Linux"]),
            "resume-1",
        )
        .expect("merge");
        let second = merge_extracted(
            Some(&first),
            &extracted("Rust Engineer", &["Rust", "C#"]),
            "resume-2",
        )
        .expect("merge");

        assert_eq!(second.core_competencies, ["Rust", "Linux", "C#"]);
        assert_eq!(
            second
                .extras
                .competency_sources
                .get("Rust")
                .expect("source"),
            &["resume-1".to_string(), "resume-2".to_string()]
        );
        assert_eq!(second.source_documents.len(), 2);
    }

    #[test]
    fn flags_conflicting_current_titles_without_discarding_values() {
        let first = merge_extracted(None, &extracted("Rust Engineer", &["Rust"]), "resume-1")
            .expect("merge");
        let second = merge_extracted(
            Some(&first),
            &extracted("Staff Engineer", &["Rust"]),
            "resume-2",
        )
        .expect("merge");

        let conflict = &second.extras.conflicts[0];
        assert_eq!(conflict.field, "current_title");
        assert_eq!(conflict.values, ["Rust Engineer", "Staff Engineer"]);
    }

    #[test]
    fn preserves_operator_manual_fields() {
        let mut existing =
            merge_extracted(None, &extracted("Rust Engineer", &["Rust"]), "resume-1")
                .expect("merge");
        existing.acceptable_technologies = vec!["React".into()];
        existing.accepted_cities = vec!["Curitiba".into()];
        existing.dealbreakers = vec!["sales".into()];

        let merged = merge_extracted(
            Some(&existing),
            &extracted("Rust Engineer", &["C#"]),
            "resume-2",
        )
        .expect("merge");
        assert!(merged.technologies().contains(&"React".to_string()));
        assert_eq!(merged.accepted_cities, ["Curitiba"]);
        assert_eq!(merged.dealbreakers, ["sales"]);
    }
}
