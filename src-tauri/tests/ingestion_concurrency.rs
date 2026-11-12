use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use jobs_search_lib::db::Db;
use jobs_search_lib::llm::provider::{
    EvaluationInput, LlmError, LlmProvider, LlmRouter, ProviderPlan,
};
use jobs_search_lib::llm::schemas::{
    EvaluationEvidence, MatchType, Requirement, RequirementDimension,
};
use jobs_search_lib::matching::{EvaluationSettings, evaluate_run};
use jobs_search_lib::types::WorkModality;

mod common;

struct GaugeProvider {
    in_flight: Arc<AtomicUsize>,
    max_in_flight: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl LlmProvider for GaugeProvider {
    fn provider(&self) -> &'static str {
        "gauge"
    }

    fn model_for_attempt(&self, _attempt: u32) -> String {
        "gauge-model".to_string()
    }

    async fn evaluate(
        &self,
        input: &EvaluationInput,
        _attempt: u32,
    ) -> Result<EvaluationEvidence, LlmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let current = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_in_flight.fetch_max(current, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(25)).await;
        self.in_flight.fetch_sub(1, Ordering::SeqCst);

        if input.job.title.contains("Failure") {
            return Err(LlmError::rate_limit("rate limited"));
        }
        Ok(evidence(!input.job.title.contains("Below")))
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
        requirements.push(requirement("c2", RequirementDimension::CoreTechnical, true));
    }
    EvaluationEvidence {
        seniority: "Pleno".to_string(),
        work_modality: WorkModality::Remote,
        location: "Remote - Brazil".to_string(),
        required_skills: vec!["Rust".to_string()],
        desirable_skills: Vec::new(),
        requirements,
        summary: "fit".to_string(),
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

fn seeded_database(titles: &[String]) -> Db {
    let db = common::test_db();
    common::ensure_source(
        &db,
        "fake",
        "fake",
        jobs_search_lib::config::SourceType::Api,
    );
    for (index, title) in titles.iter().enumerate() {
        let url = format!("https://example.com/{index}");
        let job = common::new_job(
            title,
            "Acme",
            "Remote - Brazil",
            WorkModality::Remote,
            &url,
            "Rust systems",
        );
        common::seed_job(&db, "fake", &format!("fixture-{index}"), &job);
    }
    db
}

fn titles(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| format!("Rust Engineer {index}"))
        .collect()
}

async fn run(db: &Db, concurrency: usize) -> (jobs_search_lib::matching::EvaluationSummary, usize) {
    let profile = common::fixture_profile();
    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_in_flight = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let router = LlmRouter::from_plans(vec![ProviderPlan {
        provider: Arc::new(GaugeProvider {
            in_flight,
            max_in_flight: max_in_flight.clone(),
            calls,
        }),
        attempts: 1,
    }]);
    let run = jobs_search_lib::db::runs::start_run(db, "{}").expect("run");
    let settings = EvaluationSettings {
        threshold: 70,
        max_jobs: 100,
        concurrency,
        ..Default::default()
    };
    let summary = evaluate_run(db, &run.run_id, &profile, &router, &settings, None)
        .await
        .expect("evaluation");
    (summary, max_in_flight.load(Ordering::SeqCst))
}

#[tokio::test]
async fn observed_concurrency_never_exceeds_the_setting() {
    let db = seeded_database(&titles(12));
    let (summary, max) = run(&db, 3).await;
    assert_eq!(summary.evaluated, 12);
    assert_eq!(summary.approved, 12);
    assert_eq!(max, 3, "the queue should reach, never exceed, the setting");
}

#[tokio::test]
async fn a_serialized_run_produces_the_same_outcomes() {
    let serial = seeded_database(&titles(6));
    let (serial_summary, serial_max) = run(&serial, 1).await;
    let parallel = seeded_database(&titles(6));
    let (parallel_summary, parallel_max) = run(&parallel, 4).await;

    assert_eq!(serial_max, 1);
    assert_eq!(parallel_max, 4);
    assert_eq!(serial_summary.evaluated, parallel_summary.evaluated);
    assert_eq!(serial_summary.approved, parallel_summary.approved);
    assert_eq!(serial_summary.pending, parallel_summary.pending);
    assert_eq!(serial_summary.rejected, parallel_summary.rejected);
    assert_eq!(serial_summary.error, parallel_summary.error);
    assert_eq!(
        jobs_search_lib::db::jobs::status_counts(&serial).expect("serial"),
        jobs_search_lib::db::jobs::status_counts(&parallel).expect("parallel")
    );
}

#[tokio::test]
async fn a_rate_limit_error_preserves_the_completed_evaluations() {
    let mut titles = titles(5);
    titles.push("Rust Engineer Failure".to_string());
    let db = seeded_database(&titles);
    let (summary, _) = run(&db, 3).await;

    assert_eq!(summary.evaluated, 6);
    assert_eq!(summary.approved, 5);
    assert_eq!(summary.error, 1);
    assert_eq!(
        jobs_search_lib::db::jobs::status_counts(&db)
            .expect("statuses")
            .2,
        5
    );
}

#[tokio::test]
async fn concurrency_is_clamped_to_the_supported_range() {
    let zero = seeded_database(&titles(4));
    let (_, zero_max) = run(&zero, 0).await;
    assert_eq!(zero_max, 1);

    let high = seeded_database(&titles(16));
    let (_, high_max) = run(&high, 99).await;
    assert_eq!(high_max, 8);
}
