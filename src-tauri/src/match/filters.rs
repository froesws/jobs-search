use crate::db::jobs::StoredJob;
use crate::dedup::identity::normalize_text;
use crate::ingestion::normalize::{contains_word, detect_seniority, infer_modality};
use crate::profile::CandidateProfile;
use crate::types::{HardFilterRejection, Seniority, WorkModality};

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
    let mut rejections = Vec::new();
    let text = format!("{} {} {}", job.title, job.location, job.description_clean);
    let normalized_text = normalize_text(&text);
    let modality = effective_modality(job, &text);
    let seniority = detect_seniority(&format!("{} {}", job.title, job.description_clean));

    if !location_eligible(profile, job, &normalized_text, modality) {
        rejections.push(HardFilterRejection::new(
            "LOCATION_NOT_ELIGIBLE",
            "location is outside remote Brazil or Porto Alegre/Curitiba",
        ));
    }

    if let Some(seniority) = seniority
        && !profile.accepted_seniorities().contains(&seniority)
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
    if !trainee_like {
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

    for criterion in &profile.mandatory_criteria {
        let key = criterion.trim().to_ascii_lowercase();
        let known = match key.as_str() {
            "location" => {
                normalize_text(&job.location) != "unknown"
                    && !normalize_text(&job.location).is_empty()
            }
            "modality" => modality != WorkModality::Unknown,
            "seniority" => seniority.is_some(),
            _ => key
                .strip_prefix("domain:")
                .map(str::trim)
                .is_some_and(|domain| {
                    !domain.is_empty() && contains_word(&normalized_text, &normalize_text(domain))
                }),
        };
        if !known {
            rejections.push(HardFilterRejection::new(
                "UNKNOWN_MANDATORY_CRITERION",
                format!("mandatory criterion is unknown: {criterion}"),
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
