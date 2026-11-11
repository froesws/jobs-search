use jobs_search_lib::llm::schemas::{
    EvaluationEvidence, MatchType, Requirement, RequirementDimension,
};
use jobs_search_lib::matching::score::{compute, status_for};
use jobs_search_lib::types::{EvaluationStatus, WorkModality};

fn requirement(id: &str, dimension: RequirementDimension, matched: bool) -> Requirement {
    Requirement {
        id: id.to_string(),
        text: format!("requirement-{id}"),
        dimension,
        match_type: if matched {
            MatchType::Exact
        } else {
            MatchType::None
        },
        evidence: String::new(),
    }
}

fn evidence(requirements: Vec<Requirement>) -> EvaluationEvidence {
    EvaluationEvidence {
        seniority: "Pleno".to_string(),
        work_modality: WorkModality::Remote,
        location: "Remote - Brazil".to_string(),
        required_skills: Vec::new(),
        desirable_skills: Vec::new(),
        requirements,
        summary: "summary".to_string(),
    }
}

#[test]
fn score_is_deterministic_and_uses_40_25_20_15() {
    let all = compute(&evidence(vec![
        requirement("c", RequirementDimension::CoreTechnical, true),
        requirement("d", RequirementDimension::Domain, true),
        requirement("s", RequirementDimension::Seniority, true),
        requirement("x", RequirementDimension::Secondary, true),
    ]));
    assert_eq!(all.score, 100);
    assert_eq!(all.breakdown.core_technical.weight, 40);
    assert_eq!(all.breakdown.domain.weight, 25);
    assert_eq!(all.breakdown.seniority.weight, 20);
    assert_eq!(all.breakdown.secondary.weight, 15);

    let partial = compute(&evidence(vec![
        requirement("c1", RequirementDimension::CoreTechnical, true),
        requirement("c2", RequirementDimension::CoreTechnical, false),
        requirement("d", RequirementDimension::Domain, false),
    ]));
    assert_eq!(partial.score, 20);
    assert_eq!(partial.matched, 1);
    assert_eq!(partial.total, 3);
    assert_eq!(
        partial.matched + (partial.total - partial.matched),
        partial.total
    );
    assert_eq!(partial.breakdown.core_technical.contribution, 20.0);
}

#[test]
fn missing_dimension_contributes_zero_and_n_plus_missing_equals_m() {
    let result = compute(&evidence(vec![
        requirement("c", RequirementDimension::CoreTechnical, false),
        requirement("x", RequirementDimension::Secondary, true),
    ]));
    assert_eq!(result.breakdown.domain.total, 0);
    assert_eq!(result.breakdown.domain.contribution, 0.0);
    assert_eq!(result.matched, 1);
    assert_eq!(result.total, 2);
    assert_eq!(result.matched_keywords.len(), 1);
    assert_eq!(result.missing_keywords.len(), 1);
}

#[test]
fn threshold_is_inclusive_at_70() {
    assert_eq!(status_for(70, 70), EvaluationStatus::Approved);
    assert_eq!(status_for(69, 70), EvaluationStatus::Pending);
    assert_eq!(status_for(85, 70), EvaluationStatus::Approved);
}
