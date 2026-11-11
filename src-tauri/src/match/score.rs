//! Deterministic 40/25/20/15 scoring.
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

use crate::llm::schemas::{EvaluationEvidence, RequirementDimension};
use crate::types::EvaluationStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub weight: i64,
    pub matched: i64,
    pub total: i64,
    pub contribution: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub core_technical: DimensionScore,
    pub domain: DimensionScore,
    pub seniority: DimensionScore,
    pub secondary: DimensionScore,
    pub raw_total: f64,
    pub score: i64,
}

#[derive(Debug, Clone)]
pub struct ScoreResult {
    pub score: i64,
    pub matched: i64,
    pub total: i64,
    pub breakdown: ScoreBreakdown,
    pub matched_keywords: Vec<String>,
    pub missing_keywords: Vec<String>,
}

/// Deterministic 40/25/20/15 score. A dimension with no extracted
/// requirements contributes zero, and matched + missing always equals total.
pub fn compute(evidence: &EvaluationEvidence) -> ScoreResult {
    let core = dimension(evidence, RequirementDimension::CoreTechnical);
    let domain = dimension(evidence, RequirementDimension::Domain);
    let seniority = dimension(evidence, RequirementDimension::Seniority);
    let secondary = dimension(evidence, RequirementDimension::Secondary);

    let raw_total =
        core.contribution + domain.contribution + seniority.contribution + secondary.contribution;
    let score = raw_total.round().clamp(0.0, 100.0) as i64;
    let matched = core.matched + domain.matched + seniority.matched + secondary.matched;
    let total = core.total + domain.total + seniority.total + secondary.total;

    ScoreResult {
        score,
        matched,
        total,
        breakdown: ScoreBreakdown {
            core_technical: core,
            domain,
            seniority,
            secondary,
            raw_total,
            score,
        },
        matched_keywords: evidence.matched_keywords(),
        missing_keywords: evidence.missing_keywords(),
    }
}

fn dimension(evidence: &EvaluationEvidence, dimension: RequirementDimension) -> DimensionScore {
    let (matched, total) = evidence.dimension_counts(dimension);
    let weight = dimension.weight();
    let contribution = if total == 0 {
        0.0
    } else {
        weight as f64 * matched as f64 / total as f64
    };
    DimensionScore {
        weight,
        matched,
        total,
        contribution,
    }
}

pub fn status_for(score: i64, threshold: i64) -> EvaluationStatus {
    if score >= threshold {
        EvaluationStatus::Approved
    } else {
        EvaluationStatus::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::schemas::{MatchType, Requirement};
    use crate::types::WorkModality;

    fn evidence(requirements: Vec<Requirement>) -> EvaluationEvidence {
        EvaluationEvidence {
            seniority: "Pleno".to_string(),
            work_modality: WorkModality::Remote,
            location: "Remote - Brazil".to_string(),
            required_skills: vec![],
            desirable_skills: vec![],
            requirements,
            summary: "summary".to_string(),
        }
    }

    fn requirement(id: &str, dimension: RequirementDimension, matched: bool) -> Requirement {
        Requirement {
            id: id.to_string(),
            text: format!("requirement {id}"),
            dimension,
            match_type: if matched {
                MatchType::Exact
            } else {
                MatchType::None
            },
            evidence: String::new(),
        }
    }

    #[test]
    fn all_matched_is_100() {
        let result = compute(&evidence(vec![
            requirement("a", RequirementDimension::CoreTechnical, true),
            requirement("b", RequirementDimension::Domain, true),
            requirement("c", RequirementDimension::Seniority, true),
            requirement("d", RequirementDimension::Secondary, true),
        ]));
        assert_eq!(result.score, 100);
        assert_eq!(result.matched, 4);
        assert_eq!(result.total, 4);
    }

    #[test]
    fn no_matched_is_zero() {
        let result = compute(&evidence(vec![
            requirement("a", RequirementDimension::CoreTechnical, false),
            requirement("b", RequirementDimension::Domain, false),
        ]));
        assert_eq!(result.score, 0);
        assert_eq!(result.matched, 0);
        assert_eq!(result.total, 2);
    }

    #[test]
    fn weights_are_40_25_20_15() {
        let result = compute(&evidence(vec![
            requirement("a", RequirementDimension::CoreTechnical, true),
            requirement("b", RequirementDimension::Domain, true),
            requirement("c", RequirementDimension::Seniority, true),
            requirement("d", RequirementDimension::Secondary, true),
            requirement("e", RequirementDimension::CoreTechnical, false),
        ]));
        assert_eq!(result.breakdown.core_technical.weight, 40);
        assert_eq!(result.breakdown.domain.weight, 25);
        assert_eq!(result.breakdown.seniority.weight, 20);
        assert_eq!(result.breakdown.secondary.weight, 15);
        assert_eq!(result.score, 80); // 20 + 25 + 20 + 15
        assert_eq!(result.matched, 4);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn threshold_boundary_is_inclusive() {
        assert_eq!(status_for(70, 70), EvaluationStatus::Approved);
        assert_eq!(status_for(69, 70), EvaluationStatus::Pending);
    }
}
