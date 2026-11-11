use jobs_search_lib::db::Db;
use jobs_search_lib::db::applications::{
    apply_job, approve_job, events, get_application, ignore_job, restore_job,
};
use jobs_search_lib::db::evaluations::{EvaluationRecord, upsert_evaluation};
use jobs_search_lib::db::jobs::{get_detail, upsert_job};
use jobs_search_lib::db::runs::start_run;
use jobs_search_lib::types::{EvaluationStatus, WorkModality};

mod common;

fn approved_job(
    db: &Db,
    title: &str,
    status: EvaluationStatus,
    score: i64,
) -> jobs_search_lib::types::JobId {
    common::ensure_source(db, "src", "src", jobs_search_lib::config::SourceType::Api);
    let job = common::new_job(
        title,
        "Acme",
        "Remote - Brazil",
        WorkModality::Remote,
        "https://example.com/job",
        "Rust role",
    );
    let job_id = common::seed_job(db, "src", title, &job);
    let run = start_run(db, "{}").expect("run");
    upsert_evaluation(
        db,
        &EvaluationRecord {
            job_id: job_id.clone(),
            run_id: run.run_id,
            profile_version: 1,
            status,
            score: Some(score),
            matching_items: Some(3),
            total_requirements: Some(4),
            breakdown: Some(serde_json::json!({
                "core_technical": {"weight": 40, "matched": 3, "total": 4, "contribution": 30.0},
                "raw_total": 30.0,
                "score": score,
            })),
            matched_keywords: vec!["Rust".into()],
            missing_keywords: vec!["gRPC".into()],
            rejections: Vec::new(),
            provider: Some("fake".into()),
            model: Some("fake-model".into()),
            threshold: 70,
        },
    )
    .expect("evaluation");
    job_id
}

#[test]
fn applying_records_state_and_snapshot_without_duplicate_opportunity() {
    let db = common::test_db();
    let job_id = approved_job(&db, "Rust Engineer", EvaluationStatus::Approved, 92);

    let application = apply_job(&db, &job_id).expect("apply");
    assert_eq!(application.lifecycle_state, "APPLIED");
    assert!(application.applied_at.is_some());
    assert_eq!(application.approval_source, "automatic");

    let snapshot: serde_json::Value =
        serde_json::from_str(&application.evaluation_snapshot_json).expect("snapshot");
    assert_eq!(snapshot["match_score"], 92);
    assert_eq!(snapshot["matching_items_count"], 3);

    let stored = get_application(&db, &job_id)
        .expect("load")
        .expect("application");
    assert_eq!(stored.lifecycle_state, "APPLIED");

    // Re-ingesting the same canonical opportunity must not duplicate it.
    let job = common::new_job(
        "Rust Engineer",
        "Acme",
        "Remote - Brazil",
        WorkModality::Remote,
        "https://example.com/job",
        "Rust role updated",
    );
    let again = upsert_job(&db, &job).expect("re-ingest");
    assert_eq!(again.job_id, job_id);
    assert_eq!(
        get_application(&db, &job_id)
            .expect("load")
            .expect("application")
            .lifecycle_state,
        "APPLIED"
    );
}

#[test]
fn ignore_and_restore_append_events() {
    let db = common::test_db();
    let job_id = approved_job(&db, "Rust Engineer", EvaluationStatus::Approved, 88);

    let ignored = ignore_job(&db, &job_id).expect("ignore");
    assert_eq!(ignored.lifecycle_state, "IGNORED");
    let restored = restore_job(&db, &job_id).expect("restore");
    assert_eq!(restored.lifecycle_state, "DISCOVERED");

    let history = events(&db, &job_id).expect("events");
    let types: Vec<&str> = history
        .iter()
        .map(|event| event.event_type.as_str())
        .collect();
    assert!(types.contains(&"IGNORED"));
    assert!(types.contains(&"RESTORED"));
    assert!(types.contains(&"APPROVED"));
    assert!(history.len() >= 4);
}

#[test]
fn manual_approval_preserves_score_and_breakdown() {
    let db = common::test_db();
    let job_id = approved_job(&db, "Rust Engineer", EvaluationStatus::Pending, 58);
    let before = get_detail(&db, &job_id).expect("detail");
    let breakdown = before.score_breakdown.clone();

    let approved = approve_job(&db, &job_id).expect("approve");
    assert_eq!(approved.evaluation_status, "APPROVED");
    assert_eq!(approved.approval_source, "manual");
    assert_eq!(approved.match_score, Some(58));

    let after = get_detail(&db, &job_id).expect("detail");
    assert_eq!(after.summary.evaluation_status.as_deref(), Some("APPROVED"));
    assert_eq!(after.summary.match_score, Some(58));
    assert_eq!(after.score_breakdown, breakdown);
    assert!(
        events(&db, &job_id)
            .expect("events")
            .iter()
            .any(|event| event.event_type == "APPROVED"
                && event.reason.as_deref() == Some("manual approval"))
    );
}

#[test]
fn rejected_or_unscored_jobs_cannot_be_applied_or_approved() {
    let db = common::test_db();
    let rejected = approved_job(&db, "Sales Director", EvaluationStatus::Rejected, 20);
    assert!(apply_job(&db, &rejected).is_err());
    assert!(approve_job(&db, &rejected).is_err());
}
