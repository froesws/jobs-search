pub mod config;
pub mod event;
pub mod health;
pub mod queue;
pub mod sanitize;
pub mod writer;

use std::thread::JoinHandle;
use std::time::Duration;

use tracing::{Event, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use crate::error::AppError;
pub use config::TelemetryConfig;
use event::{Level, LogEventVisitor};
use health::{HealthState, LogHealth};
use queue::BoundedQueue;
use writer::WriterHandle;

pub struct Telemetry;

impl Telemetry {
    pub fn app_start(version: &str, session_id: &str) {
        tracing::info!(
            event_type = "APP_START",
            component = "app",
            outcome = "SUCCESS",
            message = %format!("version={version} session={session_id}")
        );
    }

    pub fn app_shutdown() {
        tracing::info!(
            event_type = "APP_SHUTDOWN",
            component = "app",
            outcome = "SUCCESS",
            message = "shutdown"
        );
    }

    pub fn config_warning(message: &str) {
        tracing::warn!(
            event_type = "APP_START",
            component = "telemetry",
            outcome = "PARTIAL",
            message = %message
        );
    }

    pub fn search_run_start(run_id: &str) {
        tracing::info!(
            event_type = "SEARCH_RUN_START",
            component = "app",
            run_id = %run_id,
            outcome = "SUCCESS",
            message = "search started"
        );
    }

    pub fn search_run_end(run_id: &str, outcome: &str) {
        tracing::info!(
            event_type = "SEARCH_RUN_END",
            component = "app",
            run_id = %run_id,
            outcome = %outcome,
            message = "search finished"
        );
    }

    pub fn source_failure(source: &str, run_id: &str, category: &str, elapsed_ms: u64) {
        tracing::warn!(
            event_type = "SOURCE_FAILURE",
            component = "ingestion",
            source = %source,
            run_id = %run_id,
            outcome = "FAILURE",
            error_category = %category,
            elapsed_ms = elapsed_ms,
            message = "source request failed"
        );
    }

    pub fn dedup_complete(run_id: &str, count: u64) {
        tracing::info!(
            event_type = "DEDUP_COMPLETE",
            component = "ingestion",
            run_id = %run_id,
            outcome = "SUCCESS",
            dedup_count = count,
            message = "deduplication completed"
        );
    }

    pub fn llm_call(run_id: &str, provider: &str, model: &str, outcome: &str, elapsed_ms: u64) {
        tracing::info!(
            event_type = "LLM_CALL",
            component = "llm",
            run_id = %run_id,
            provider = %provider,
            model = %model,
            outcome = %outcome,
            elapsed_ms = elapsed_ms,
            message = "model call completed"
        );
    }

    pub fn llm_fallback(
        run_id: &str,
        provider: &str,
        model: &str,
        fallback_from: &str,
        category: &str,
        elapsed_ms: u64,
    ) {
        tracing::warn!(
            event_type = "LLM_FALLBACK",
            component = "llm",
            run_id = %run_id,
            provider = %provider,
            model = %model,
            fallback_from = %fallback_from,
            outcome = "PARTIAL",
            error_category = %category,
            elapsed_ms = elapsed_ms,
            message = "provider fallback"
        );
    }

    pub fn evaluation_complete(run_id: &str, outcome: &str, elapsed_ms: u64) {
        tracing::info!(
            event_type = "EVALUATION_COMPLETE",
            component = "match",
            run_id = %run_id,
            outcome = %outcome,
            elapsed_ms = elapsed_ms,
            message = "evaluation completed"
        );
    }

    pub fn application_action(job_ref: &str, transition: &str) {
        tracing::info!(
            event_type = "APPLICATION_ACTION",
            component = "ui",
            job_ref = %job_ref,
            lifecycle_transition = %transition,
            outcome = "SUCCESS",
            message = "application action"
        );
    }
}

pub struct TelemetryLayer {
    queue: BoundedQueue,
    health: HealthState,
}

impl TelemetryLayer {
    pub fn new(queue: BoundedQueue, health: HealthState) -> Self {
        Self { queue, health }
    }
}

impl<S: Subscriber> Layer<S> for TelemetryLayer {
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let level = Level::from_tracing(*event.metadata().level());
        let mut visitor = LogEventVisitor::new(level);
        event.record(&mut visitor);

        if let Some(log_event) = visitor.finish() {
            match serde_json::to_vec(&log_event) {
                Ok(line) => self.queue.push(line),
                Err(error) => self.health.mark_error(&error.to_string()),
            }
        }
    }
}

pub struct TelemetryRuntime {
    pub health: HealthState,
    pub guard: TelemetryGuard,
}

impl TelemetryRuntime {
    pub fn degraded(message: String) -> Self {
        let queue = BoundedQueue::new(1);
        let health = HealthState::new(queue.clone());
        health.mark_unavailable(&message);
        Self {
            health: health.clone(),
            guard: TelemetryGuard {
                queue,
                writer: None,
                health,
            },
        }
    }
}

pub struct TelemetryGuard {
    queue: BoundedQueue,
    writer: Option<JoinHandle<()>>,
    health: HealthState,
}

impl TelemetryGuard {
    pub fn shutdown(&mut self) {
        if self.writer.is_none() {
            return;
        }

        Telemetry::app_shutdown();
        self.queue.close();

        if let Some(handle) = self.writer.take() {
            writer::join_writer(handle, Duration::from_secs(5));
        }

        self.health.mark_available(false);
    }
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn init(config: TelemetryConfig) -> Result<TelemetryRuntime, AppError> {
    let queue = BoundedQueue::new(config.queue_capacity);
    let health = HealthState::new(queue.clone());
    let WriterHandle {
        thread,
        queue: _queue,
        health: _health,
    } = writer::spawn_writer(&config, queue.clone(), health.clone())?;

    let filter =
        EnvFilter::try_new(config.level.as_filter_str()).unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(TelemetryLayer::new(queue.clone(), health.clone()));

    if let Err(error) = tracing::subscriber::set_global_default(subscriber) {
        queue.close();
        writer::join_writer(thread, Duration::from_secs(5));
        return Err(AppError::Telemetry(format!(
            "could not install telemetry subscriber: {error}"
        )));
    }

    let session_id = format!(
        "pid-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    );
    Telemetry::app_start(env!("CARGO_PKG_VERSION"), &session_id);
    health.set_current_file(Some(format!(
        "jobs-search.{}.log",
        chrono::Utc::now().format("%Y-%m-%d")
    )));

    Ok(TelemetryRuntime {
        health: health.clone(),
        guard: TelemetryGuard {
            queue,
            writer: Some(thread),
            health,
        },
    })
}

pub fn log_health(health: &HealthState) -> LogHealth {
    health.snapshot()
}
