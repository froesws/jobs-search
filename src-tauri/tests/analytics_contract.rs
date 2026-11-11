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

#[test]
fn fifty_of_one_hundred_jobs_missing_kubernetes_is_50_percent() {
    let db = common::test_db();
    common::analytics::seed_analytics(&db, 100, 50);

    let payload = get_analytics(
        &db,
        AnalyticsRequest {
            scope: AnalyticsScope::Last7Days,
            run_id: None,
        },
    )
    .expect("analytics");

    assert_eq!(payload.denominator, 100);
    let kubernetes = payload
        .skill_gap
        .iter()
        .find(|metric| metric.display_name == "Kubernetes")
        .expect("kubernetes");
    assert_eq!(kubernetes.demand_count, 50);
    assert_eq!(kubernetes.demand_percentage, 50.0);
    assert_eq!(kubernetes.rank, 1);
}
