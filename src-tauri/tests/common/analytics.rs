use std::collections::BTreeMap;

use jobs_search_lib::db::Db;
use jobs_search_lib::db::evaluations::{EvaluationRecord, upsert_evaluation};
use jobs_search_lib::db::runs::{RunFinish, finish_run, start_run};
use jobs_search_lib::types::{EvaluationStatus, WorkModality};

use super::{ensure_source, new_job, seed_job};

pub struct AnalyticsSeed {
    pub run_id: jobs_search_lib::types::RunId,
    pub eligible: usize,
}

/// Seed eligible jobs. The first `kubernetes` jobs have Kubernetes missing.
pub fn seed_analytics(db: &Db, eligible: usize, kubernetes: usize) -> AnalyticsSeed {
    ensure_source(db, "src", "src", jobs_search_lib::config::SourceType::Api);
    let run = start_run(db, r#"{"timeframe":"D7"}"#).expect("run");

    for index in 0..eligible {
        let title = format!("Rust Engineer {index}");
        let job = new_job(
            &title,
            "Acme",
            "Remote - Brazil",
            WorkModality::Remote,
            &format!("https://example.com/{index}"),
            "Rust systems work",
        );
        let job_id = seed_job(db, "src", &title, &job);
        let missing = if index < kubernetes {
            vec!["Kubernetes".to_string()]
        } else {
            Vec::new()
        };
        upsert_evaluation(
            db,
            &EvaluationRecord {
                job_id,
                run_id: run.run_id.clone(),
                profile_version: 1,
                status: EvaluationStatus::Approved,
                score: Some(80),
                matching_items: Some(1),
                total_requirements: Some(1),
                breakdown: Some(serde_json::json!({
                    "core_technical": {"weight": 40, "matched": 1, "total": 1, "contribution": 40.0},
                    "raw_total": 40.0,
                    "score": 80,
                })),
                matched_keywords: vec!["Rust".to_string()],
                missing_keywords: missing,
                rejections: Vec::new(),
                provider: Some("deepseek".to_string()),
                model: Some("deepseek-chat".to_string()),
                threshold: 70,
            },
        )
        .expect("evaluation");
    }

    let mut rejected = BTreeMap::new();
    rejected.insert("SENIORITY_NOT_ELIGIBLE".to_string(), 1);
    let mut scores = BTreeMap::new();
    scores.insert("80-89".to_string(), eligible as i64);
    finish_run(
        db,
        &RunFinish {
            run_id: &run.run_id,
            status: "SUCCESS",
            total_ingested: eligible as i64,
            duplicates_merged: 0,
            rejected_count: 1,
            pending_count: 0,
            approved_count: eligible as i64,
            error_count: 0,
            source_failures: Vec::new(),
            rejected_by_reason: rejected,
            score_distribution: scores,
            source_breakdown: BTreeMap::from([("src".to_string(), eligible as i64)]),
        },
    )
    .expect("finish run");

    AnalyticsSeed {
        run_id: run.run_id,
        eligible,
    }
}
