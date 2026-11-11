use std::path::Path;

use async_trait::async_trait;
use serde::Serialize;
use tauri::State;

use crate::db;
use crate::error::{AppError, ErrorPayload};
use crate::llm::provider::LlmRouter;
use crate::pdf;
use crate::profile::merge::{ProfileExtras, merge_extracted};
use crate::profile::schema::ExtractedProfile;
use crate::profile::{CandidateProfile, fixture};
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct UploadOutcome {
    pub profile: CandidateProfile,
    pub duplicate: bool,
}

#[async_trait]
pub trait ProfileExtractor: Send + Sync {
    async fn extract(&self, resume_text: &str) -> Result<ExtractedProfile, AppError>;
}

pub struct LlmProfileExtractor {
    router: LlmRouter,
}

impl LlmProfileExtractor {
    pub fn new(router: LlmRouter) -> Self {
        Self { router }
    }
}

#[async_trait]
impl ProfileExtractor for LlmProfileExtractor {
    async fn extract(&self, resume_text: &str) -> Result<ExtractedProfile, AppError> {
        let outcome = self
            .router
            .extract_profile(resume_text)
            .await
            .map_err(|error| AppError::Provider(error.message))?;
        Ok(outcome.profile)
    }
}

/// Load the persisted profile, falling back to the first-milestone fixture.
pub fn load_profile(state: &AppState) -> Result<CandidateProfile, AppError> {
    if let Some(profile) = db::profile::load_active_profile(&state.db)? {
        return Ok(profile);
    }
    fixture::load(state.config.fixture_profile_path.as_deref())
}

/// Load the profile for matching and block searches when the last resume was
/// deleted without a re-verified profile.
pub fn load_profile_for_search(state: &AppState) -> Result<CandidateProfile, AppError> {
    let profile = load_profile(state)?;
    if !profile.has_resumes() {
        return Err(AppError::ResumeRequired);
    }
    Ok(profile)
}

pub fn save_profile(
    state: &AppState,
    mut profile: CandidateProfile,
) -> Result<CandidateProfile, AppError> {
    profile.acceptable_technologies = dedupe(&profile.acceptable_technologies);
    profile.accepted_cities = dedupe(&profile.accepted_cities);
    profile.extras.resumes_available = true;
    let saved = db::profile::save_profile(&state.db, profile)?;
    db::evaluations::mark_stale(&state.db, "profile updated")?;
    Ok(saved)
}

pub async fn upload_resume_with(
    state: &AppState,
    file_path: &str,
    extractor: &dyn ProfileExtractor,
) -> Result<UploadOutcome, AppError> {
    let path = Path::new(file_path);
    validate_pdf_extension(path)?;
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::validation("the resume filename is invalid"))?
        .to_string();

    let bytes = std::fs::read(path)
        .map_err(|_| AppError::validation("the resume file could not be read"))?;
    if bytes.len() as u64 > pdf::MAX_PDF_BYTES {
        return Err(AppError::validation("the resume PDF is larger than 20 MB"));
    }
    let content_hash = crate::dedup::identity::sha256_bytes(&bytes);

    if let Some(_existing) = db::profile::find_resume_by_hash(&state.db, &content_hash)? {
        let profile =
            db::profile::load_active_profile(&state.db)?.ok_or(AppError::ProfileRequired)?;
        return Ok(UploadOutcome {
            profile,
            duplicate: true,
        });
    }

    let text = pdf::extract_text_from_mem(&bytes)?;
    let extracted = extractor.extract(text.as_str()).await?;
    drop(text);

    let existing = db::profile::load_active_profile(&state.db)?;
    let merged = merge_extracted(existing.as_ref(), &extracted, &content_hash)?;

    let local_path = write_resume_file(&state.config.app_data_dir, &content_hash, &bytes)?;
    let extracted_json = serde_json::to_string(&extracted)?;
    let inserted = db::profile::insert_resume(
        &state.db,
        &filename,
        &content_hash,
        &local_path,
        Some(&extracted_json),
    )?;
    if !inserted.inserted {
        return Ok(UploadOutcome {
            profile: db::profile::load_active_profile(&state.db)?
                .ok_or(AppError::ProfileRequired)?,
            duplicate: true,
        });
    }

    match db::profile::save_profile(&state.db, merged) {
        Ok(profile) => Ok(UploadOutcome {
            profile,
            duplicate: false,
        }),
        Err(error) => {
            let _ = db::profile::delete_resume(&state.db, &inserted.resume.id);
            let _ = std::fs::remove_file(&local_path);
            Err(error)
        }
    }
}

pub fn delete_resume_with(state: &AppState, file_id: &str) -> Result<CandidateProfile, AppError> {
    let deleted = db::profile::delete_resume(&state.db, file_id)?
        .ok_or_else(|| AppError::not_found("resume"))?;
    let _ = std::fs::remove_file(&deleted.local_path);

    let existing = db::profile::load_active_profile(&state.db)?.ok_or(AppError::ProfileRequired)?;
    let remaining = db::profile::list_resumes(&state.db)?;
    if remaining.is_empty() {
        let mut profile = existing;
        profile.extras.resumes_available = false;
        return db::profile::save_profile(&state.db, profile);
    }

    let mut base = reset_extracted(existing);
    for resume in &remaining {
        let Some(json) = &resume.extracted_json else {
            continue;
        };
        let extracted: ExtractedProfile = serde_json::from_str(json)?;
        base = merge_extracted(Some(&base), &extracted, &resume.content_hash)?;
    }
    base.extras.resumes_available = !remaining.is_empty();
    db::profile::save_profile(&state.db, base)
}

fn reset_extracted(mut profile: CandidateProfile) -> CandidateProfile {
    profile.core_competencies.clear();
    profile.secondary_competencies.clear();
    profile.domain_experience.clear();
    profile.extras = ProfileExtras {
        resumes_available: true,
        ..ProfileExtras::default()
    };
    profile.source_documents.clear();
    profile
}

fn validate_pdf_extension(path: &Path) -> Result<(), AppError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if extension.as_deref() != Some("pdf") {
        return Err(AppError::validation("only PDF resume files are supported"));
    }
    Ok(())
}

fn write_resume_file(
    app_data_dir: &Path,
    content_hash: &str,
    bytes: &[u8],
) -> Result<String, AppError> {
    let directory = app_data_dir.join("resumes");
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(format!("{content_hash}.pdf"));
    std::fs::write(&path, bytes)?;
    restrict_permissions(&path)?;
    Ok(path.display().to_string())
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<(), AppError> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), AppError> {
    Ok(())
}

fn dedupe(values: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() && seen.insert(trimmed.to_ascii_lowercase()) {
            result.push(trimmed.to_string());
        }
    }
    result
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResumeSummary {
    pub id: String,
    pub filename: String,
    pub content_hash: String,
    pub uploaded_at: String,
}

pub fn list_resume_summaries(state: &AppState) -> Result<Vec<ResumeSummary>, AppError> {
    Ok(db::profile::list_resumes(&state.db)?
        .into_iter()
        .map(|resume| ResumeSummary {
            id: resume.id,
            filename: resume.filename,
            content_hash: resume.content_hash,
            uploaded_at: resume.uploaded_at,
        })
        .collect())
}

#[tauri::command]
pub async fn cmd_upload_resume(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<CandidateProfile, ErrorPayload> {
    let router = crate::llm::router_from_config(&state.config, state.http.clone());
    let extractor = LlmProfileExtractor::new(router);
    upload_resume_with(&state, &file_path, &extractor)
        .await
        .map(|outcome| outcome.profile)
        .map_err(Into::into)
}

#[tauri::command]
pub fn cmd_delete_resume(
    state: State<'_, AppState>,
    file_id: String,
) -> Result<CandidateProfile, ErrorPayload> {
    delete_resume_with(&state, &file_id).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_list_resumes(state: State<'_, AppState>) -> Result<Vec<ResumeSummary>, ErrorPayload> {
    list_resume_summaries(&state).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_get_profile(state: State<'_, AppState>) -> Result<CandidateProfile, ErrorPayload> {
    load_profile(&state).map_err(Into::into)
}

#[tauri::command]
pub fn cmd_save_profile(
    state: State<'_, AppState>,
    profile: CandidateProfile,
) -> Result<CandidateProfile, ErrorPayload> {
    save_profile(&state, profile).map_err(Into::into)
}
