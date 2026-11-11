//! Candidate profile and resume persistence.
//!
//! Exactly one profile is active. Resume content hashes are unique, and resume
//! paths are app-managed and never logged.

use rusqlite::{OptionalExtension, params};

use crate::db::{Db, now_rfc3339};
use crate::error::AppError;
use crate::profile::CandidateProfile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredResume {
    pub id: String,
    pub filename: String,
    pub content_hash: String,
    pub local_path: String,
    pub uploaded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeInsertOutcome {
    pub inserted: bool,
    pub resume: StoredResume,
}

pub fn load_active_profile(db: &Db) -> Result<Option<CandidateProfile>, AppError> {
    let json: Option<String> = db.with_connection(|connection| {
        Ok(connection
            .query_row(
                "SELECT profile_json FROM candidate_profiles ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?)
    })?;

    match json {
        Some(value) => {
            let profile: CandidateProfile = serde_json::from_str(&value)?;
            Ok(Some(profile))
        }
        None => Ok(None),
    }
}

/// Persist the single active profile. The stored version always advances
/// monotonically, even if the caller submits an older version.
pub fn save_profile(db: &Db, mut profile: CandidateProfile) -> Result<CandidateProfile, AppError> {
    profile.validate()?;
    let existing = load_active_profile(db)?;
    let next_version = existing
        .as_ref()
        .map(|current| current.version.saturating_add(1))
        .unwrap_or(1)
        .max(profile.version);
    profile.version = next_version;
    profile.updated_at = now_rfc3339();

    let profile_json = serde_json::to_string(&profile)?;
    let technologies = serde_json::to_string(&profile.acceptable_technologies)?;
    let cities = serde_json::to_string(&profile.accepted_cities)?;

    db.with_connection_mut(|connection| {
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM candidate_profiles", [])?;
        transaction.execute(
            "INSERT INTO candidate_profiles
             (id, profile_json, acceptable_technologies_json, accepted_cities_json, version, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                profile.profile_id,
                profile_json,
                technologies,
                cities,
                profile.version,
                profile.updated_at,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    })?;

    Ok(profile)
}

pub fn find_resume_by_hash(db: &Db, content_hash: &str) -> Result<Option<StoredResume>, AppError> {
    db.with_connection(|connection| {
        Ok(connection
            .query_row(
                "SELECT id, filename, content_hash, local_path, uploaded_at
                 FROM resumes WHERE content_hash = ?1",
                [content_hash],
                resume_from_row,
            )
            .optional()?)
    })
}

pub fn insert_resume(
    db: &Db,
    filename: &str,
    content_hash: &str,
    local_path: &str,
) -> Result<ResumeInsertOutcome, AppError> {
    if let Some(existing) = find_resume_by_hash(db, content_hash)? {
        return Ok(ResumeInsertOutcome {
            inserted: false,
            resume: existing,
        });
    }

    let resume = StoredResume {
        id: uuid::Uuid::new_v4().to_string(),
        filename: filename.to_string(),
        content_hash: content_hash.to_string(),
        local_path: local_path.to_string(),
        uploaded_at: now_rfc3339(),
    };

    db.with_connection(|connection| {
        connection.execute(
            "INSERT INTO resumes (id, filename, content_hash, local_path, uploaded_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                resume.id,
                resume.filename,
                resume.content_hash,
                resume.local_path,
                resume.uploaded_at,
            ],
        )?;
        Ok(())
    })?;

    Ok(ResumeInsertOutcome {
        inserted: true,
        resume,
    })
}

pub fn list_resumes(db: &Db) -> Result<Vec<StoredResume>, AppError> {
    db.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, filename, content_hash, local_path, uploaded_at
             FROM resumes ORDER BY uploaded_at ASC",
        )?;
        let rows = statement.query_map([], resume_from_row)?;
        let mut resumes = Vec::new();
        for row in rows {
            resumes.push(row?);
        }
        Ok(resumes)
    })
}

pub fn delete_resume(db: &Db, id: &str) -> Result<Option<StoredResume>, AppError> {
    let resume = db.with_connection(|connection| {
        Ok(connection
            .query_row(
                "SELECT id, filename, content_hash, local_path, uploaded_at
                 FROM resumes WHERE id = ?1",
                [id],
                resume_from_row,
            )
            .optional()?)
    })?;

    if resume.is_some() {
        db.with_connection(|connection| {
            connection.execute("DELETE FROM resumes WHERE id = ?1", [id])?;
            Ok(())
        })?;
    }

    Ok(resume)
}

pub fn resume_count(db: &Db) -> Result<i64, AppError> {
    db.with_connection(|connection| {
        Ok(connection.query_row("SELECT COUNT(*) FROM resumes", [], |row| row.get(0))?)
    })
}

fn resume_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredResume> {
    Ok(StoredResume {
        id: row.get(0)?,
        filename: row.get(1)?,
        content_hash: row.get(2)?,
        local_path: row.get(3)?,
        uploaded_at: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> CandidateProfile {
        let mut value: CandidateProfile =
            serde_json::from_str(include_str!("../../tests/fixtures/profile.json"))
                .expect("fixture");
        value.version = 1;
        value
    }

    #[test]
    fn profile_version_advances_and_only_one_row_remains() {
        let db = Db::open_in_memory().expect("db");
        let first = save_profile(&db, profile()).expect("first");
        let second = save_profile(&db, first.clone()).expect("second");
        assert_eq!(first.version, 1);
        assert_eq!(second.version, 2);
        let count: i64 = db
            .with_connection(|connection| {
                Ok(
                    connection.query_row("SELECT COUNT(*) FROM candidate_profiles", [], |row| {
                        row.get(0)
                    })?,
                )
            })
            .expect("count");
        assert_eq!(count, 1);
    }

    #[test]
    fn resume_content_hash_is_unique() {
        let db = Db::open_in_memory().expect("db");
        let first = insert_resume(&db, "a.pdf", "hash", "/tmp/a.pdf").expect("first");
        let second = insert_resume(&db, "b.pdf", "hash", "/tmp/b.pdf").expect("second");
        assert!(first.inserted);
        assert!(!second.inserted);
        assert_eq!(list_resumes(&db).expect("list").len(), 1);
        assert_eq!(resume_count(&db).expect("count"), 1);
    }

    #[test]
    fn rejects_invalid_profile_without_overwriting() {
        let db = Db::open_in_memory().expect("db");
        save_profile(&db, profile()).expect("save");
        let mut invalid = profile();
        invalid.target_roles.clear();
        assert!(save_profile(&db, invalid).is_err());
        assert!(load_active_profile(&db).expect("load").is_some());
    }
}
