mod common;

use common::{events, test_sink, with_telemetry};
use jobs_search_lib::telemetry::Telemetry;

#[test]
fn application_actions_record_job_reference_and_transition() {
    let sink = test_sink();
    with_telemetry("info", &sink, || {
        Telemetry::application_action("job-1", "APPLIED");
    });

    let events = events(&sink);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event_type"], "APPLICATION_ACTION");
    assert_eq!(events[0]["job_ref"], "job-1");
    assert_eq!(events[0]["lifecycle_transition"], "APPLIED");
    assert_eq!(events[0]["component"], "ui");
}
