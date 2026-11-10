mod common;

use common::{events, test_sink, with_telemetry};
use jobs_search_lib::telemetry::Telemetry;

#[test]
fn ingestion_events_record_source_and_dedup() {
    let sink = test_sink();
    with_telemetry("info", &sink, || {
        Telemetry::source_failure("jobspy", "run-1", "TIMEOUT", 1_200);
        Telemetry::dedup_complete("run-1", 3);
    });

    let events = events(&sink);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["event_type"], "SOURCE_FAILURE");
    assert_eq!(events[0]["source"], "jobspy");
    assert_eq!(events[0]["error_category"], "TIMEOUT");
    assert_eq!(events[0]["elapsed_ms"], 1_200);
    assert_eq!(events[1]["event_type"], "DEDUP_COMPLETE");
    assert_eq!(events[1]["dedup_count"], 3);
}
