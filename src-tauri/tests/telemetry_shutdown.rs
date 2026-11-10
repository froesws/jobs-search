use std::fs;
use std::time::{Duration, Instant};

use jobs_search_lib::telemetry::event::Level;
use jobs_search_lib::telemetry::{Telemetry, TelemetryConfig};
use tempfile::tempdir;

#[test]
fn shutdown_flushes_within_five_seconds() {
    let directory = tempdir().unwrap();
    let config = TelemetryConfig::new(directory.path().to_path_buf(), Level::Info);
    let mut runtime = jobs_search_lib::telemetry::init(config).unwrap();

    Telemetry::search_run_start("run-shutdown");
    let started = Instant::now();
    runtime.guard.shutdown();
    assert!(started.elapsed() < Duration::from_secs(5));

    let mut found = false;
    for entry in fs::read_dir(directory.path()).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            let text = fs::read_to_string(path).unwrap_or_default();
            if text.contains("run-shutdown") {
                found = true;
            }
        }
    }
    assert!(found, "pending event was not flushed");
}
