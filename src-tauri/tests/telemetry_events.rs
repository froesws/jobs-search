use std::fs;

use jobs_search_lib::telemetry::event::Level;
use jobs_search_lib::telemetry::{Telemetry, TelemetryConfig};
use tempfile::tempdir;

#[test]
fn full_event_sequence_is_written_and_readable() {
    let directory = tempdir().unwrap();
    let config = TelemetryConfig::new(directory.path().to_path_buf(), Level::Info);
    let mut runtime = jobs_search_lib::telemetry::init(config).unwrap();

    Telemetry::search_run_start("run-e2e");
    Telemetry::source_failure("jobspy", "run-e2e", "TIMEOUT", 1_200);
    Telemetry::dedup_complete("run-e2e", 3);
    Telemetry::llm_call("run-e2e", "gemini", "gemini-flash", "SUCCESS", 800);
    Telemetry::llm_fallback(
        "run-e2e",
        "openrouter",
        "free-model",
        "gemini",
        "RATE_LIMIT",
        1_200,
    );
    Telemetry::evaluation_complete("run-e2e", "SUCCESS", 1_500);
    Telemetry::application_action("job-1", "APPLIED");

    runtime.guard.shutdown();

    let mut contents = String::new();
    for entry in fs::read_dir(directory.path()).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            contents.push_str(&fs::read_to_string(path).unwrap_or_default());
        }
    }

    let events: Vec<serde_json::Value> = contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("valid JSON line"))
        .collect();

    let types: Vec<&str> = events
        .iter()
        .map(|event| event["event_type"].as_str().unwrap())
        .collect();

    for expected in [
        "APP_START",
        "SEARCH_RUN_START",
        "SOURCE_FAILURE",
        "DEDUP_COMPLETE",
        "LLM_CALL",
        "LLM_FALLBACK",
        "EVALUATION_COMPLETE",
        "APPLICATION_ACTION",
        "APP_SHUTDOWN",
    ] {
        assert!(types.contains(&expected), "missing {expected}");
    }
}
