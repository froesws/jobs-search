use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use jobs_search_lib::commands::profile::{
    ProfileExtractor, delete_resume_with, load_profile_for_search,
    save_profile as save_profile_command, upload_resume_with,
};
use jobs_search_lib::config::AppConfig;
use jobs_search_lib::db::Db;
use jobs_search_lib::db::profile::{
    delete_resume, insert_resume, list_resumes, load_active_profile, resume_count, save_profile,
};
use jobs_search_lib::error::AppError;
use jobs_search_lib::pdf::{extract_text_from_mem, extract_text_from_path};
use jobs_search_lib::profile::merge::merge_extracted;
use jobs_search_lib::profile::schema::{ExtractedProfile, WorkHistoryEntry};
use jobs_search_lib::state::AppState;
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

#[test]
fn profile_and_resumes_survive_reopen() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("profile.sqlite3");
    let db = Db::open(&path).expect("open");
    let profile =
        merge_extracted(None, &extracted("Rust Engineer", &["Rust"]), "resume-1").expect("profile");
    let saved = save_profile(&db, profile).expect("save");
    insert_resume(&db, "resume.pdf", "sha-1", "/tmp/resume.pdf", None).expect("resume");
    drop(db);

    let reopened = Db::open(&path).expect("reopen");
    let loaded = load_active_profile(&reopened)
        .expect("load")
        .expect("profile");
    assert_eq!(loaded.profile_id, saved.profile_id);
    assert_eq!(loaded.version, 1);
    assert_eq!(list_resumes(&reopened).expect("resumes").len(), 1);
}

#[test]
fn profile_version_is_monotonic_and_delete_removes_resume() {
    let db = Db::open_in_memory().expect("db");
    let profile =
        merge_extracted(None, &extracted("Rust Engineer", &["Rust"]), "resume-1").expect("profile");
    let first = save_profile(&db, profile).expect("first");
    let second = save_profile(&db, first.clone()).expect("second");
    assert_eq!(second.version, first.version + 1);

    let resume =
        insert_resume(&db, "resume.pdf", "sha-1", "/tmp/resume.pdf", None).expect("resume");
    let deleted = delete_resume(&db, &resume.resume.id).expect("delete");
    assert!(deleted.is_some());
    assert_eq!(resume_count(&db).expect("count"), 0);
}

#[test]
fn invalid_profile_does_not_overwrite_the_last_valid_one() {
    let db = Db::open_in_memory().expect("db");
    let profile =
        merge_extracted(None, &extracted("Rust Engineer", &["Rust"]), "resume-1").expect("profile");
    let saved = save_profile(&db, profile.clone()).expect("save");
    let mut invalid = profile;
    invalid.target_roles.clear();

    assert!(save_profile(&db, invalid).is_err());
    let loaded = load_active_profile(&db).expect("load").expect("profile");
    assert_eq!(loaded, saved);
}

struct FakeExtractor {
    fail: bool,
    calls: Arc<AtomicUsize>,
}

impl FakeExtractor {
    fn new(fail: bool) -> Self {
        Self {
            fail,
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ProfileExtractor for FakeExtractor {
    async fn extract(&self, _resume_text: &str) -> Result<ExtractedProfile, AppError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(AppError::Provider("provider failed".into()));
        }
        Ok(extracted("Rust Engineer", &["Rust", "Linux"]))
    }
}

fn app_state(directory: &std::path::Path) -> AppState {
    let config =
        AppConfig::from_lookup(|_| None, directory.join("app-data"), directory).expect("config");
    AppState::new(config).expect("state")
}

fn write_resume(directory: &std::path::Path) -> String {
    let path = directory.join("resume.pdf");
    std::fs::write(&path, text_pdf("Rust and Linux kernel work")).expect("pdf");
    path.display().to_string()
}

#[tokio::test]
async fn upload_resume_persists_profile_and_file() {
    let directory = tempfile::tempdir().expect("tempdir");
    let state = app_state(directory.path());
    let path = write_resume(directory.path());

    let extractor = FakeExtractor::new(false);
    let outcome = upload_resume_with(&state, &path, &extractor)
        .await
        .expect("upload");

    assert!(!outcome.duplicate);
    assert!(outcome.warning.is_none());
    assert_eq!(extractor.calls(), 1);
    let profile = outcome.profile.expect("profile");
    assert!(profile.core_competencies.contains(&"Rust".to_string()));
    assert!(profile.has_resumes());
    assert_eq!(resume_count(&state.db).expect("count"), 1);
    assert!(load_profile_for_search(&state).is_ok());
    let resumes = list_resumes(&state.db).expect("resumes");
    assert_eq!(resumes.len(), 1);
    assert!(resumes[0].extracted_json.is_some());
    assert!(std::path::Path::new(&resumes[0].local_path).exists());
}

#[tokio::test]
async fn duplicate_resume_is_not_stored_twice() {
    let directory = tempfile::tempdir().expect("tempdir");
    let state = app_state(directory.path());
    let path = write_resume(directory.path());

    let extractor = FakeExtractor::new(false);
    let first = upload_resume_with(&state, &path, &extractor)
        .await
        .expect("first");
    let second = upload_resume_with(&state, &path, &extractor)
        .await
        .expect("second");

    assert!(!first.duplicate);
    assert!(second.duplicate);
    assert!(second.warning.is_none());
    assert_eq!(extractor.calls(), 1);
    assert_eq!(resume_count(&state.db).expect("count"), 1);
    let first_profile = first.profile.expect("first profile");
    let second_profile = second.profile.expect("second profile");
    assert_eq!(first_profile.version, second_profile.version);
}

#[tokio::test]
async fn duplicate_failed_resume_is_reported_without_reprocessing() {
    let directory = tempfile::tempdir().expect("tempdir");
    let state = app_state(directory.path());
    let path = write_resume(directory.path());

    let failing = FakeExtractor::new(true);
    let first = upload_resume_with(&state, &path, &failing)
        .await
        .expect("partial success");
    assert!(first.warning.is_some());
    assert!(first.profile.is_none());
    assert_eq!(failing.calls(), 1);

    let succeeding = FakeExtractor::new(false);
    let second = upload_resume_with(&state, &path, &succeeding)
        .await
        .expect("duplicate");
    assert!(second.duplicate);
    assert!(second.warning.is_some());
    assert!(second.profile.is_none());
    assert_eq!(succeeding.calls(), 0);
    assert_eq!(resume_count(&state.db).expect("count"), 1);
}

#[tokio::test]
async fn failed_extraction_keeps_the_resume_and_returns_a_warning() {
    let directory = tempfile::tempdir().expect("tempdir");
    let state = app_state(directory.path());
    let path = write_resume(directory.path());

    let extractor = FakeExtractor::new(true);
    let outcome = upload_resume_with(&state, &path, &extractor)
        .await
        .expect("partial success");
    assert!(outcome.warning.is_some());
    assert!(!outcome.duplicate);
    assert!(outcome.profile.is_none());
    assert_eq!(extractor.calls(), 1);

    let resumes = list_resumes(&state.db).expect("resumes");
    assert_eq!(resumes.len(), 1);
    assert!(resumes[0].extracted_json.is_none());
    assert!(std::path::Path::new(&resumes[0].local_path).exists());
    assert!(load_active_profile(&state.db).expect("load").is_none());
}

#[tokio::test]
async fn deleting_the_last_resume_blocks_new_searches() {
    let directory = tempfile::tempdir().expect("tempdir");
    let state = app_state(directory.path());
    let path = write_resume(directory.path());
    upload_resume_with(&state, &path, &FakeExtractor::new(false))
        .await
        .expect("upload");
    let resume = list_resumes(&state.db).expect("resumes").remove(0);

    let profile = delete_resume_with(&state, resume.id.as_str())
        .expect("delete")
        .expect("profile");
    assert!(!profile.has_resumes());
    assert_eq!(resume_count(&state.db).expect("count"), 0);
    assert!(!std::path::Path::new(&resume.local_path).exists());
    let error = load_profile_for_search(&state).expect_err("blocked");
    assert_eq!(error.code(), "RESUME_REQUIRED");
}

#[tokio::test]
async fn deleting_a_failed_resume_without_a_profile_succeeds() {
    let directory = tempfile::tempdir().expect("tempdir");
    let state = app_state(directory.path());
    let path = write_resume(directory.path());

    upload_resume_with(&state, &path, &FakeExtractor::new(true))
        .await
        .expect("partial success");
    let resume = list_resumes(&state.db).expect("resumes").remove(0);

    let profile = delete_resume_with(&state, resume.id.as_str()).expect("delete");
    assert!(profile.is_none());
    assert_eq!(resume_count(&state.db).expect("count"), 0);
    assert!(!std::path::Path::new(&resume.local_path).exists());
}

#[tokio::test]
async fn saving_the_profile_advances_version_and_clears_no_resume_state() {
    let directory = tempfile::tempdir().expect("tempdir");
    let state = app_state(directory.path());
    let path = write_resume(directory.path());
    let uploaded = upload_resume_with(&state, &path, &FakeExtractor::new(false))
        .await
        .expect("upload");
    let uploaded_profile = uploaded.profile.expect("profile");

    let mut edited = uploaded_profile.clone();
    edited.extras.resumes_available = false;
    edited.acceptable_technologies = vec!["React".into(), "react".into()];
    let saved = save_profile_command(&state, edited).expect("save");

    assert_eq!(saved.version, uploaded_profile.version + 1);
    assert!(saved.has_resumes());
    assert_eq!(saved.acceptable_technologies, ["React"]);
}

struct RoleExtractor;

#[async_trait]
impl ProfileExtractor for RoleExtractor {
    async fn extract(&self, resume_text: &str) -> Result<ExtractedProfile, AppError> {
        let title = if resume_text.contains("C#") {
            "C# Engineer"
        } else {
            "Rust Engineer"
        };
        let mut profile = extracted(title, &["Rust"]);
        profile.work_history = vec![WorkHistoryEntry {
            title: title.to_string(),
            company: "Acme".to_string(),
            start: "2024".to_string(),
            end: None,
            highlights: vec![],
        }];
        Ok(profile)
    }
}

struct TextExtractor;

#[async_trait]
impl ProfileExtractor for TextExtractor {
    async fn extract(&self, resume_text: &str) -> Result<ExtractedProfile, AppError> {
        let core = if resume_text.contains("C#") {
            vec!["C#"]
        } else {
            vec!["Rust"]
        };
        Ok(extracted("Engineer", &core))
    }
}

#[tokio::test]
async fn multiple_roles_from_two_resumes_are_kept_without_conflicts() {
    let directory = tempfile::tempdir().expect("tempdir");
    let state = app_state(directory.path());
    let first_path = directory.path().join("rust.pdf");
    let second_path = directory.path().join("csharp.pdf");
    std::fs::write(&first_path, text_pdf("Rust systems work")).expect("first");
    std::fs::write(&second_path, text_pdf("C# migration tools")).expect("second");

    upload_resume_with(&state, &first_path.display().to_string(), &RoleExtractor)
        .await
        .expect("first upload");
    upload_resume_with(&state, &second_path.display().to_string(), &RoleExtractor)
        .await
        .expect("second upload");

    let profile = load_active_profile(&state.db)
        .expect("load")
        .expect("profile");
    assert!(profile.extras.conflicts.is_empty());
    assert_eq!(
        profile
            .extras
            .work_history
            .iter()
            .map(|entry| entry.title.as_str())
            .collect::<Vec<_>>(),
        ["C# Engineer", "Rust Engineer"]
    );
}

#[tokio::test]
async fn deleting_one_resume_removes_only_its_contributions() {
    let directory = tempfile::tempdir().expect("tempdir");
    let state = app_state(directory.path());
    let first_path = directory.path().join("rust.pdf");
    let second_path = directory.path().join("csharp.pdf");
    std::fs::write(&first_path, text_pdf("Rust systems work")).expect("first");
    std::fs::write(&second_path, text_pdf("C# migration tools")).expect("second");

    upload_resume_with(&state, &first_path.display().to_string(), &TextExtractor)
        .await
        .expect("first upload");
    let merged = upload_resume_with(&state, &second_path.display().to_string(), &TextExtractor)
        .await
        .expect("second upload");
    let profile = merged.profile.expect("profile");
    assert!(profile.core_competencies.contains(&"Rust".to_string()));
    assert!(profile.core_competencies.contains(&"C#".to_string()));

    let first_resume = list_resumes(&state.db).expect("resumes").remove(0);
    let recalculated = delete_resume_with(&state, first_resume.id.as_str())
        .expect("delete")
        .expect("profile");

    assert_eq!(recalculated.core_competencies, ["C#"]);
    assert_eq!(recalculated.source_documents.len(), 1);
    assert_eq!(resume_count(&state.db).expect("count"), 1);
}

#[tokio::test]
async fn provider_404_still_persists_the_resume() {
    use std::time::Duration;

    use jobs_search_lib::commands::profile::LlmProfileExtractor;
    use jobs_search_lib::llm::gemini::GeminiProvider;
    use jobs_search_lib::llm::provider::{LlmRouter, ProviderPlan};
    use wiremock::matchers::{method, path as wire_path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let directory = tempfile::tempdir().expect("tempdir");
    let state = app_state(directory.path());
    let path = write_resume(directory.path());

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wire_path("/v1beta/models/gemini-test:generateContent"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");
    let router = LlmRouter::from_plans(vec![ProviderPlan {
        provider: Arc::new(GeminiProvider::new(
            client,
            server.uri(),
            "gemini-test",
            "invalid-key",
            Duration::from_secs(5),
        )),
        attempts: 1,
    }]);
    let extractor = LlmProfileExtractor::new(router);

    let outcome = upload_resume_with(&state, &path, &extractor)
        .await
        .expect("partial success");
    assert!(outcome.warning.is_some());
    assert!(outcome.profile.is_none());
    assert_eq!(resume_count(&state.db).expect("count"), 1);
    let resumes = list_resumes(&state.db).expect("resumes");
    assert!(resumes[0].extracted_json.is_none());
    assert!(std::path::Path::new(&resumes[0].local_path).exists());
}
