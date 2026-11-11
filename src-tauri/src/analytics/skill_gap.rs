//! Skill-gap aggregation over eligible evaluations.

use std::collections::{BTreeMap, HashSet};

use crate::analytics::{EligibleJob, SkillGapMetric};
use crate::profile::CandidateProfile;

/// Canonical key for a skill: trimmed, case-folded, whitespace-collapsed, with
/// a small documented alias map for common market variants.
#[must_use]
pub fn normalize_skill(value: &str) -> String {
    let collapsed = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    match collapsed.as_str() {
        "k8s" => "kubernetes".to_string(),
        "js" => "javascript".to_string(),
        "ts" => "typescript".to_string(),
        "golang" => "go".to_string(),
        "node" | "nodejs" => "node.js".to_string(),
        "postgres" => "postgresql".to_string(),
        "react.js" | "reactjs" => "react".to_string(),
        "c sharp" | "csharp" => "c#".to_string(),
        "dotnet" => ".net".to_string(),
        _ => collapsed,
    }
}

#[must_use]
pub fn display_name(key: &str) -> String {
    match key {
        "kubernetes" => "Kubernetes".to_string(),
        "javascript" => "JavaScript".to_string(),
        "typescript" => "TypeScript".to_string(),
        "go" => "Go".to_string(),
        "grpc" => "gRPC".to_string(),
        "node.js" => "Node.js".to_string(),
        "postgresql" => "PostgreSQL".to_string(),
        "react" => "React".to_string(),
        "c#" => "C#".to_string(),
        ".net" => ".NET".to_string(),
        _ => {
            let mut characters = key.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                None => key.to_string(),
            }
        }
    }
}

fn profile_skills(profile: Option<&CandidateProfile>) -> HashSet<String> {
    profile
        .map(|profile| {
            profile
                .all_competencies()
                .into_iter()
                .map(|skill| normalize_skill(&skill))
                .collect()
        })
        .unwrap_or_default()
}

/// Aggregate missing-skill demand over the eligible jobs.
#[must_use]
pub fn compute(jobs: &[EligibleJob], profile: Option<&CandidateProfile>) -> Vec<SkillGapMetric> {
    let excluded = profile_skills(profile);
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let mut displays: BTreeMap<String, String> = BTreeMap::new();

    for job in jobs {
        let matched = job
            .matched_keywords
            .iter()
            .map(|value| normalize_skill(value))
            .collect::<HashSet<_>>();
        let mut counted = HashSet::new();
        for missing in &job.missing_keywords {
            let key = normalize_skill(missing);
            if key.is_empty()
                || excluded.contains(&key)
                || matched.contains(&key)
                || !counted.insert(key.clone())
            {
                continue;
            }
            *counts.entry(key.clone()).or_insert(0) += 1;
            displays
                .entry(key)
                .or_insert_with(|| display_name(&normalize_skill(missing)));
        }
    }

    let denominator = jobs.len() as f64;
    let mut metrics = counts
        .into_iter()
        .map(|(key, demand_count)| SkillGapMetric {
            skill: key.clone(),
            display_name: displays.remove(&key).unwrap_or_else(|| display_name(&key)),
            demand_count,
            demand_percentage: if denominator == 0.0 {
                0.0
            } else {
                round_one_decimal(demand_count as f64 / denominator * 100.0)
            },
            rank: 0,
        })
        .collect::<Vec<_>>();

    metrics.sort_by(|left, right| {
        right.demand_count.cmp(&left.demand_count).then_with(|| {
            left.display_name
                .to_lowercase()
                .cmp(&right.display_name.to_lowercase())
        })
    });
    for (index, metric) in metrics.iter_mut().enumerate() {
        metric.rank = (index + 1) as u32;
    }
    metrics
}

fn round_one_decimal(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
