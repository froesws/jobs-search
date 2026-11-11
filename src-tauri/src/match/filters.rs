//! Deterministic hard filters.
#![allow(missing_docs)]

use crate::db::jobs::StoredJob;
use crate::dedup::identity::normalize_text;
use crate::ingestion::normalize::{contains_word, detect_seniority, infer_modality};
use crate::profile::CandidateProfile;
use crate::types::{HardFilterRejection, Seniority, WorkModality};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchCriteria {
    pub seniorities: Vec<Seniority>,
    pub modalities: Vec<WorkModality>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardFilterResult {
    pub passed: bool,
    pub rejections: Vec<HardFilterRejection>,
}

impl HardFilterResult {
    pub fn passed() -> Self {
        Self {
            passed: true,
            rejections: Vec::new(),
        }
    }

    pub fn rejected(reasons: Vec<HardFilterRejection>) -> Self {
        Self {
            passed: false,
            rejections: reasons,
        }
    }
}

/// Deterministic hard filters applied before any LLM call.
pub fn check(profile: &CandidateProfile, job: &StoredJob) -> HardFilterResult {
    check_with_criteria(profile, job, &SearchCriteria::default())
}

/// Hard filters using the resolved search criteria. Empty criteria preserve
/// the profile/default behavior used by `check`.
pub fn check_with_criteria(
    profile: &CandidateProfile,
    job: &StoredJob,
    criteria: &SearchCriteria,
) -> HardFilterResult {
    let accepted_seniorities = if criteria.seniorities.is_empty() {
        profile.accepted_seniorities()
    } else {
        criteria.seniorities.clone()
    };
    let mut selected_modalities = Vec::new();
    let configured_modalities = if criteria.modalities.is_empty() {
        vec![
            WorkModality::Remote,
            WorkModality::Hybrid,
            WorkModality::Onsite,
        ]
    } else {
        criteria.modalities.clone()
    };
    for modality in configured_modalities {
        if !selected_modalities.contains(&modality) {
            selected_modalities.push(modality);
        }
    }
    let all_modalities_selected = selected_modalities.len() >= 3;
    let mut rejections = Vec::new();
    let text = format!("{} {} {}", job.title, job.location, job.description_clean);
    let normalized_text = normalize_text(&text);
    let modality = effective_modality(job, &text);
    let seniority = detect_seniority(&format!("{} {}", job.title, job.description_clean));

    if modality == WorkModality::Unknown {
        if !all_modalities_selected {
            rejections.push(HardFilterRejection::new(
                "MODALITY_NOT_ELIGIBLE",
                "posting modality is unknown and the selected filters are restricted",
            ));
        }
    } else if !selected_modalities.contains(&modality) {
        rejections.push(HardFilterRejection::new(
            "MODALITY_NOT_ELIGIBLE",
            format!(
                "modality {} is outside the selected filters",
                modality.as_str()
            ),
        ));
    }

    if !location_eligible(profile, job, &normalized_text, modality) {
        rejections.push(HardFilterRejection::new(
            "LOCATION_NOT_ELIGIBLE",
            "location is outside remote Brazil or Porto Alegre/Curitiba",
        ));
    }

    if let Some(seniority) = seniority
        && !accepted_seniorities.contains(&seniority)
    {
        rejections.push(HardFilterRejection::new(
            "SENIORITY_NOT_ELIGIBLE",
            format!(
                "seniority {} is outside the accepted range",
                seniority.as_str()
            ),
        ));
    }

    let trainee_like = matches!(seniority, Some(Seniority::Trainee | Seniority::Intern));
    // An empty description means the technology is unknown, not absent. JobSpy
    // and other metadata-only sources return no description, so rejecting on
    // the title alone would discard almost everything. Only judge the
    // technology when the posting actually has text to inspect.
    let has_description = !job.description_clean.trim().is_empty();
    if !trainee_like && has_description {
        let technologies = profile.technologies();
        let has_technology = technologies
            .iter()
            .any(|technology| contains_technology(&normalized_text, technology));
        if !has_technology {
            rejections.push(HardFilterRejection::new(
                "NO_CORE_TECHNOLOGY",
                "posting does not mention an accepted core technology",
            ));
        }
    }

    for dealbreaker in &profile.dealbreakers {
        if contains_word(&normalized_text, &normalize_text(dealbreaker)) {
            rejections.push(HardFilterRejection::new(
                "DEALBREAKER",
                "posting matches a profile dealbreaker",
            ));
            break;
        }
    }

    // Mandatory criteria never reject because a field is *unknown*: a missing
    // location, modality, or seniority is not a mismatch, and the dedicated
    // filters above already reject the values we can prove are wrong. Only a
    // domain keyword that is provably absent from a non-empty description is a
    // rejection.
    for criterion in &profile.mandatory_criteria {
        let key = criterion.trim().to_ascii_lowercase();
        if matches!(key.as_str(), "location" | "modality" | "seniority") {
            continue;
        }
        let Some(domain) = key.strip_prefix("domain:").map(str::trim) else {
            continue;
        };
        if domain.is_empty() || !has_description {
            continue;
        }
        if !contains_word(&normalized_text, &normalize_text(domain)) {
            rejections.push(HardFilterRejection::new(
                "UNKNOWN_MANDATORY_CRITERION",
                format!("posting does not mention mandatory domain: {domain}"),
            ));
        }
    }

    if rejections.is_empty() {
        HardFilterResult::passed()
    } else {
        HardFilterResult::rejected(rejections)
    }
}

pub fn effective_modality(job: &StoredJob, text: &str) -> WorkModality {
    if job.work_modality == WorkModality::Unknown {
        infer_modality(text, None)
    } else {
        job.work_modality
    }
}

fn location_eligible(
    profile: &CandidateProfile,
    job: &StoredJob,
    normalized_text: &str,
    modality: WorkModality,
) -> bool {
    let location = normalize_text(&job.location);
    let accepted_cities = profile
        .accepted_cities
        .iter()
        .map(|city| normalize_text(city))
        .collect::<Vec<_>>();

    match modality {
        WorkModality::Remote => {
            if mentions_brazil(&location) || mentions_brazil(normalized_text) {
                return true;
            }
            // A truly global remote posting is acceptable; an explicit
            // non-Brazil region is not.
            !mentions_restricted_region(&location)
        }
        WorkModality::Hybrid | WorkModality::Onsite => {
            accepted_cities.iter().any(|city| location.contains(city))
        }
        WorkModality::Unknown => {
            accepted_cities.iter().any(|city| location.contains(city)) || mentions_brazil(&location)
        }
    }
}

fn mentions_brazil(value: &str) -> bool {
    contains_word(value, "brazil")
        || contains_word(value, "brasil")
        || contains_word(value, "brazilian")
        || contains_word(value, "br")
        || value.contains("latam")
        || value.contains("latin america")
        || value.contains("south america")
        || value.contains("worldwide")
        || value.contains("anywhere")
        || value.contains("global")
}

fn mentions_restricted_region(value: &str) -> bool {
    [
        "usa",
        "united states",
        "us only",
        "canada",
        "europe",
        "emea",
        "united kingdom",
        "germany",
        "france",
        "india",
        "australia",
        "singapore",
        "japan",
        "uk",
        "us",
    ]
    .iter()
    .any(|region| contains_word(value, region))
}

fn contains_technology(normalized_text: &str, technology: &str) -> bool {
    let normalized_technology = technology.trim().to_ascii_lowercase();
    if normalized_technology.is_empty() {
        return false;
    }

    if normalized_technology.contains('+')
        || normalized_technology.contains('#')
        || normalized_technology.contains('.')
    {
        return format!(" {normalized_text} ").contains(&format!(" {normalized_technology} "))
            || normalized_text.contains(&normalized_technology);
    }

    contains_word(normalized_text, &normalized_technology)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WorkModality;

    fn profile() -> CandidateProfile {
        serde_json::from_str(include_str!("../../tests/fixtures/profile.json")).expect("fixture")
    }

    fn job(title: &str, location: &str, modality: WorkModality, description: &str) -> StoredJob {
        StoredJob {
            id: crate::types::JobId::from("job"),
            title: title.to_string(),
            company: "Acme".to_string(),
            location: location.to_string(),
            work_modality: modality,
            url: "https://example.com/1".to_string(),
            posted_at: None,
            description_clean: description.to_string(),
            extracted_metadata_json: None,
        }
    }

    #[test]
    fn accepts_remote_brazil_and_accepted_cities() {
        assert!(
            check(
                &profile(),
                &job(
                    "Rust Engineer",
                    "Remote - Brazil",
                    WorkModality::Remote,
                    "Rust"
                )
            )
            .passed
        );
        assert!(
            check(
                &profile(),
                &job(
                    "Rust Engineer",
                    "Porto Alegre, RS",
                    WorkModality::Onsite,
                    "Rust"
                )
            )
            .passed
        );
        assert!(
            check(
                &profile(),
                &job("Rust Engineer", "Curitiba", WorkModality::Hybrid, "Rust")
            )
            .passed
        );
    }

    #[test]
    fn rejects_onsite_outside_accepted_cities_and_foreign_remote() {
        assert!(
            !check(
                &profile(),
                &job("Rust Engineer", "São Paulo", WorkModality::Onsite, "Rust")
            )
            .passed
        );
        assert!(
            !check(
                &profile(),
                &job(
                    "Rust Engineer",
                    "Remote - USA",
                    WorkModality::Remote,
                    "Rust"
                )
            )
            .passed
        );
    }

    #[test]
    fn rejects_senior_only_but_accepts_pleno_senior_ranges() {
        assert!(
            !check(
                &profile(),
                &job(
                    "Senior Rust Engineer",
                    "Remote - Brazil",
                    WorkModality::Remote,
                    "Rust"
                )
            )
            .passed
        );
        assert!(
            check(
                &profile(),
                &job(
                    "Pleno/Sênior Rust Engineer",
                    "Remote - Brazil",
                    WorkModality::Remote,
                    "Rust"
                )
            )
            .passed
        );
    }

    #[test]
    fn rejects_missing_core_technology_except_trainee() {
        assert!(
            !check(
                &profile(),
                &job(
                    "Sales Manager",
                    "Remote - Brazil",
                    WorkModality::Remote,
                    "sales"
                )
            )
            .passed
        );
        assert!(
            check(
                &profile(),
                &job(
                    "Trainee Generalist",
                    "Remote - Brazil",
                    WorkModality::Remote,
                    "generalist"
                )
            )
            .passed
        );
    }

    #[test]
    fn brazil_and_foreign_markers_are_explicit() {
        assert!(mentions_brazil("remote brazil"));
        assert!(mentions_brazil("brasil"));
        assert!(mentions_brazil("brazilian"));
        assert!(mentions_brazil("br"));
        assert!(!mentions_brazil("worldwide"));
        assert!(!mentions_brazil("latam"));
        assert!(!mentions_brazil("anywhere"));

        assert!(mentions_foreign_region("remote worldwide"));
        assert!(mentions_foreign_region("buenos aires argentina"));
        assert!(mentions_foreign_region("us only"));
        assert!(!mentions_foreign_region("remote brazil"));
    }

    #[test]
    fn metadata_only_postings_are_not_rejected_for_missing_technology() {
        assert!(
            check(
                &profile(),
                &job(
                    "Analista de Desenvolvimento de Software Pleno",
                    "Brazil",
                    WorkModality::Unknown,
                    ""
                )
            )
            .passed
        );
    }

    #[test]
    fn empty_description_does_not_hide_known_mismatches() {
        let result = check(
            &profile(),
            &job(
                "Senior Rust Engineer",
                "Remote - Brazil",
                WorkModality::Remote,
                "",
            ),
        );
        assert!(!result.passed);
        assert!(
            result
                .rejections
                .iter()
                .any(|reason| reason.code == "SENIORITY_NOT_ELIGIBLE")
        );
    }

    #[test]
    fn empty_criteria_matches_default_check() {
        let profile = profile();
        let job = job(
            "Rust Engineer",
            "Remote - Brazil",
            WorkModality::Remote,
            "Rust",
        );
        assert_eq!(
            check(&profile, &job),
            check_with_criteria(&profile, &job, &SearchCriteria::default())
        );
    }

    #[test]
    fn rejects_dealbreakers() {
        let result = check(
            &profile(),
            &job(
                "Rust Engineer",
                "Remote - Brazil",
                WorkModality::Remote,
                "commission only sales role",
            ),
        );
        assert!(!result.passed);
        assert!(
            result
                .rejections
                .iter()
                .any(|reason| reason.code == "DEALBREAKER")
        );
    }
}
