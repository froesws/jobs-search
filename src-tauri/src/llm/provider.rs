use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Serialize;

use crate::llm::schemas::EvaluationEvidence;
use crate::profile::CandidateProfile;
use crate::types::WorkModality;

#[derive(Debug, Clone)]
pub struct EvaluationJob {
    pub title: String,
    pub company: String,
    pub location: String,
    pub work_modality: WorkModality,
    pub description_clean: String,
}

#[derive(Debug, Clone)]
pub struct EvaluationInput {
    pub profile: CandidateProfile,
    pub job: EvaluationJob,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCategory {
    Timeout,
    RateLimit,
    Auth,
    Network,
    Parse,
    Config,
    Unknown,
}

impl ErrorCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "TIMEOUT",
            Self::RateLimit => "RATE_LIMIT",
            Self::Auth => "AUTH",
            Self::Network => "NETWORK",
            Self::Parse => "PARSE",
            Self::Config => "CONFIG",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LlmError {
    pub category: ErrorCategory,
    pub retryable: bool,
    pub message: String,
}

impl LlmError {
    pub fn new(category: ErrorCategory, retryable: bool, message: impl Into<String>) -> Self {
        Self {
            category,
            retryable,
            message: message.into(),
        }
    }

    pub fn auth(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Auth, false, message)
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Config, false, message)
    }

    pub fn parse(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Parse, true, message)
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Timeout, true, message)
    }

    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::RateLimit, true, message)
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Network, true, message)
    }
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn provider(&self) -> &'static str;
    fn model_for_attempt(&self, attempt: u32) -> String;

    async fn evaluate(
        &self,
        input: &EvaluationInput,
        attempt: u32,
    ) -> Result<EvaluationEvidence, LlmError>;
}

#[derive(Clone)]
pub struct ProviderPlan {
    pub provider: Arc<dyn LlmProvider>,
    pub attempts: u32,
}

impl std::fmt::Debug for ProviderPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderPlan")
            .field("provider", &self.provider.provider())
            .field("attempts", &self.attempts)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderAttempt {
    pub provider: String,
    pub model: String,
    pub latency_ms: i64,
    pub outcome: String,
    pub error_category: Option<String>,
    pub fallback_from: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EvaluationOutcome {
    pub evidence: EvaluationEvidence,
    pub provider: String,
    pub model: String,
    pub attempts: Vec<ProviderAttempt>,
}

#[derive(Debug, Clone)]
pub struct RouterError {
    pub category: ErrorCategory,
    pub message: String,
    pub fatal: bool,
    pub attempts: Vec<ProviderAttempt>,
    pub last_provider: Option<String>,
    pub last_model: Option<String>,
}

#[derive(Clone)]
pub struct LlmRouter {
    plans: Vec<ProviderPlan>,
    timeout: Duration,
}

impl LlmRouter {
    pub fn new(plans: Vec<ProviderPlan>, timeout: Duration) -> Self {
        Self { plans, timeout }
    }

    pub fn from_plans(plans: Vec<ProviderPlan>) -> Self {
        Self {
            plans,
            timeout: Duration::from_secs(60),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.plans.is_empty()
    }

    pub fn provider_names(&self) -> Vec<&'static str> {
        self.plans
            .iter()
            .map(|plan| plan.provider.provider())
            .collect()
    }

    pub async fn evaluate(
        &self,
        input: &EvaluationInput,
    ) -> Result<EvaluationOutcome, RouterError> {
        if self.plans.is_empty() {
            return Err(RouterError {
                category: ErrorCategory::Config,
                message: "no LLM provider is configured".to_string(),
                fatal: true,
                attempts: Vec::new(),
                last_provider: None,
                last_model: None,
            });
        }

        let mut attempts = Vec::new();
        let mut fallback_from: Option<String> = None;
        let mut last_provider = None;
        let mut last_model = None;

        for plan in &self.plans {
            let provider_name = plan.provider.provider().to_string();
            for attempt in 1..=plan.attempts.max(1) {
                let model = plan.provider.model_for_attempt(attempt);
                let started = Instant::now();
                let result =
                    tokio::time::timeout(self.timeout, plan.provider.evaluate(input, attempt))
                        .await;
                let latency_ms = started.elapsed().as_millis() as i64;

                match result {
                    Ok(Ok(evidence)) => {
                        attempts.push(ProviderAttempt {
                            provider: provider_name.clone(),
                            model: model.clone(),
                            latency_ms,
                            outcome: "SUCCESS".to_string(),
                            error_category: None,
                            fallback_from,
                        });
                        return Ok(EvaluationOutcome {
                            evidence,
                            provider: provider_name,
                            model,
                            attempts,
                        });
                    }
                    Ok(Err(error)) => {
                        attempts.push(ProviderAttempt {
                            provider: provider_name.clone(),
                            model: model.clone(),
                            latency_ms,
                            outcome: "ERROR".to_string(),
                            error_category: Some(error.category.as_str().to_string()),
                            fallback_from: fallback_from.clone(),
                        });
                        last_provider = Some(provider_name.clone());
                        last_model = Some(model.clone());
                        fallback_from = Some(format!("{provider_name}/{model}"));
                        if !error.retryable {
                            return Err(RouterError {
                                category: error.category,
                                message: error.message,
                                fatal: true,
                                attempts,
                                last_provider,
                                last_model,
                            });
                        }
                    }
                    Err(_) => {
                        attempts.push(ProviderAttempt {
                            provider: provider_name.clone(),
                            model: model.clone(),
                            latency_ms,
                            outcome: "ERROR".to_string(),
                            error_category: Some(ErrorCategory::Timeout.as_str().to_string()),
                            fallback_from: fallback_from.clone(),
                        });
                        last_provider = Some(provider_name.clone());
                        last_model = Some(model.clone());
                        fallback_from = Some(format!("{provider_name}/{model}"));
                    }
                }
            }
        }

        Err(RouterError {
            category: ErrorCategory::Unknown,
            message: "all configured LLM providers were exhausted".to_string(),
            fatal: false,
            attempts,
            last_provider,
            last_model,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::schemas::{EvaluationEvidence, Requirement, RequirementDimension};
    use crate::types::Seniority;

    fn evidence() -> EvaluationEvidence {
        EvaluationEvidence {
            seniority: "Pleno".to_string(),
            work_modality: WorkModality::Remote,
            location: "Remote - Brazil".to_string(),
            required_skills: vec!["Rust".to_string()],
            desirable_skills: vec![],
            requirements: vec![Requirement {
                id: "r1".to_string(),
                text: "Rust".to_string(),
                dimension: RequirementDimension::CoreTechnical,
                match_type: crate::llm::schemas::MatchType::Exact,
                evidence: "resume".to_string(),
            }],
            summary: "ok".to_string(),
        }
    }

    struct Fake {
        name: &'static str,
        fail: bool,
        fatal: bool,
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait]
    impl LlmProvider for Fake {
        fn provider(&self) -> &'static str {
            self.name
        }

        fn model_for_attempt(&self, _attempt: u32) -> String {
            "fake-model".to_string()
        }

        async fn evaluate(
            &self,
            _input: &EvaluationInput,
            _attempt: u32,
        ) -> Result<EvaluationEvidence, LlmError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.fail {
                if self.fatal {
                    Err(LlmError::auth("bad key"))
                } else {
                    Err(LlmError::rate_limit("429"))
                }
            } else {
                Ok(evidence())
            }
        }
    }

    fn input() -> EvaluationInput {
        EvaluationInput {
            profile: crate::profile::CandidateProfile {
                profile_id: "p".into(),
                version: 1,
                updated_at: "2026-01-01T00:00:00Z".into(),
                target_roles: vec!["Rust".into()],
                seniority_range: vec![Seniority::Pleno.as_str().into()],
                work_modalities: vec!["Remote".into()],
                accepted_cities: vec!["Porto Alegre".into()],
                acceptable_technologies: vec!["Rust".into()],
                core_competencies: vec!["Rust".into()],
                secondary_competencies: vec![],
                domain_experience: vec![],
                dealbreakers: vec![],
                mandatory_criteria: vec![],
                work_authorization: None,
                salary_threshold: None,
                source_documents: vec![],
            },
            job: EvaluationJob {
                title: "Rust".into(),
                company: "Acme".into(),
                location: "Remote".into(),
                work_modality: WorkModality::Remote,
                description_clean: "Rust".into(),
            },
        }
    }

    #[tokio::test]
    async fn falls_back_after_retryable_errors() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let router = LlmRouter::from_plans(vec![
            ProviderPlan {
                provider: Arc::new(Fake {
                    name: "first",
                    fail: true,
                    fatal: false,
                    calls: calls.clone(),
                }),
                attempts: 2,
            },
            ProviderPlan {
                provider: Arc::new(Fake {
                    name: "second",
                    fail: false,
                    fatal: false,
                    calls: calls.clone(),
                }),
                attempts: 1,
            },
        ]);
        let outcome = router.evaluate(&input()).await.expect("fallback");
        assert_eq!(outcome.provider, "second");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 3);
        assert_eq!(outcome.attempts.len(), 3);
    }

    #[tokio::test]
    async fn auth_errors_stop_without_fallback() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let router = LlmRouter::from_plans(vec![
            ProviderPlan {
                provider: Arc::new(Fake {
                    name: "first",
                    fail: true,
                    fatal: true,
                    calls: calls.clone(),
                }),
                attempts: 3,
            },
            ProviderPlan {
                provider: Arc::new(Fake {
                    name: "second",
                    fail: false,
                    fatal: false,
                    calls: calls.clone(),
                }),
                attempts: 1,
            },
        ]);
        let error = router.evaluate(&input()).await.expect_err("fatal");
        assert!(error.fatal);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
