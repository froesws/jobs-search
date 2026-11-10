use jobs_search_lib::telemetry::queue::BoundedQueue;

#[test]
fn queue_drops_oldest_and_counts_drops() {
    let queue = BoundedQueue::new(10_000);
    for index in 0..=10_000 {
        queue.push(format!("line-{index}").into_bytes());
    }

    assert_eq!(queue.depth(), 10_000);
    assert_eq!(queue.dropped_count(), 1);

    let first = queue.try_pop_batch(1);
    assert_eq!(
        String::from_utf8(first[0].clone()).unwrap(),
        "line-1",
        "the oldest event should be dropped"
    );
}
