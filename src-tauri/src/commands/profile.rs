//! Profile and resume commands.
#![allow(missing_docs)]

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
use crate::types::ResumeId;

#[derive(Debug, Clone)]
pub struct UploadOutcome {
    pub resume_id: String,
    pub profile: Option<CandidateProfile>,
    pub duplicate: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UploadResumeResult {
    pub resume_id: String,
    pub profile: Option<CandidateProfile>,
    pub duplicate: bool,
    pub warning: Option<String>,
}

impl From<UploadOutcome> for UploadResumeResult {
    fn from(outcome: UploadOutcome) -> Self {
        Self {
            resume_id: outcome.resume_id,
            profile: outcome.profile,
            duplicate: outcome.duplicate,
            warning: outcome.warning,
        }
    }
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
        match self.router.extract_profile(resume_text).await {
            Ok(outcome) => Ok(outcome.profile),
            Err(error) => {
                tracing::warn!(
                    category = error.category.as_str(),
                    provider = error.last_provider.as_deref().unwrap_or("none"),
                    model = error.last_model.as_deref().unwrap_or("none"),
                    message = %error.message,
                    "profile extraction failed"
                );
                Err(AppError::Provider(error.message))
            }
        }
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
    let path = Path::new(file_path).to_path_buf();
    validate_pdf_extension(&path)?;
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::validation("the resume filename is invalid"))?
        .to_string();

    // File I/O and hashing are CPU/disk bound; keep them off the async
    // executor so a large PDF cannot freeze the UI.
    let read_path = path.clone();
    let (bytes, content_hash) =
        tokio::task::spawn_blocking(move || -> Result<(Vec<u8>, String), AppError> {
            let bytes = std::fs::read(&read_path)
                .map_err(|_| AppError::validation("the resume file could not be read"))?;
            if bytes.len() as u64 > pdf::MAX_PDF_BYTES {
                return Err(AppError::validation("the resume PDF is larger than 20 MB"));
            }
            let content_hash = crate::dedup::identity::sha256_bytes(&bytes);
            Ok((bytes, content_hash))
        })
        .await
        .map_err(|_| AppError::Database("resume read task failed".to_string()))??;

    if let Some(existing) = db::profile::find_resume_by_hash(&state.db, &content_hash)? {
        return Ok(UploadOutcome {
            resume_id: existing.id.as_str().to_string(),
            warning: duplicate_warning(existing.extracted_json.as_deref()),
            profile: db::profile::load_active_profile(&state.db)?,
            duplicate: true,
        });
    }

    let extract_bytes = bytes.clone();
    let text = tokio::task::spawn_blocking(move || pdf::extract_text_from_mem(&extract_bytes))
        .await
        .map_err(|_| AppError::Database("resume extraction task failed".to_string()))??;

    let app_data = state.config.app_data_dir.clone();
    let hash_for_path = content_hash.clone();
    let local_path =
        tokio::task::spawn_blocking(move || write_resume_file(&app_data, &hash_for_path, &bytes))
            .await
            .map_err(|_| AppError::Database("resume write task failed".to_string()))??;

    let inserted =
        match db::profile::insert_resume(&state.db, &filename, &content_hash, &local_path, None) {
            Ok(inserted) => inserted,
            Err(error) => {
                let _ = std::fs::remove_file(&local_path);
                return Err(error);
            }
        };

    if !inserted.inserted {
        return Ok(UploadOutcome {
            resume_id: inserted.resume.id.as_str().to_string(),
            warning: duplicate_warning(inserted.resume.extracted_json.as_deref()),
            profile: db::profile::load_active_profile(&state.db)?,
            duplicate: true,
        });
    }

    let resume_id = inserted.resume.id.as_str().to_string();
    let extracted = match extractor.extract(text.as_str()).await {
        Ok(extracted) => extracted,
        Err(error) => {
            drop(text);
            tracing::warn!(
                resume_id = %resume_id,
                code = error.code(),
                message = %error.message(),
                "resume saved but profile extraction failed"
            );
            return resume_outcome_with_warning(
                state,
                resume_id,
                false,
                format!(
                    "Resume saved, but profile extraction failed: {}",
                    error.message()
                ),
            );
        }
    };
    drop(text);

    let extracted_json = match serde_json::to_string(&extracted) {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(
                resume_id = %resume_id,
                message = %error,
                "resume saved but extraction serialization failed"
            );
            return resume_outcome_with_warning(
                state,
                resume_id,
                false,
                format!("Resume saved, but the extracted profile could not be stored: {error}"),
            );
        }
    };

    if let Err(error) =
        db::profile::update_resume_extracted(&state.db, &inserted.resume.id, Some(&extracted_json))
    {
        tracing::warn!(
            resume_id = %resume_id,
            message = %error.message(),
            "resume saved but extracted profile backfill failed"
        );
        return resume_outcome_with_warning(
            state,
            resume_id,
            false,
            format!(
                "Resume saved, but the extracted profile could not be stored: {}",
                error.message()
            ),
        );
    }

    let existing = db::profile::load_active_profile(&state.db)?;
    let merged = match merge_extracted(existing.as_ref(), &extracted, &content_hash) {
        Ok(merged) => merged,
        Err(error) => {
            tracing::warn!(
                resume_id = %resume_id,
                message = %error.message(),
                "resume saved but profile merge failed"
            );
            return resume_outcome_with_warning(
                state,
                resume_id,
                false,
                format!(
                    "Resume saved, but the profile could not be updated: {}",
                    error.message()
                ),
            );
        }
    };

    match db::profile::save_profile(&state.db, merged) {
        Ok(profile) => Ok(UploadOutcome {
            resume_id,
            profile: Some(profile),
            duplicate: false,
            warning: None,
        }),
        Err(error) => {
            tracing::warn!(
                resume_id = %resume_id,
                message = %error.message(),
                "resume saved but profile save failed"
            );
            resume_outcome_with_warning(
                state,
                resume_id,
                false,
                format!(
                    "Resume saved, but the profile could not be updated: {}",
                    error.message()
                ),
            )
        }
    }
}

fn duplicate_warning(extracted_json: Option<&str>) -> Option<String> {
    extracted_json.is_none().then(|| {
        "This resume is already stored, but its profile extraction did not complete. Delete it and upload again after fixing the LLM configuration."
            .to_string()
    })
}

fn resume_outcome_with_warning(
    state: &AppState,
    resume_id: String,
    duplicate: bool,
    warning: String,
) -> Result<UploadOutcome, AppError> {
    Ok(UploadOutcome {
        resume_id,
        profile: db::profile::load_active_profile(&state.db)?,
        duplicate,
        warning: Some(warning),
    })
}

pub fn delete_resume_with(
    state: &AppState,
    file_id: &str,
) -> Result<Option<CandidateProfile>, AppError> {
    let resume_id = ResumeId::from(file_id);
    let deleted = db::profile::delete_resume(&state.db, &resume_id)?
        .ok_or_else(|| AppError::not_found("resume"))?;
    let _ = std::fs::remove_file(&deleted.local_path);

    let existing = db::profile::load_active_profile(&state.db)?;
    let remaining = db::profile::list_resumes(&state.db)?;
    if remaining.is_empty() {
        return match existing {
            Some(mut profile) => {
                profile.extras.resumes_available = false;
                Ok(Some(db::profile::save_profile(&state.db, profile)?))
            }
            None => Ok(None),
        };
    }

    let mut merged = existing.map(reset_extracted);
    for resume in &remaining {
        let Some(json) = &resume.extracted_json else {
            continue;
        };
        let extracted: ExtractedProfile = serde_json::from_str(json)?;
        merged = Some(merge_extracted(
            merged.as_ref(),
            &extracted,
            &resume.content_hash,
        )?);
    }

    match merged {
        Some(mut profile) => {
            profile.extras.resumes_available = true;
            Ok(Some(db::profile::save_profile(&state.db, profile)?))
        }
        None => Ok(None),
    }
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
            id: resume.id.as_str().to_string(),
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
) -> Result<UploadResumeResult, ErrorPayload> {
    let router = crate::llm::router_from_config(&state.config, state.http.clone());
    let extractor = LlmProfileExtractor::new(router);
    upload_resume_with(&state, &file_path, &extractor)
        .await
        .map(UploadResumeResult::from)
        .map_err(Into::into)
}

#[tauri::command]
pub fn cmd_delete_resume(
    state: State<'_, AppState>,
    file_id: String,
) -> Result<Option<CandidateProfile>, ErrorPayload> {
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
