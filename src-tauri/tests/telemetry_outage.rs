use std::fs;

use jobs_search_lib::telemetry::event::Level;
use jobs_search_lib::telemetry::health::HealthState;
use jobs_search_lib::telemetry::queue::BoundedQueue;
use jobs_search_lib::telemetry::writer::spawn_writer;
use jobs_search_lib::telemetry::{TelemetryConfig, TelemetryRuntime};
use tempfile::tempdir;

#[test]
fn degraded_runtime_reports_unavailable() {
    let runtime = TelemetryRuntime::degraded("disk full".to_string());
    let health = runtime.health.snapshot();
    assert!(!health.available);
    assert!(
        health
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("disk full")
    );
}

#[test]
fn writer_rejects_a_file_as_log_directory() {
    let directory = tempdir().unwrap();
    let file_path = directory.path().join("not-a-directory");
    fs::write(&file_path, b"x").unwrap();

    let queue = BoundedQueue::new(2);
    let health = HealthState::new(queue.clone());
    let config = TelemetryConfig::new(file_path, Level::Info);

    assert!(spawn_writer(&config, queue, health).is_err());
}
