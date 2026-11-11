use jobs_search_lib::pdf::{extract_text_from_mem, extract_text_from_path};
use jobs_search_lib::profile::merge::merge_extracted;
use jobs_search_lib::profile::schema::{ExtractedProfile, WorkHistoryEntry};
use lopdf::{Document, Object, Stream, dictionary};

fn text_pdf(text: &str) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });
    let escaped = text
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)");
    let content = format!("BT /F1 14 Tf 72 720 Td ({escaped}) Tj ET");
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(612),
            Object::Integer(792),
        ],
        "Contents" => content_id,
        "Resources" => resources_id,
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.compress();

    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).expect("save pdf");
    bytes
}

fn image_only_pdf() -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let content_id = doc.add_object(Stream::new(dictionary! {}, b"q Q".to_vec()));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(612),
            Object::Integer(792),
        ],
        "Contents" => content_id,
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.compress();

    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).expect("save pdf");
    bytes
}

#[test]
fn extracts_text_from_a_text_pdf() {
    let text = extract_text_from_mem(&text_pdf("Rust and Linux kernel work")).expect("extract");
    assert!(text.contains("Rust"));
    assert!(text.contains("Linux"));
}

#[test]
fn rejects_corrupt_and_non_pdf_files() {
    assert!(extract_text_from_mem(b"not a pdf").is_err());
    assert!(extract_text_from_mem(b"%PDF-1.4\ncorrupt").is_err());
}

#[test]
fn rejects_image_only_pdfs_without_partial_text() {
    let error = extract_text_from_mem(&image_only_pdf()).expect_err("image-only");
    assert!(error.message().contains("no extractable text"));
}

#[test]
fn extracts_from_a_validated_file_path() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("resume.pdf");
    std::fs::write(&path, text_pdf("C# migration tools")).expect("write");
    let text = extract_text_from_path(&path).expect("extract");
    assert!(text.contains("C# migration tools"));
}

#[test]
fn rejects_oversized_files_before_parsing() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("huge.pdf");
    std::fs::write(&path, vec![b'%'; (20 * 1024 * 1024 + 1) as usize]).expect("write");
    let error = extract_text_from_path(&path).expect_err("oversized");
    assert!(error.message().contains("larger than 20 MB"));
}

fn extracted(title: &str, core: &[&str]) -> ExtractedProfile {
    ExtractedProfile {
        core_competencies: core.iter().map(|value| value.to_string()).collect(),
        secondary_competencies: vec!["EF SET C1".into()],
        inferred_competencies: vec!["Embedded C".into()],
        domain_experience: vec!["systems programming".into()],
        work_history: vec![WorkHistoryEntry {
            title: title.into(),
            company: "Acme".into(),
            start: "2023".into(),
            end: None,
            highlights: vec![],
        }],
        education: vec![],
        certifications: vec!["EF SET C1".into()],
        source_evidence: vec![],
    }
}

#[test]
fn merge_unions_lists_without_duplicates() {
    let first = merge_extracted(
        None,
        &extracted("Rust Engineer", &["Rust", "Linux"]),
        "resume-1",
    )
    .expect("first");
    let second = merge_extracted(
        Some(&first),
        &extracted("Rust Engineer", &["Rust", "C#"]),
        "resume-2",
    )
    .expect("second");

    assert_eq!(second.core_competencies, ["Rust", "Linux", "C#"]);
    assert_eq!(second.source_documents, ["resume-1", "resume-2"]);
    assert_eq!(
        second.extras.competency_sources.get("Rust").expect("rust"),
        &["resume-1".to_string(), "resume-2".to_string()]
    );
}

#[test]
fn merge_flags_scalar_conflicts_instead_of_overwriting() {
    let first =
        merge_extracted(None, &extracted("Rust Engineer", &["Rust"]), "resume-1").expect("first");
    let second = merge_extracted(
        Some(&first),
        &extracted("Staff Engineer", &["Rust"]),
        "resume-2",
    )
    .expect("second");

    assert_eq!(second.extras.conflicts.len(), 1);
    assert_eq!(second.extras.conflicts[0].field, "current_title");
    assert_eq!(
        second.extras.conflicts[0].values,
        ["Rust Engineer", "Staff Engineer"]
    );
}

#[test]
fn merge_preserves_manual_profile_precedence() {
    let mut existing =
        merge_extracted(None, &extracted("Rust Engineer", &["Rust"]), "resume-1").expect("first");
    existing.acceptable_technologies = vec!["React".into()];
    existing.accepted_cities = vec!["Curitiba".into()];
    existing.dealbreakers = vec!["sales".into()];

    let merged = merge_extracted(
        Some(&existing),
        &extracted("Rust Engineer", &["C#"]),
        "resume-2",
    )
    .expect("second");

    assert!(merged.technologies().contains(&"React".to_string()));
    assert!(merged.technologies().contains(&"C#".to_string()));
    assert_eq!(merged.accepted_cities, ["Curitiba"]);
    assert_eq!(merged.dealbreakers, ["sales"]);
    assert_eq!(merged.salary_threshold, None);
}
