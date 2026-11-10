use jobs_search_lib::telemetry::health::HealthState;
use jobs_search_lib::telemetry::log_health;
use jobs_search_lib::telemetry::queue::BoundedQueue;

#[test]
fn health_payload_has_the_contract_fields() {
    let queue = BoundedQueue::new(10);
    queue.push(b"event".to_vec());
    let health = HealthState::new(queue);
    health.mark_error("disk full");

    let value = serde_json::to_value(log_health(&health)).unwrap();
    for field in [
        "available",
        "queue_depth",
        "queue_capacity",
        "dropped_count",
        "last_error",
        "last_success_at",
        "current_file",
    ] {
        assert!(value.get(field).is_some(), "missing field {field}");
    }
    assert_eq!(value["queue_capacity"], 10);
    assert_eq!(value["available"], false);
}
