use std::collections::BTreeMap;

use jobs_search_lib::db::jobs::{list_summaries, upsert_job};
use jobs_search_lib::db::runs::{RunFinish, finish_run, get_audit, start_run};
use jobs_search_lib::types::{JobTab, WorkModality};

mod common;

#[test]
fn audit_reconciles_with_the_run_records() {
    let db = common::test_db();
    let run = start_run(&db, r#"{"timeframe":"D7"}"#).expect("run");
    let mut rejected = BTreeMap::new();
    rejected.insert("SENIORITY_NOT_ELIGIBLE".to_string(), 2);
    let mut scores = BTreeMap::new();
    scores.insert("70-79".to_string(), 1);
    let mut sources = BTreeMap::new();
    sources.insert("jobspy".to_string(), 5);

    finish_run(
        &db,
        &RunFinish {
            run_id: &run.run_id,
            status: "PARTIAL",
            total_ingested: 7,
            duplicates_merged: 2,
            rejected_count: 2,
            pending_count: 1,
            approved_count: 3,
            error_count: 1,
            source_failures: vec!["jobspy: timeout".to_string()],
            rejected_by_reason: rejected.clone(),
            score_distribution: scores.clone(),
            source_breakdown: sources.clone(),
        },
    )
    .expect("finish");

    let audit = get_audit(&db, &run.run_id).expect("audit");
    assert_eq!(audit.run_id, run.run_id);
    assert_eq!(audit.status, "PARTIAL");
    assert_eq!(audit.total_ingested, 7);
    assert_eq!(audit.duplicates_merged, 2);
    assert_eq!(audit.rejected_count, 2);
    assert_eq!(audit.pending_count, 1);
    assert_eq!(audit.approved_count, 3);
    assert_eq!(audit.error_count, 1);
    assert_eq!(audit.rejected_by_reason, rejected);
    assert_eq!(audit.score_distribution, scores);
    assert_eq!(audit.source_breakdown, sources);
    assert_eq!(audit.source_failures, ["jobspy: timeout"]);
    assert_eq!(audit.search_config["timeframe"], "D7");
    assert!(audit.finished_at.is_some());
    assert_eq!(
        audit.approved_count + audit.pending_count + audit.rejected_count,
        6
    );
}

#[test]
fn audit_returns_not_found_for_an_unknown_run() {
    let db = common::test_db();
    let error =
        get_audit(&db, &jobs_search_lib::types::RunId::from("missing")).expect_err("missing run");
    assert_eq!(error.code(), "NOT_FOUND");
}

#[test]
fn zero_approved_jobs_return_an_empty_page() {
    let db = common::test_db();
    let page = list_summaries(&db, JobTab::NewJobs, 20, 0).expect("page");
    assert!(page.items.is_empty());
    assert_eq!(page.total, 0);
}

#[test]
fn approved_jobs_are_sorted_by_score_then_core_contribution() {
    let db = common::test_db();
    common::ensure_source(&db, "src", "src", jobs_search_lib::config::SourceType::Api);
    let run = start_run(&db, "{}").expect("run");

    for (title, score, core) in [("Low", 80, 20.0), ("High", 90, 30.0), ("Tie", 80, 35.0)] {
        let job = common::new_job(
            title,
            "Acme",
            "Remote - Brazil",
            WorkModality::Remote,
            "https://example.com/job",
            "Rust",
        );
        let job_id = common::seed_job(&db, "src", title, &job);
        jobs_search_lib::db::evaluations::upsert_evaluation(
            &db,
            &jobs_search_lib::db::evaluations::EvaluationRecord {
                job_id,
                run_id: run.run_id.clone(),
                profile_version: 1,
                status: jobs_search_lib::types::EvaluationStatus::Approved,
                score: Some(score),
                matching_items: Some(1),
                total_requirements: Some(1),
                breakdown: Some(serde_json::json!({
                    "core_technical": {"contribution": core},
                })),
                matched_keywords: Vec::new(),
                missing_keywords: Vec::new(),
                rejections: Vec::new(),
                provider: None,
                model: None,
                threshold: 70,
            },
        )
        .expect("evaluation");
    }

    let page = list_summaries(&db, JobTab::NewJobs, 10, 0).expect("page");
    let titles: Vec<&str> = page.items.iter().map(|job| job.title.as_str()).collect();
    assert_eq!(titles, ["High", "Tie", "Low"]);
}

#[test]
fn ranked_feed_excludes_stale_evaluations() {
    let db = common::test_db();
    common::ensure_source(&db, "src", "src", jobs_search_lib::config::SourceType::Api);
    let job = common::new_job(
        "Rust Engineer",
        "Acme",
        "Remote - Brazil",
        WorkModality::Remote,
        "https://example.com/job",
        "Rust",
    );
    let job_id = common::seed_job(&db, "src", "one", &job);
    let run = start_run(&db, "{}").expect("run");
    jobs_search_lib::db::evaluations::upsert_evaluation(
        &db,
        &jobs_search_lib::db::evaluations::EvaluationRecord {
            job_id,
            run_id: run.run_id,
            profile_version: 1,
            status: jobs_search_lib::types::EvaluationStatus::Approved,
            score: Some(95),
            matching_items: Some(1),
            total_requirements: Some(1),
            breakdown: None,
            matched_keywords: Vec::new(),
            missing_keywords: Vec::new(),
            rejections: Vec::new(),
            provider: None,
            model: None,
            threshold: 70,
        },
    )
    .expect("evaluation");

    jobs_search_lib::db::evaluations::mark_stale(&db, "profile updated").expect("stale");
    assert_eq!(
        list_summaries(&db, JobTab::NewJobs, 10, 0)
            .expect("page")
            .total,
        0
    );

    // A stale job is picked up again by evaluation.
    assert!(upsert_job(&db, &job).is_ok());
}
