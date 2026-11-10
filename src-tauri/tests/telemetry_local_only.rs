use jobs_search_lib::telemetry::event::{Component, EventType, Level, LogEvent, Outcome};

#[test]
fn serialized_event_has_no_external_or_sensitive_fields() {
    let event = LogEvent::new(
        Level::Info,
        EventType::SourceFailure,
        Component::Ingestion,
        Outcome::Failure,
    )
    .with_run_id("run-1")
    .with_source("jobspy")
    .with_message("source request failed");

    let json = serde_json::to_string(&event).unwrap();
    for forbidden in [
        "mongodb",
        "http://",
        "https://",
        "prompt",
        "response",
        "description",
        "candidate",
        "api_key",
        "/home/",
    ] {
        assert!(
            !json.to_ascii_lowercase().contains(forbidden),
            "event contained {forbidden}: {json}"
        );
    }
}
