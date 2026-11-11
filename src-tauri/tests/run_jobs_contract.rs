use jobs_search_lib::db::evaluations::{EvaluationRecord, upsert_evaluation};
use jobs_search_lib::db::jobs::{list_run_jobs, run_job_rows};
use jobs_search_lib::db::runs::start_run;
use jobs_search_lib::types::{EvaluationStatus, HardFilterRejection, WorkModality};

mod common;

fn seed_run_job(
    db: &jobs_search_lib::db::Db,
    run_id: &str,
    title: &str,
    status: EvaluationStatus,
    score: Option<i64>,
    rejections: Vec<HardFilterRejection>,
) {
    let job = common::new_job(
        title,
        "Acme",
        "Remote - Brazil",
        WorkModality::Remote,
        &format!("https://example.com/{}", title.replace(' ', "-")),
        "Rust and C++",
    );
    let job_id = common::seed_job(db, "jobspy", title, &job);
    upsert_evaluation(
        db,
        &EvaluationRecord {
            job_id: job_id.clone(),
            run_id: jobs_search_lib::types::RunId::from(run_id),
            profile_version: 1,
            status,
            score,
            matching_items: Some(1),
            total_requirements: Some(2),
            breakdown: None,
            matched_keywords: vec!["Rust".to_string()],
            missing_keywords: vec![],
            rejections,
            provider: Some("deepseek".to_string()),
            model: Some("deepseek-flash".to_string()),
            threshold: 50,
        },
    )
    .expect("evaluation");
}

#[test]
fn run_jobs_are_scoped_to_the_run_and_ordered_by_score() {
    let db = common::test_db();
    common::ensure_source(
        &db,
        "jobspy",
        "jobspy",
        jobs_search_lib::config::SourceType::Mcp,
    );
    let run = start_run(&db, r#"{"timeframe":"D7"}"#).expect("run");
    let other = start_run(&db, r#"{"timeframe":"D7"}"#).expect("other run");

    seed_run_job(
        &db,
        run.run_id.as_ref(),
        "Low Score",
        EvaluationStatus::Pending,
        Some(40),
        vec![],
    );
    seed_run_job(
        &db,
        run.run_id.as_ref(),
        "High Score",
        EvaluationStatus::Approved,
        Some(90),
        vec![],
    );
    seed_run_job(
        &db,
        run.run_id.as_ref(),
        "Rejected One",
        EvaluationStatus::Rejected,
        None,
        vec![HardFilterRejection::new(
            "SENIORITY_NOT_ELIGIBLE",
            "seniority not eligible",
        )],
    );
    seed_run_job(
        &db,
        other.run_id.as_ref(),
        "Other Run",
        EvaluationStatus::Approved,
        Some(95),
        vec![],
    );

    let page = list_run_jobs(&db, run.run_id.as_ref(), 10, 0).expect("page");
    assert_eq!(page.total, 3);
    assert_eq!(page.items.len(), 3);
    assert_eq!(page.items[0].title, "High Score");
    assert_eq!(page.items[1].title, "Low Score");
    assert_eq!(page.items[2].title, "Rejected One");
    assert_eq!(
        page.items[2].rejection_reason.as_deref(),
        Some("seniority not eligible")
    );
    assert!(
        !page.items.iter().any(|item| item.title == "Other Run"),
        "jobs from another run must not leak in"
    );

    let second = list_run_jobs(&db, run.run_id.as_ref(), 2, 2).expect("page 2");
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].title, "Rejected One");

    let all = run_job_rows(&db, run.run_id.as_ref()).expect("rows");
    assert_eq!(all.len(), 3);
}
