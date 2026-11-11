use std::collections::BTreeMap;

use rusqlite::{OptionalExtension, params};

use crate::db::Db;
use crate::db::now_rfc3339;
use crate::error::AppError;
use crate::types::{EvaluationStatus, HardFilterRejection, JobTab, LifecycleState, WorkModality};

const SUMMARY_COLUMNS: &str = "
    j.id, j.title, j.company, j.location, j.work_modality, j.url, j.posted_at,
    e.evaluation_status, COALESCE(a.lifecycle_state, 'DISCOVERED'),
    e.match_score, e.matching_items_count, e.total_requirements_count,
    substr(j.description_clean, 1, 240)
";

#[derive(Debug, Clone)]
pub struct NewJob {
    pub canonical_key: String,
    pub canonical_hash: String,
    pub title: String,
    pub company: String,
    pub location: String,
    pub work_modality: WorkModality,
    pub url: String,
    pub posted_at: Option<String>,
    pub description_clean: String,
    pub extracted_metadata_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StoredJob {
    pub id: String,
    pub title: String,
    pub company: String,
    pub location: String,
    pub work_modality: WorkModality,
    pub url: String,
    pub posted_at: Option<String>,
    pub description_clean: String,
    pub extracted_metadata_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertOutcome {
    pub job_id: String,
    pub inserted: bool,
}

impl UpsertOutcome {
    pub fn duplicate(&self) -> bool {
        !self.inserted
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct JobSummary {
    pub job_id: String,
    pub title: String,
    pub company: String,
    pub location: String,
    pub work_modality: String,
    pub url: String,
    pub posted_at: Option<String>,
    pub evaluation_status: Option<String>,
    pub lifecycle_state: String,
    pub match_score: Option<i64>,
    pub matching_items_count: Option<i64>,
    pub total_requirements_count: Option<i64>,
    pub summary: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SourceLineage {
    pub source_id: String,
    pub source_name: String,
    pub external_id: String,
    pub url: String,
    pub ingested_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct JobDetail {
    #[serde(flatten)]
    pub summary: JobSummary,
    pub description_clean: String,
    pub requirements: Vec<crate::llm::schemas::Requirement>,
    pub matched_keywords: Vec<String>,
    pub missing_keywords: Vec<String>,
    pub hard_filter_rejections: Vec<HardFilterRejection>,
    pub source_lineage: Vec<SourceLineage>,
    pub score_breakdown: Option<serde_json::Value>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub error_category: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PaginatedJobs {
    pub items: Vec<JobSummary>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
    pub tab: JobTab,
}

pub fn upsert_job(db: &Db, job: &NewJob) -> Result<UpsertOutcome, AppError> {
    db.with_connection(|connection| {
        let now = now_rfc3339();
        let existing: Option<String> = connection
            .query_row(
                "SELECT id FROM job_postings WHERE canonical_hash = ?1",
                [&job.canonical_hash],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(job_id) = existing {
            connection.execute(
                "UPDATE job_postings
                 SET title = ?1, company = ?2, location = ?3, work_modality = ?4,
                     url = ?5, posted_at = COALESCE(?6, posted_at),
                     description_clean = ?7,
                     extracted_metadata_json = COALESCE(?8, extracted_metadata_json),
                     updated_at = ?9
                 WHERE id = ?10",
                params![
                    job.title,
                    job.company,
                    job.location,
                    job.work_modality.as_str(),
                    job.url,
                    job.posted_at,
                    job.description_clean,
                    job.extracted_metadata_json,
                    now,
                    job_id,
                ],
            )?;
            Ok(UpsertOutcome {
                job_id,
                inserted: false,
            })
        } else {
            let job_id = uuid::Uuid::new_v4().to_string();
            connection.execute(
                "INSERT INTO job_postings
                 (id, canonical_key, canonical_hash, title, company, location,
                  work_modality, url, posted_at, description_clean,
                  extracted_metadata_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    job_id,
                    job.canonical_key,
                    job.canonical_hash,
                    job.title,
                    job.company,
                    job.location,
                    job.work_modality.as_str(),
                    job.url,
                    job.posted_at,
                    job.description_clean,
                    job.extracted_metadata_json,
                    now,
                    now,
                ],
            )?;
            Ok(UpsertOutcome {
                job_id,
                inserted: true,
            })
        }
    })
}

pub fn add_lineage(
    db: &Db,
    job_id: &str,
    source_id: &str,
    external_id: &str,
    url: &str,
) -> Result<(), AppError> {
    db.with_connection(|connection| {
        connection.execute(
            "INSERT INTO job_posting_sources
             (id, job_id, source_id, external_id, url, ingested_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(job_id, source_id, external_id)
             DO UPDATE SET url = excluded.url, ingested_at = excluded.ingested_at",
            params![
                uuid::Uuid::new_v4().to_string(),
                job_id,
                source_id,
                external_id,
                url,
                now_rfc3339(),
            ],
        )?;
        Ok(())
    })
}

pub fn lineage(db: &Db, job_id: &str) -> Result<Vec<SourceLineage>, AppError> {
    db.with_connection(|connection| lineage_with_connection(connection, job_id))
}

fn lineage_with_connection(
    connection: &rusqlite::Connection,
    job_id: &str,
) -> Result<Vec<SourceLineage>, AppError> {
    let mut statement = connection.prepare(
        "SELECT s.id, s.name, l.external_id, l.url, l.ingested_at
         FROM job_posting_sources l
         JOIN job_sources s ON s.id = l.source_id
         WHERE l.job_id = ?1
         ORDER BY s.name ASC, l.ingested_at ASC",
    )?;
    let rows = statement.query_map([job_id], |row| {
        Ok(SourceLineage {
            source_id: row.get(0)?,
            source_name: row.get(1)?,
            external_id: row.get(2)?,
            url: row.get(3)?,
            ingested_at: row.get(4)?,
        })
    })?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }
    Ok(values)
}

pub fn lineage_urls(db: &Db, job_id: &str) -> Result<Vec<String>, AppError> {
    let mut urls = lineage(db, job_id)?
        .into_iter()
        .map(|value| value.url)
        .collect::<Vec<_>>();
    urls.sort();
    urls.dedup();
    Ok(urls)
}

pub fn update_extracted_metadata(
    db: &Db,
    job_id: &str,
    metadata_json: &str,
) -> Result<(), AppError> {
    db.with_connection(|connection| {
        connection.execute(
            "UPDATE job_postings SET extracted_metadata_json = ?1, updated_at = ?2 WHERE id = ?3",
            params![metadata_json, now_rfc3339(), job_id],
        )?;
        Ok(())
    })
}

pub fn list_for_evaluation(db: &Db, limit: usize) -> Result<Vec<StoredJob>, AppError> {
    db.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT j.id, j.title, j.company, j.location, j.work_modality, j.url,
                    j.posted_at, j.description_clean, j.extracted_metadata_json
             FROM job_postings j
             LEFT JOIN job_evaluations e ON e.job_id = j.id
             WHERE e.job_id IS NULL OR e.stale = 1
             ORDER BY j.updated_at DESC, j.id ASC
             LIMIT ?1",
        )?;
        let rows = statement.query_map([limit as i64], |row| {
            Ok(StoredJob {
                id: row.get(0)?,
                title: row.get(1)?,
                company: row.get(2)?,
                location: row.get(3)?,
                work_modality: WorkModality::parse(&row.get::<_, String>(4)?),
                url: row.get(5)?,
                posted_at: row.get(6)?,
                description_clean: row.get(7)?,
                extracted_metadata_json: row.get(8)?,
            })
        })?;
        let mut values = Vec::new();
        for row in rows {
            values.push(row?);
        }
        Ok(values)
    })
}

pub fn count_jobs(db: &Db) -> Result<i64, AppError> {
    db.with_connection(|connection| {
        Ok(connection.query_row("SELECT COUNT(*) FROM job_postings", [], |row| row.get(0))?)
    })
}

pub fn list_summaries(
    db: &Db,
    tab: JobTab,
    limit: u32,
    offset: u32,
) -> Result<PaginatedJobs, AppError> {
    let limit = limit.clamp(1, 100);
    let (filter, order) = match tab {
        JobTab::NewJobs => (
            "WHERE e.evaluation_status = 'APPROVED'
             AND COALESCE(a.lifecycle_state, 'DISCOVERED') <> 'IGNORED'",
            "ORDER BY e.match_score DESC, e.evaluated_at DESC, j.id ASC",
        ),
        JobTab::BelowThreshold => (
            "WHERE e.evaluation_status = 'PENDING'",
            "ORDER BY e.match_score DESC, e.evaluated_at DESC, j.id ASC",
        ),
        JobTab::Applications => (
            "WHERE a.lifecycle_state = 'APPLIED'",
            "ORDER BY a.applied_at DESC, j.id ASC",
        ),
    };

    db.with_connection(|connection| {
        let count_sql = format!(
            "SELECT COUNT(*) FROM job_postings j
             LEFT JOIN job_evaluations e ON e.job_id = j.id
             LEFT JOIN job_applications a ON a.job_id = j.id
             {filter}"
        );
        let total: i64 = connection.query_row(&count_sql, [], |row| row.get(0))?;

        let query = format!(
            "SELECT {SUMMARY_COLUMNS}
             FROM job_postings j
             LEFT JOIN job_evaluations e ON e.job_id = j.id
             LEFT JOIN job_applications a ON a.job_id = j.id
             {filter}
             {order}
             LIMIT ?1 OFFSET ?2"
        );
        let mut statement = connection.prepare(&query)?;
        let rows = statement.query_map(params![limit as i64, offset as i64], summary_from_row)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }

        Ok(PaginatedJobs {
            items,
            total,
            limit,
            offset,
            tab,
        })
    })
}

pub fn get_detail(db: &Db, job_id: &str) -> Result<JobDetail, AppError> {
    db.with_connection(|connection| {
        let query = format!(
            "SELECT {SUMMARY_COLUMNS}, j.description_clean,
                    e.matched_keywords_json, e.missing_keywords_json,
                    e.hard_filter_rejections_json, e.score_breakdown_json,
                    e.provider, e.model, j.extracted_metadata_json
             FROM job_postings j
             LEFT JOIN job_evaluations e ON e.job_id = j.id
             LEFT JOIN job_applications a ON a.job_id = j.id
             WHERE j.id = ?1"
        );

        let mut statement = connection.prepare(&query)?;
        let mut rows = statement.query([job_id])?;
        let Some(row) = rows.next()? else {
            return Err(AppError::not_found("job"));
        };

        let summary = summary_from_row(row)?;
        let description_clean: String = row.get(13)?;
        let matched_keywords = parse_csv_json(row.get::<_, Option<String>>(14)?);
        let missing_keywords = parse_csv_json(row.get::<_, Option<String>>(15)?);
        let hard_filter_rejections = parse_rejections(row.get::<_, Option<String>>(16)?);
        let score_breakdown = parse_json_value(row.get::<_, Option<String>>(17)?);
        let provider: Option<String> = row.get(18)?;
        let model: Option<String> = row.get(19)?;
        let metadata: Option<String> = row.get(20)?;
        let requirements = parse_requirements(metadata.as_deref());
        let error_category = error_category(score_breakdown.as_ref());
        let source_lineage = lineage_with_connection(connection, job_id)?;

        drop(rows);
        drop(statement);

        Ok(JobDetail {
            summary,
            description_clean,
            requirements,
            matched_keywords,
            missing_keywords,
            hard_filter_rejections,
            source_lineage,
            score_breakdown,
            provider,
            model,
            error_category,
        })
    })
}

fn summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobSummary> {
    Ok(JobSummary {
        job_id: row.get(0)?,
        title: row.get(1)?,
        company: row.get(2)?,
        location: row.get(3)?,
        work_modality: row.get(4)?,
        url: row.get(5)?,
        posted_at: row.get(6)?,
        evaluation_status: row.get(7)?,
        lifecycle_state: row.get(8)?,
        match_score: row.get(9)?,
        matching_items_count: row.get(10)?,
        total_requirements_count: row.get(11)?,
        summary: truncate(&row.get::<_, String>(12)?, 240),
    })
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut output: String = value.chars().take(max_chars).collect();
    output.push_str("...");
    output
}

fn parse_csv_json(value: Option<String>) -> Vec<String> {
    value
        .and_then(|value| serde_json::from_str::<Vec<String>>(&value).ok())
        .unwrap_or_default()
}

fn parse_rejections(value: Option<String>) -> Vec<HardFilterRejection> {
    value
        .and_then(|value| serde_json::from_str::<Vec<HardFilterRejection>>(&value).ok())
        .unwrap_or_default()
}

fn parse_json_value(value: Option<String>) -> Option<serde_json::Value> {
    value.and_then(|value| serde_json::from_str(&value).ok())
}

#[derive(serde::Deserialize)]
struct MetadataShape {
    #[serde(default)]
    requirements: Vec<crate::llm::schemas::Requirement>,
}

fn parse_requirements(metadata: Option<&str>) -> Vec<crate::llm::schemas::Requirement> {
    metadata
        .and_then(|value| serde_json::from_str::<MetadataShape>(value).ok())
        .map(|shape| shape.requirements)
        .unwrap_or_default()
}

fn error_category(breakdown: Option<&serde_json::Value>) -> Option<String> {
    breakdown
        .and_then(|value| value.get("error_category"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// Group lineage counts by source name for run metrics.
pub fn source_breakdown(db: &Db) -> Result<BTreeMap<String, i64>, AppError> {
    db.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT s.name, COUNT(*) FROM job_posting_sources l
             JOIN job_sources s ON s.id = l.source_id
             GROUP BY s.name ORDER BY s.name",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut values = BTreeMap::new();
        for row in rows {
            let (name, count) = row?;
            values.insert(name, count);
        }
        Ok(values)
    })
}

/// Source lineage counts ingested at or after a run start.
pub fn source_breakdown_since(
    db: &Db,
    started_at: &str,
) -> Result<BTreeMap<String, i64>, AppError> {
    db.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT s.name, COUNT(*) FROM job_posting_sources l
             JOIN job_sources s ON s.id = l.source_id
             WHERE l.ingested_at >= ?1
             GROUP BY s.name ORDER BY s.name",
        )?;
        let rows = statement.query_map([started_at], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut values = BTreeMap::new();
        for row in rows {
            let (name, count) = row?;
            values.insert(name, count);
        }
        Ok(values)
    })
}

/// Evaluation status counts used by the search metrics response.
pub fn status_counts(db: &Db) -> Result<(i64, i64, i64, i64), AppError> {
    db.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT evaluation_status, COUNT(*) FROM job_evaluations GROUP BY evaluation_status",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut rejected = 0;
        let mut pending = 0;
        let mut approved = 0;
        let mut error = 0;
        for row in rows {
            let (status, count) = row?;
            match EvaluationStatus::parse(&status) {
                Some(EvaluationStatus::Rejected) => rejected = count,
                Some(EvaluationStatus::Pending) => pending = count,
                Some(EvaluationStatus::Approved) => approved = count,
                Some(EvaluationStatus::Error) => error = count,
                None => {}
            }
        }
        Ok((rejected, pending, approved, error))
    })
}

pub fn lifecycle(db: &Db, job_id: &str) -> Result<LifecycleState, AppError> {
    db.with_connection(|connection| {
        let state: Option<String> = connection
            .query_row(
                "SELECT lifecycle_state FROM job_applications WHERE job_id = ?1",
                [job_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(state
            .as_deref()
            .and_then(|value| match value {
                "APPLIED" => Some(LifecycleState::Applied),
                "IGNORED" => Some(LifecycleState::Ignored),
                _ => None,
            })
            .unwrap_or(LifecycleState::Discovered))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_job(title: &str) -> NewJob {
        NewJob {
            canonical_key: format!("v1|acme|{title}|remote"),
            canonical_hash: crate::dedup::identity::sha256_hex(title),
            title: title.to_string(),
            company: "Acme".to_string(),
            location: "Remote".to_string(),
            work_modality: WorkModality::Remote,
            url: "https://example.com/1".to_string(),
            posted_at: None,
            description_clean: "Rust role".to_string(),
            extracted_metadata_json: None,
        }
    }

    #[test]
    fn upsert_is_repeat_safe_and_preserves_lineage() {
        let db = Db::open_in_memory().expect("db");
        crate::db::sources::upsert_source(
            &db,
            &crate::config::SourceConfig {
                id: "jobspy".to_string(),
                name: "jobspy".to_string(),
                source_type: crate::config::SourceType::Mcp,
                enabled: true,
                config: serde_yaml::from_str("command: python3").expect("config"),
                rate_limit: crate::config::RateLimitConfig::default(),
                credential_ref: None,
            },
        )
        .expect("source");

        let first = upsert_job(&db, &test_job("Rust Engineer")).expect("insert");
        assert!(first.inserted);
        let second = upsert_job(&db, &test_job("Rust Engineer")).expect("update");
        assert!(!second.inserted);
        assert_eq!(first.job_id, second.job_id);
        assert_eq!(count_jobs(&db).expect("count"), 1);

        add_lineage(&db, &first.job_id, "jobspy", "li-1", "https://a/1").expect("lineage");
        add_lineage(&db, &first.job_id, "jobspy", "li-1", "https://a/1").expect("lineage repeat");
        assert_eq!(lineage(&db, &first.job_id).expect("lineage").len(), 1);
    }
}
