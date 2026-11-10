use std::fs;

use chrono::{DateTime, Utc};
use jobs_search_lib::telemetry::writer::{cleanup_logs, parse_log_date};
use tempfile::tempdir;

#[test]
fn parses_only_daily_jobs_search_files() {
    assert!(parse_log_date("jobs-search.2026-11-10.log").is_some());
    assert!(parse_log_date("jobs-search.log.2026-11-10").is_none());
    assert!(parse_log_date("other.2026-11-10.log").is_none());
}

#[test]
fn removes_expired_and_oversized_files() {
    let directory = tempdir().unwrap();
    let expired = directory.path().join("jobs-search.2020-01-01.log");
    let older = directory.path().join("jobs-search.2026-11-08.log");
    let newest = directory.path().join("jobs-search.2026-11-09.log");

    fs::write(&expired, vec![b'a'; 8]).unwrap();
    fs::write(&older, vec![b'b'; 8]).unwrap();
    fs::write(&newest, vec![b'c'; 8]).unwrap();

    let now = DateTime::parse_from_rfc3339("2026-11-10T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    cleanup_logs(directory.path(), now, 7, 8).unwrap();

    assert!(!expired.exists());
    assert!(!older.exists());
    assert!(newest.exists());
}
