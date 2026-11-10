#![allow(dead_code)]

use jobs_search_lib::telemetry::TelemetryLayer;
use jobs_search_lib::telemetry::health::HealthState;
use jobs_search_lib::telemetry::queue::BoundedQueue;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;

pub struct TestSink {
    pub queue: BoundedQueue,
    pub health: HealthState,
}

pub fn test_sink() -> TestSink {
    let queue = BoundedQueue::new(1_000);
    TestSink {
        health: HealthState::new(queue.clone()),
        queue,
    }
}

pub fn with_telemetry<F: FnOnce()>(level: &str, sink: &TestSink, action: F) {
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new(level))
        .with(TelemetryLayer::new(sink.queue.clone(), sink.health.clone()));
    let dispatch = tracing::Dispatch::new(subscriber);
    tracing::dispatcher::with_default(&dispatch, action);
}

pub fn events(sink: &TestSink) -> Vec<serde_json::Value> {
    sink.queue
        .try_pop_batch(10_000)
        .into_iter()
        .map(|line| serde_json::from_slice(&line).expect("valid JSON event"))
        .collect()
}
