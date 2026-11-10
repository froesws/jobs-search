use jobs_search_lib::telemetry::event::{Component, EventType, Level, LogEvent, Outcome};
use jobs_search_lib::telemetry::health::LogHealth;
use jsonschema::validator_for;

fn schema(path: &str) -> serde_json::Value {
    serde_json::from_str(path).expect("valid schema JSON")
}

#[test]
fn log_event_matches_schema_and_rejects_unknown_fields() {
    let schema = schema(include_str!(
        "../../specs/004-telemetry-execution-logging/contracts/log-event.schema.json"
    ));
    let validator = validator_for(&schema).unwrap();

    let event = LogEvent::new(
        Level::Info,
        EventType::SourceFailure,
        Component::Ingestion,
        Outcome::Failure,
    )
    .with_run_id("run-1")
    .with_source("jobspy")
    .with_elapsed_ms(1_200)
    .with_message("source request failed");

    let value = serde_json::to_value(&event).unwrap();
    if !validator.is_valid(&value) {
        for error in validator.iter_errors(&value) {
            eprintln!("schema error: {error}");
        }
        eprintln!("event: {value}");
    }
    assert!(validator.is_valid(&value), "event should match schema");

    let invalid = serde_json::json!({
        "timestamp": "2026-11-10T20:00:00Z",
        "level": "INFO",
        "event_type": "APP_START",
        "component": "app",
        "outcome": "SUCCESS",
        "url": "https://example.com"
    });
    assert!(
        !validator.is_valid(&invalid),
        "unknown url must be rejected"
    );
}

#[test]
fn log_health_matches_schema() {
    let schema = schema(include_str!(
        "../../specs/004-telemetry-execution-logging/contracts/log-health.schema.json"
    ));
    let validator = validator_for(&schema).unwrap();

    let health = LogHealth {
        available: true,
        queue_depth: 0,
        queue_capacity: 10_000,
        dropped_count: 0,
        last_error: None,
        last_success_at: Some("2026-11-10T20:00:00Z".to_string()),
        current_file: Some("jobs-search.2026-11-10.log".to_string()),
    };

    let value = serde_json::to_value(health).unwrap();
    assert!(validator.is_valid(&value), "health should match schema");
}
