use jobs_search_lib::ingestion::adapter::RawJob;
use jobs_search_lib::ingestion::normalize::{
    clean_description, detect_seniority, infer_modality, normalize,
};
use jobs_search_lib::types::{Seniority, WorkModality};
use zeroize::Zeroizing;

fn raw(description: &str) -> RawJob {
    RawJob {
        external_id: "1".to_string(),
        title: "Rust Engineer".to_string(),
        company: "Acme".to_string(),
        location: "Remote - Brazil".to_string(),
        work_modality: Some("Remote".to_string()),
        url: "https://example.com/1".to_string(),
        posted_at: Some("2026-11-08T12:00:00Z".to_string()),
        description: Zeroizing::new(description.to_string()),
        source_name: "test".to_string(),
        tags: Vec::new(),
    }
}

#[test]
fn cleaning_removes_markup_control_characters_and_decodes_entities() {
    let cleaned = clean_description(
        "<div><p>Rust&nbsp;&amp; C++</p><script>alert(1)</script>\u{0000}<br>Remote</div>",
    );
    assert!(cleaned.contains("Rust & C++"));
    assert!(!cleaned.contains("<p>"));
    assert!(!cleaned.contains('\u{0000}'));
    assert!(cleaned.contains('\n'));
}

#[test]
fn modality_and_seniority_are_controlled() {
    assert_eq!(
        infer_modality("Remoto no Brasil", None),
        WorkModality::Remote
    );
    assert_eq!(
        infer_modality("modelo híbrido em Porto Alegre", None),
        WorkModality::Hybrid
    );
    assert_eq!(
        infer_modality("presencial em Curitiba", Some("Unknown")),
        WorkModality::Onsite
    );
    assert_eq!(
        detect_seniority("Desenvolvedor Pleno/Sênior"),
        Some(Seniority::Pleno)
    );
    assert_eq!(
        detect_seniority("Senior Rust Engineer"),
        Some(Seniority::Senior)
    );
    assert_eq!(detect_seniority("Rust Engineer"), None);
}

#[test]
fn normalize_rejects_invalid_urls_and_unknown_posted_dates() {
    let mut job = raw("Rust");
    job.url = "not-a-url".to_string();
    assert!(normalize(job, "source", "Source").is_err());

    let mut job = raw("Rust");
    job.posted_at = Some("not-a-date".to_string());
    let normalized = normalize(job, "source", "Source").expect("normalized");
    assert_eq!(normalized.posted_at, None);
}

#[test]
fn normalize_keeps_only_clean_text_and_sets_identity() {
    let normalized =
        normalize(raw("<p>Rust &amp; C++</p>"), "source", "Source").expect("normalized");
    assert_eq!(normalized.title, "Rust Engineer");
    assert_eq!(normalized.description_clean, "Rust & C++");
    assert!(!normalized.description_clean.contains('<'));
    assert!(!normalized.canonical_hash.is_empty());
    assert_eq!(normalized.work_modality, WorkModality::Remote);
}
