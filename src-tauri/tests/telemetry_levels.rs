mod common;

use std::path::PathBuf;

use common::{events, test_sink, with_telemetry};
use jobs_search_lib::telemetry::event::Level;
use jobs_search_lib::telemetry::{Telemetry, TelemetryConfig};

#[test]
fn info_filters_debug_events() {
    let sink = test_sink();
    with_telemetry("info", &sink, || {
        Telemetry::search_run_start("run-1");
        tracing::debug!(
            event_type = "APP_START",
            component = "app",
            outcome = "SUCCESS",
            message = "debug event"
        );
    });

    let events = events(&sink);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["level"], "INFO");
}

#[test]
fn debug_allows_debug_events() {
    let sink = test_sink();
    with_telemetry("debug", &sink, || {
        Telemetry::search_run_start("run-1");
        tracing::debug!(
            event_type = "APP_START",
            component = "app",
            outcome = "SUCCESS",
            message = "debug event"
        );
    });

    let events = events(&sink);
    assert_eq!(events.len(), 2);
    assert_eq!(events[1]["level"], "DEBUG");
}

#[test]
fn invalid_rust_log_falls_back_to_info() {
    let (config, warning) = TelemetryConfig::from_env_value("verbose", PathBuf::from("/tmp/logs"));
    assert_eq!(config.level, Level::Info);
    assert!(warning.is_some());
}
