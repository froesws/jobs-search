mod common;

use common::{events, test_sink, with_telemetry};
use jobs_search_lib::telemetry::Telemetry;

#[test]
fn lifecycle_events_have_required_fields() {
    let sink = test_sink();
    with_telemetry("info", &sink, || {
        Telemetry::app_start("0.1.0", "session-1");
        Telemetry::search_run_start("run-1");
        Telemetry::search_run_end("run-1", "SUCCESS");
        Telemetry::app_shutdown();
    });

    let events = events(&sink);
    assert_eq!(events.len(), 4);

    assert_eq!(events[0]["event_type"], "APP_START");
    assert_eq!(events[0]["level"], "INFO");
    assert_eq!(events[0]["component"], "app");
    assert_eq!(events[0]["outcome"], "SUCCESS");

    assert_eq!(events[1]["event_type"], "SEARCH_RUN_START");
    assert_eq!(events[1]["run_id"], "run-1");
    assert_eq!(events[2]["event_type"], "SEARCH_RUN_END");
    assert_eq!(events[2]["outcome"], "SUCCESS");
    assert_eq!(events[3]["event_type"], "APP_SHUTDOWN");
}
