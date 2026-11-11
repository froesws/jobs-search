use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use jobs_search_lib::commands::search::SearchFilters;
use jobs_search_lib::db::evaluations::score_distribution;
use jobs_search_lib::db::jobs::{get_detail, list_summaries};
use jobs_search_lib::db::runs::start_run;
use jobs_search_lib::llm::provider::{
    EvaluationInput, LlmError, LlmProvider, LlmRouter, ProviderAttempt, ProviderPlan,
};
use jobs_search_lib::llm::schemas::{
    EvaluationEvidence, MatchType, Requirement, RequirementDimension,
};
use jobs_search_lib::matching::{EvaluationSettings, evaluate_run};
use jobs_search_lib::types::{EvaluationStatus, JobId, JobTab, Seniority, WorkModality};

mod common;

struct FakeProvider {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl LlmProvider for FakeProvider {
    fn provider(&self) -> &'static str {
        "fake"
    }

    fn model_for_attempt(&self, _attempt: u32) -> String {
        "fake-model".to_string()
    }

    async fn evaluate(
        &self,
        input: &EvaluationInput,
        _attempt: u32,
    ) -> Result<EvaluationEvidence, LlmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if input.job.title.contains("Failure") {
            return Err(LlmError::rate_limit("rate limited"));
        }
        let matched = !input.job.title.contains("Below");
        Ok(evidence(matched))
    }
}

fn evidence(full: bool) -> EvaluationEvidence {
    let mut requirements = vec![
        requirement("c1", RequirementDimension::CoreTechnical, full),
        requirement("d1", RequirementDimension::Domain, full),
        requirement("s1", RequirementDimension::Seniority, full),
        requirement("x1", RequirementDimension::Secondary, full),
    ];
    if !full {
        // One extra missed core requirement keeps the score below 70.
        requirements.push(requirement("c2", RequirementDimension::CoreTechnical, true));
    }
    EvaluationEvidence {
        seniority: "Pleno".to_string(),
        work_modality: WorkModality::Remote,
        location: "Remote - Brazil".to_string(),
        required_skills: vec!["Rust".to_string()],
        desirable_skills: vec!["Docker".to_string()],
        requirements,
        summary: if full { "Strong fit" } else { "Partial fit" }.to_string(),
    }
}

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
        evidence: "fixture".to_string(),
    }
}

fn seed(db: &jobs_search_lib::db::Db) {
    common::ensure_source(db, "fake", "fake", jobs_search_lib::config::SourceType::Api);
    let cases = [
        (
            "Rust Engineer",
            "Remote - Brazil",
            WorkModality::Remote,
            "Rust systems",
        ),
        (
            "Rust Engineer Below",
            "Remote - Brazil",
            WorkModality::Remote,
            "Rust systems",
        ),
        (
            "Senior Rust Engineer",
            "Remote - Brazil",
            WorkModality::Remote,
            "Rust systems",
        ),
        (
            "Rust Engineer Onsite",
            "São Paulo",
            WorkModality::Onsite,
            "Rust systems",
        ),
        (
            "Sales Manager",
            "Remote - Brazil",
            WorkModality::Remote,
            "sales role",
        ),
        (
            "Rust Engineer Failure",
            "Remote - Brazil",
            WorkModality::Remote,
            "Rust systems",
        ),
    ];
    for (index, (title, location, modality, description)) in cases.iter().enumerate() {
        let job = common::new_job(
            title,
            "Acme",
            location,
            *modality,
            &format!("https://example.com/{index}"),
            description,
        );
        common::seed_job(db, "fake", &format!("fixture-{index}"), &job);
    }
}

#[tokio::test]
async fn fixture_profile_evaluates_approved_pending_rejected_and_error_jobs() {
    let db = common::test_db();
    seed(&db);
    let profile = common::fixture_profile();
    let calls = Arc::new(AtomicUsize::new(0));
    let router = LlmRouter::from_plans(vec![ProviderPlan {
        provider: Arc::new(FakeProvider {
            calls: calls.clone(),
        }),
        attempts: 1,
    }]);
    let run = start_run(&db, "{}").expect("run");

    let summary = evaluate_run(
        &db,
        &run.run_id,
        &profile,
        &router,
        &EvaluationSettings {
            threshold: 70,
            max_jobs: 100,
            concurrency: 2,
            ..Default::default()
        },
    )
    .await
    .expect("evaluation");

    assert_eq!(summary.evaluated, 6);
    assert_eq!(summary.approved, 1);
    assert_eq!(summary.pending, 1);
    assert_eq!(summary.rejected, 3);
    assert_eq!(summary.error, 1);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert!(!summary.fatal_provider_error);

    let approved = list_summaries(&db, JobTab::NewJobs, 10, 0).expect("approved");
    assert_eq!(approved.total, 1);
    assert_eq!(approved.items[0].match_score, Some(100));
    assert_eq!(
        approved.items[0].evaluation_status.as_deref(),
        Some("APPROVED")
    );

    let pending = list_summaries(&db, JobTab::BelowThreshold, 10, 0).expect("pending");
    assert_eq!(pending.total, 1);
    assert_eq!(
        pending.items[0].evaluation_status.as_deref(),
        Some("PENDING")
    );

    let detail = get_detail(&db, &approved.items[0].job_id).expect("detail");
    assert_eq!(detail.provider.as_deref(), Some("fake"));
    assert_eq!(detail.model.as_deref(), Some("fake-model"));
    assert_eq!(detail.requirements.len(), 4);
    assert!(detail.score_breakdown.is_some());

    let distribution = score_distribution(&db).expect("distribution");
    assert_eq!(distribution.get("90-100"), Some(&1));
    assert_eq!(distribution.get("0-49"), Some(&1));

    let evaluations_for_run: i64 = db
        .with_connection(|connection| {
            Ok(connection.query_row(
                "SELECT COUNT(*) FROM job_evaluations WHERE run_id = ?1",
                [run.run_id.as_str()],
                |row| row.get(0),
            )?)
        })
        .expect("evaluations for run");
    assert_eq!(evaluations_for_run, 6);

    let metrics: i64 = db
        .with_connection(|connection| {
            Ok(connection.query_row(
                "SELECT COUNT(*) FROM llm_metrics WHERE run_id = ?1",
                [run.run_id.as_str()],
                |row| row.get(0),
            )?)
        })
        .expect("metrics");
    assert_eq!(metrics, 3);
}

#[test]
fn evaluation_defaults_are_bounded() {
    let settings = jobs_search_lib::matching::EvaluationSettings::default();
    assert_eq!(settings.threshold, 50);
    assert_eq!(settings.max_jobs, 100);
    assert_eq!(settings.concurrency, 2);
}

#[tokio::test]
async fn stale_evaluations_are_picked_up_again() {
    let db = common::test_db();
    seed(&db);
    let profile = common::fixture_profile();
    let router = LlmRouter::from_plans(vec![ProviderPlan {
        provider: Arc::new(FakeProvider {
            calls: Arc::new(AtomicUsize::new(0)),
        }),
        attempts: 1,
    }]);
    let run = start_run(&db, "{}").expect("run");
    let settings = EvaluationSettings {
        threshold: 70,
        max_jobs: 100,
        concurrency: 2,
        ..Default::default()
    };
    evaluate_run(&db, &run.run_id, &profile, &router, &settings)
        .await
        .expect("first");

    jobs_search_lib::db::evaluations::mark_stale(&db, "profile changed").expect("stale");
    assert_eq!(
        list_summaries(&db, JobTab::NewJobs, 10, 0)
            .expect("approved")
            .total,
        0
    );
    assert_eq!(
        list_summaries(&db, JobTab::BelowThreshold, 10, 0)
            .expect("pending")
            .total,
        0
    );
    let run = start_run(&db, "{}").expect("run");
    let summary = evaluate_run(&db, &run.run_id, &profile, &router, &settings)
        .await
        .expect("second");
    assert_eq!(summary.evaluated, 6);
    assert_eq!(
        list_summaries(&db, JobTab::NewJobs, 10, 0)
            .expect("approved")
            .total,
        1
    );
    let statuses = jobs_search_lib::db::jobs::status_counts(&db).expect("statuses");
    assert_eq!(statuses.2, 1);
    assert_eq!(statuses.3, 1);
    assert_eq!(
        jobs_search_lib::db::jobs::get_detail(
            &db,
            &list_summaries(&db, JobTab::NewJobs, 10, 0)
                .expect("approved")
                .items[0]
                .job_id
        )
        .expect("detail")
        .summary
        .evaluation_status
        .as_deref(),
        Some(EvaluationStatus::Approved.as_str())
    );
}

#[test]
fn provider_attempt_serializes_for_metrics() {
    let attempt = ProviderAttempt {
        provider: "fake".to_string(),
        model: "fake-model".to_string(),
        latency_ms: 1,
        outcome: "SUCCESS".to_string(),
        error_category: None,
        fallback_from: None,
    };
    let value = serde_json::to_value(attempt).expect("serialize");
    assert_eq!(value["provider"], "fake");
    assert_eq!(value["model"], "fake-model");
}

#[tokio::test]
async fn selected_panels_drive_query_plan_and_hard_filters() {
    let db = common::test_db();
    seed(&db);
    let profile = common::fixture_profile();
    let calls = Arc::new(AtomicUsize::new(0));
    let router = LlmRouter::from_plans(vec![ProviderPlan {
        provider: Arc::new(FakeProvider {
            calls: calls.clone(),
        }),
        attempts: 1,
    }]);
    let filters = SearchFilters {
        seniorities: vec![Seniority::Trainee, Seniority::Junior, Seniority::Pleno],
        modalities: vec![WorkModality::Remote, WorkModality::Hybrid],
        ..SearchFilters::default()
    }
    .resolve(&profile);
    let queries = filters.ingestion_queries();
    assert_eq!(queries.len(), 3);
    assert!(
        queries
            .iter()
            .all(|query| query.contains("(remoto OR híbrido)"))
    );

    let run = start_run(&db, "{}").expect("run");
    let summary = evaluate_run(
        &db,
        &run.run_id,
        &profile,
        &router,
        &EvaluationSettings {
            criteria: filters.criteria(),
            ..Default::default()
        },
    )
    .await
    .expect("evaluation");

    assert_eq!(summary.approved, 1);
    assert_eq!(summary.pending, 1);
    assert_eq!(summary.rejected, 3);
    assert_eq!(summary.error, 1);

    let onsite_id: String = db
        .with_connection(|connection| {
            Ok(connection.query_row(
                "SELECT id FROM job_postings WHERE title = 'Rust Engineer Onsite'",
                [],
                |row| row.get(0),
            )?)
        })
        .expect("onsite id");
    let detail = get_detail(&db, &JobId::from(onsite_id)).expect("detail");
    assert!(
        detail
            .hard_filter_rejections
            .iter()
            .any(|reason| reason.code == "MODALITY_NOT_ELIGIBLE")
    );
}
