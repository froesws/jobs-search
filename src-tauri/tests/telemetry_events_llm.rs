mod common;

use common::{events, test_sink, with_telemetry};
use jobs_search_lib::telemetry::Telemetry;

#[test]
fn llm_events_record_provider_model_latency_and_fallback() {
    let sink = test_sink();
    with_telemetry("info", &sink, || {
        Telemetry::llm_call("run-1", "gemini", "gemini-flash", "SUCCESS", 800);
        Telemetry::llm_fallback(
            "run-1",
            "openrouter",
            "free-model",
            "gemini",
            "RATE_LIMIT",
            1_200,
        );
        Telemetry::evaluation_complete("run-1", "SUCCESS", 1_500);
    });

    let events = events(&sink);
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["event_type"], "LLM_CALL");
    assert_eq!(events[0]["provider"], "gemini");
    assert_eq!(events[0]["model"], "gemini-flash");
    assert_eq!(events[0]["elapsed_ms"], 800);

    assert_eq!(events[1]["event_type"], "LLM_FALLBACK");
    assert_eq!(events[1]["provider"], "openrouter");
    assert_eq!(events[1]["fallback_from"], "gemini");
    assert_eq!(events[1]["error_category"], "RATE_LIMIT");
    assert_eq!(events[1]["outcome"], "PARTIAL");

    assert_eq!(events[2]["event_type"], "EVALUATION_COMPLETE");
    assert_eq!(events[2]["component"], "match");
}
