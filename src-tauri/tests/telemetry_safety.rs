use jobs_search_lib::telemetry::event::{Component, EventType, Level, LogEvent, Outcome};
use jobs_search_lib::telemetry::sanitize::{sanitize_identifier, sanitize_message};

#[test]
fn redacts_forbidden_message_content() {
    for input in [
        "see https://example.com/job",
        "file /home/will/resume.pdf",
        "prompt: tell me everything",
        "candidate email a@b.com",
        "api_key=abc",
    ] {
        assert_eq!(sanitize_message(input).as_deref(), Some("[redacted]"));
    }
}

#[test]
fn keeps_safe_messages_and_caps_length() {
    assert_eq!(
        sanitize_message("source request failed").as_deref(),
        Some("source request failed")
    );
    let capped = sanitize_message(&"a".repeat(600)).unwrap();
    assert_eq!(capped.chars().count(), 512);
}

#[test]
fn identifiers_reject_urls_whitespace_and_paths() {
    assert!(sanitize_identifier("run-1").is_some());
    assert!(sanitize_identifier("run 1").is_none());
    assert!(sanitize_identifier("https://example.com").is_none());
    assert!(sanitize_identifier("/home/will").is_none());
}

#[test]
fn event_message_is_redacted_at_the_boundary() {
    let event = LogEvent::new(
        Level::Info,
        EventType::ApplicationAction,
        Component::Ui,
        Outcome::Success,
    )
    .with_job_ref("job-1")
    .with_message("candidate email a@b.com");

    assert_eq!(event.message.as_deref(), Some("[redacted]"));
}
