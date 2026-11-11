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

#[test]
fn normalizes_variants_counts_once_and_ranks_ties_alphabetically() {
    let db = common::test_db();
    common::analytics::seed_analytics(&db, 4, 0);
    jobs_search_lib::db::profile::save_profile(&db, common::fixture_profile()).expect("profile");
    set_missing(
        &db,
        &[
            &["Kubernetes", "k8s", "gRPC"],
            &["kubernetes", "React", "gRPC"],
            &["K8S", "gRPC"],
            &["Kubernetes", "gRPC", "react.js"],
        ],
    );

    let payload = get_analytics(
        &db,
        AnalyticsRequest {
            scope: AnalyticsScope::Last7Days,
            run_id: None,
        },
    )
    .expect("analytics");

    assert_eq!(payload.denominator, 4);
    assert_eq!(payload.skill_gap.len(), 2, "{:?}", payload.skill_gap);
    assert_eq!(payload.skill_gap[0].display_name, "gRPC");
    assert_eq!(payload.skill_gap[0].demand_count, 4);
    assert_eq!(payload.skill_gap[0].demand_percentage, 100.0);
    assert_eq!(payload.skill_gap[1].display_name, "Kubernetes");
    assert_eq!(payload.skill_gap[1].demand_count, 4);
    assert!(
        !payload
            .skill_gap
            .iter()
            .any(|metric| metric.display_name == "React")
    );
}

#[test]
fn excludes_rejected_and_error_and_keeps_lifecycle_neutral() {
    let db = common::test_db();
    common::analytics::seed_analytics(&db, 3, 3);
    let ids = db
        .with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT job_id FROM job_evaluations ORDER BY job_id")?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            let mut ids = Vec::new();
            for row in rows {
                ids.push(row?);
            }
            Ok(ids)
        })
        .expect("ids");

    db.with_connection(|connection| {
        connection.execute(
            "UPDATE job_evaluations SET evaluation_status = 'REJECTED' WHERE job_id = ?1",
            [ids[1].as_str()],
        )?;
        connection.execute(
            "UPDATE job_evaluations SET evaluation_status = 'ERROR' WHERE job_id = ?1",
            [ids[2].as_str()],
        )?;
        Ok(())
    })
    .expect("status");

    jobs_search_lib::db::applications::apply_job(&db, &ids[0]).expect("apply");

    let payload = get_analytics(
        &db,
        AnalyticsRequest {
            scope: AnalyticsScope::Last7Days,
            run_id: None,
        },
    )
    .expect("analytics");
    assert_eq!(payload.denominator, 1);
    assert_eq!(payload.skill_gap[0].display_name, "Kubernetes");
    assert_eq!(payload.skill_gap[0].demand_count, 1);
    assert_eq!(payload.skill_gap[0].demand_percentage, 100.0);

    set_missing(&db, &[&[]]);
    let zero = get_analytics(
        &db,
        AnalyticsRequest {
            scope: AnalyticsScope::Last7Days,
            run_id: None,
        },
    )
    .expect("analytics");
    assert_eq!(zero.denominator, 1);
    assert!(zero.skill_gap.is_empty());
}

fn set_missing(db: &jobs_search_lib::db::Db, values: &[&[&str]]) {
    let ids = db
        .with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT job_id FROM job_evaluations ORDER BY job_id")?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            let mut ids = Vec::new();
            for row in rows {
                ids.push(row?);
            }
            Ok(ids)
        })
        .expect("ids");
    for (id, missing) in ids.iter().zip(values) {
        let json = serde_json::to_string(missing).expect("json");
        db.with_connection(|connection| {
            connection.execute(
                "UPDATE job_evaluations SET missing_keywords_json = ?1 WHERE job_id = ?2",
                [json.as_str(), id.as_str()],
            )?;
            Ok(())
        })
        .expect("update");
    }
}
