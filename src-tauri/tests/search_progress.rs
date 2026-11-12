use std::sync::{Arc, Mutex};
use std::time::Duration;

use jobs_search_lib::commands::search::{SearchProgress, SearchProgressTracker, SearchStage};

fn tracker(interval: Duration) -> (Arc<Mutex<Vec<SearchProgress>>>, SearchProgressTracker) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let tracker = SearchProgressTracker::with_interval(
        move |progress| {
            sink.lock().expect("events").push(progress);
        },
        interval,
    );
    (events, tracker)
}

fn snapshot(events: &Arc<Mutex<Vec<SearchProgress>>>) -> Vec<SearchProgress> {
    events.lock().expect("events").clone()
}

#[test]
fn progress_moves_forward_once_and_ends_with_a_final_event() {
    let (events, tracker) = tracker(Duration::ZERO);
    assert_eq!(
        snapshot(&events).first().map(|event| event.stage),
        Some(SearchStage::Collecting)
    );

    tracker.collecting(5, 0);
    tracker.collecting(12, 1);
    tracker.begin_evaluating(12);
    tracker.evaluated(3);
    tracker.evaluated(12);
    drop(tracker);

    let events = snapshot(&events);
    assert!(events.len() >= 6);
    assert_eq!(events[0].stage, SearchStage::Collecting);
    assert_eq!(events[0].found, 0);
    assert_eq!(events[0].failed, 0);
    assert_eq!(events[1].found, 5);
    assert_eq!(events[2].found, 12);
    assert_eq!(events[2].failed, 1);

    let transitions = events
        .windows(2)
        .filter(|pair| pair[0].stage != pair[1].stage)
        .count();
    assert_eq!(transitions, 1);
    assert_eq!(events[3].stage, SearchStage::Evaluating);
    assert_eq!(events[3].total, 12);

    let evaluated = events
        .iter()
        .map(|event| event.evaluated)
        .collect::<Vec<_>>();
    assert!(evaluated.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(events.iter().all(|event| event.evaluated <= event.total));

    let last = events.last().expect("final event");
    assert_eq!(last.stage, SearchStage::Evaluating);
    assert_eq!(last.evaluated, 12);
    assert_eq!(last.total, 12);
}

#[test]
fn an_empty_result_set_still_emits_a_final_evaluating_event() {
    let (events, tracker) = tracker(Duration::ZERO);
    tracker.begin_evaluating(0);
    drop(tracker);

    let last = snapshot(&events).last().cloned().expect("final event");
    assert_eq!(last.stage, SearchStage::Evaluating);
    assert_eq!(last.total, 0);
    assert_eq!(last.evaluated, 0);
}

#[test]
fn late_collection_updates_never_move_the_stage_backwards() {
    let (events, tracker) = tracker(Duration::ZERO);
    tracker.begin_evaluating(4);
    tracker.collecting(99, 3);
    tracker.evaluated(2);
    tracker.evaluated(9);
    drop(tracker);

    let events = snapshot(&events);
    assert!(
        events
            .iter()
            .skip(1)
            .all(|event| event.stage == SearchStage::Evaluating)
    );
    assert_eq!(events.last().expect("final").evaluated, 4);
}

#[test]
fn updates_are_coalesced_but_stage_transitions_are_forced() {
    let (events, tracker) = tracker(Duration::from_secs(1));
    tracker.collecting(4, 0);
    tracker.collecting(8, 1);
    tracker.begin_evaluating(8);
    tracker.evaluated(1);

    // Initial collecting event + forced evaluating transition.
    assert_eq!(snapshot(&events).len(), 2);
    drop(tracker);
    // The final event still reports the latest counters.
    let last = snapshot(&events).last().cloned().expect("final");
    assert_eq!(last.found, 8);
    assert_eq!(last.failed, 1);
    assert_eq!(last.evaluated, 1);
    assert_eq!(last.total, 8);
}
