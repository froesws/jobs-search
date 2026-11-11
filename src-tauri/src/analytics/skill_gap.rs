//! Skill-gap aggregation over eligible evaluations.

use std::collections::{BTreeMap, HashSet};

use crate::profile::CandidateProfile;

/// Canonical key for a skill: trimmed, case-folded, whitespace-collapsed, with
/// a small documented alias map for common market variants.
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

pub fn display_name(key: &str) -> String {
    match key {
        "kubernetes" => "Kubernetes".to_string(),
        "javascript" => "JavaScript".to_string(),
        "typescript" => "TypeScript".to_string(),
        "go" => "Go".to_string(),
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

#[allow(dead_code)]
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

#[allow(dead_code)]
fn counted_once(values: &[String]) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    for value in values {
        let key = normalize_skill(value);
        if !key.is_empty() {
            result
                .entry(key)
                .or_insert_with(|| value.trim().to_string());
        }
    }
    result
}
