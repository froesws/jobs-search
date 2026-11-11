use jobs_search_lib::dedup::canonical_identity;

#[test]
fn same_job_across_sources_has_one_identity() {
    let first = canonical_identity(
        "Acme, Inc.",
        "Senior Rust Engineer",
        "São Paulo",
        "https://linkedin.example/1",
    );
    let second = canonical_identity(
        "acme inc",
        "senior rust engineer",
        "sao paulo",
        "https://indeed.example/2",
    );
    assert_eq!(first.hash, second.hash);
    assert!(!first.used_url_fallback);
}

#[test]
fn missing_location_falls_back_to_normalized_url() {
    let identity = canonical_identity(
        "Acme",
        "Rust Engineer",
        "Unknown",
        "https://example.com/job/1?utm_source=newsletter",
    );
    assert!(identity.used_url_fallback);
    assert!(identity.key.contains("https://example.com/job/1"));
    assert!(!identity.key.contains("utm_source"));
}

#[test]
fn title_and_company_collisions_are_not_merged_by_url_only() {
    let first = canonical_identity("Acme", "Rust Engineer", "Remote", "https://a/1");
    let second = canonical_identity("Acme", "C++ Engineer", "Remote", "https://a/1");
    assert_ne!(first.hash, second.hash);
}

#[test]
fn missing_company_falls_back_to_url() {
    let identity = canonical_identity("", "Rust Engineer", "Remote", "https://a/1");
    assert!(identity.used_url_fallback);
}
