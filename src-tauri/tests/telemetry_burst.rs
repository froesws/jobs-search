use std::time::{Duration, Instant};

use jobs_search_lib::telemetry::event::Level;
use jobs_search_lib::telemetry::{Telemetry, TelemetryConfig};
use tempfile::tempdir;

#[test]
fn thousand_event_burst_is_non_blocking() {
    let directory = tempdir().unwrap();
    let config = TelemetryConfig::new(directory.path().to_path_buf(), Level::Info);
    let mut runtime = jobs_search_lib::telemetry::init(config).unwrap();

    let started = Instant::now();
    for index in 0..1_000 {
        Telemetry::search_run_start(&format!("run-{index}"));
    }
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "event dispatch blocked for {:?}",
        started.elapsed()
    );

    let health = runtime.health.snapshot();
    assert!(health.queue_depth <= health.queue_capacity);
    runtime.guard.shutdown();
}
