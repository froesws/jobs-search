pub mod filters;
pub mod score;

use std::collections::BTreeMap;

use futures::StreamExt;

use crate::db::Db;
use crate::db::evaluations::EvaluationRecord;
use crate::db::jobs::StoredJob;
use crate::error::AppError;
use crate::llm::provider::{EvaluationInput, EvaluationJob, LlmRouter};
use crate::profile::CandidateProfile;
use crate::types::EvaluationStatus;

#[derive(Debug, Clone)]
pub struct EvaluationSettings {
    pub threshold: i64,
    pub max_jobs: usize,
    pub concurrency: usize,
}

impl Default for EvaluationSettings {
    fn default() -> Self {
        Self {
            threshold: 70,
            max_jobs: 100,
            concurrency: 2,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EvaluationSummary {
    pub evaluated: i64,
    pub rejected: i64,
    pub pending: i64,
    pub approved: i64,
    pub error: i64,
    pub fatal_provider_error: bool,
    pub rejected_by_reason: BTreeMap<String, i64>,
}

#[derive(Debug, Clone)]
struct JobOutcome {
    status: EvaluationStatus,
    rejection_codes: Vec<String>,
    fatal: bool,
}

#[tracing::instrument(skip(db, profile, router), fields(run_id, max_jobs = settings.max_jobs))]
pub async fn evaluate_run(
    db: &Db,
    run_id: &str,
    profile: &CandidateProfile,
    router: &LlmRouter,
    settings: &EvaluationSettings,
) -> Result<EvaluationSummary, AppError> {
    let jobs = crate::db::jobs::list_for_evaluation(db, settings.max_jobs.clamp(1, 1_000))?;
    let concurrency = settings.concurrency.clamp(1, 4);
    let threshold = settings.threshold;

    let db = db.clone();
    let profile = profile.clone();
    let router = router.clone();
    let run_id = run_id.to_string();

    let outcomes = futures::stream::iter(jobs.into_iter().map(move |job| {
        let db = db.clone();
        let profile = profile.clone();
        let router = router.clone();
        let run_id = run_id.clone();
        async move { evaluate_one(&db, &run_id, &profile, &router, job, threshold).await }
    }))
    .buffer_unordered(concurrency)
    .collect::<Vec<_>>()
    .await;

    let mut summary = EvaluationSummary {
        evaluated: outcomes.len() as i64,
        ..EvaluationSummary::default()
    };

    for outcome in outcomes {
        let outcome = outcome?;
        match outcome.status {
            EvaluationStatus::Approved => summary.approved += 1,
            EvaluationStatus::Pending => summary.pending += 1,
            EvaluationStatus::Rejected => summary.rejected += 1,
            EvaluationStatus::Error => summary.error += 1,
        }
        if outcome.fatal {
            summary.fatal_provider_error = true;
        }
        for code in outcome.rejection_codes {
            *summary.rejected_by_reason.entry(code).or_insert(0) += 1;
        }
    }

    Ok(summary)
}

async fn evaluate_one(
    db: &Db,
    run_id: &str,
    profile: &CandidateProfile,
    router: &LlmRouter,
    job: StoredJob,
    threshold: i64,
) -> Result<JobOutcome, AppError> {
    let hard_filters = filters::check(profile, &job);
    let job_id = job.id.clone();
    let profile_version = profile.version as i64;
    if !hard_filters.passed {
        let codes = hard_filters
            .rejections
            .iter()
            .map(|reason| reason.code.clone())
            .collect::<Vec<_>>();
        crate::db::evaluations::upsert_evaluation(
            db,
            &EvaluationRecord {
                job_id: job_id.clone(),
                run_id: run_id.to_string(),
                profile_version,
                status: EvaluationStatus::Rejected,
                score: None,
                matching_items: None,
                total_requirements: None,
                breakdown: None,
                matched_keywords: Vec::new(),
                missing_keywords: Vec::new(),
                rejections: hard_filters.rejections,
                provider: None,
                model: None,
                threshold,
            },
        )?;
        return Ok(JobOutcome {
            status: EvaluationStatus::Rejected,
            rejection_codes: codes,
            fatal: false,
        });
    }

    let input = EvaluationInput {
        profile: profile.clone(),
        job: EvaluationJob {
            title: job.title,
            company: job.company,
            location: job.location,
            work_modality: job.work_modality,
            description_clean: job.description_clean,
        },
    };

    match router.evaluate(&input).await {
        Ok(outcome) => {
            record_attempts(db, run_id, &outcome.attempts)?;
            let scored = score::compute(&outcome.evidence);
            let status = score::status_for(scored.score, threshold);

            let metadata = serde_json::json!({
                "seniority": outcome.evidence.seniority,
                "work_modality": outcome.evidence.work_modality,
                "location": outcome.evidence.location,
                "required_skills": outcome.evidence.required_skills,
                "desirable_skills": outcome.evidence.desirable_skills,
                "requirements": outcome.evidence.requirements,
                "summary": outcome.evidence.summary,
            });
            crate::db::jobs::update_extracted_metadata(db, &job_id, &metadata.to_string())?;

            crate::db::evaluations::upsert_evaluation(
                db,
                &EvaluationRecord {
                    job_id: job_id.clone(),
                    run_id: run_id.to_string(),
                    profile_version,
                    status,
                    score: Some(scored.score),
                    matching_items: Some(scored.matched),
                    total_requirements: Some(scored.total),
                    breakdown: Some(serde_json::to_value(&scored.breakdown)?),
                    matched_keywords: scored.matched_keywords,
                    missing_keywords: scored.missing_keywords,
                    rejections: Vec::new(),
                    provider: Some(outcome.provider),
                    model: Some(outcome.model),
                    threshold,
                },
            )?;

            Ok(JobOutcome {
                status,
                rejection_codes: Vec::new(),
                fatal: false,
            })
        }
        Err(error) => {
            record_attempts(db, run_id, &error.attempts)?;
            crate::db::evaluations::upsert_evaluation(
                db,
                &EvaluationRecord {
                    job_id: job_id.clone(),
                    run_id: run_id.to_string(),
                    profile_version,
                    status: EvaluationStatus::Error,
                    score: None,
                    matching_items: None,
                    total_requirements: None,
                    breakdown: Some(serde_json::json!({
                        "error_category": error.category.as_str(),
                    })),
                    matched_keywords: Vec::new(),
                    missing_keywords: Vec::new(),
                    rejections: Vec::new(),
                    provider: error.last_provider,
                    model: error.last_model,
                    threshold,
                },
            )?;

            Ok(JobOutcome {
                status: EvaluationStatus::Error,
                rejection_codes: Vec::new(),
                fatal: error.fatal,
            })
        }
    }
}

fn record_attempts(
    db: &Db,
    run_id: &str,
    attempts: &[crate::llm::provider::ProviderAttempt],
) -> Result<(), AppError> {
    for attempt in attempts {
        crate::db::runs::record_llm_metric(
            db,
            run_id,
            &attempt.provider,
            &attempt.model,
            "job_evaluation",
            attempt.latency_ms,
            &attempt.outcome,
            attempt.error_category.as_deref(),
            attempt.fallback_from.as_deref(),
        )?;
    }
    Ok(())
}
