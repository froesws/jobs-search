use jobs_search_lib::analytics::{AnalyticsRequest, AnalyticsScope, get_analytics};

mod common;

#[test]
fn empty_database_returns_a_valid_empty_payload() {
    let db = common::test_db();
    let payload = get_analytics(
        &db,
        AnalyticsRequest {
            scope: AnalyticsScope::Last7Days,
            run_id: None,
        },
    )
    .expect("analytics");

    assert_eq!(payload.denominator, 0);
    assert!(payload.skill_gap.is_empty());
    assert!(!payload.last_updated.is_empty());
    assert!(!payload.stale);
}
