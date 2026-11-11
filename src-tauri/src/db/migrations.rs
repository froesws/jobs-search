use rusqlite::Connection;

use crate::error::AppError;

const SCHEMA_VERSION: i64 = 4;

const SCHEMA_V1: &str = r#"
CREATE TABLE resumes (
    id TEXT PRIMARY KEY,
    filename TEXT NOT NULL,
    content_hash TEXT NOT NULL UNIQUE,
    local_path TEXT NOT NULL,
    uploaded_at TEXT NOT NULL
);

CREATE TABLE candidate_profiles (
    id TEXT PRIMARY KEY,
    profile_json TEXT NOT NULL,
    acceptable_technologies_json TEXT NOT NULL,
    accepted_cities_json TEXT NOT NULL,
    version INTEGER NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE job_sources (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    source_type TEXT NOT NULL CHECK (source_type IN ('mcp', 'api', 'web_scraper')),
    config_json TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    rate_limit_json TEXT NOT NULL,
    credential_ref TEXT,
    last_run_status TEXT CHECK (last_run_status IN ('SUCCESS', 'PARTIAL', 'FAILED'))
);

CREATE TABLE job_postings (
    id TEXT PRIMARY KEY,
    canonical_key TEXT NOT NULL UNIQUE,
    canonical_hash TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    company TEXT NOT NULL,
    location TEXT NOT NULL,
    work_modality TEXT NOT NULL CHECK (work_modality IN ('Remote', 'Hybrid', 'Onsite', 'Unknown')),
    url TEXT NOT NULL,
    posted_at TEXT,
    description_clean TEXT NOT NULL,
    extracted_metadata_json TEXT,
    conflicts_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_job_postings_updated_at ON job_postings(updated_at);

CREATE TABLE job_posting_sources (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES job_postings(id) ON DELETE CASCADE,
    source_id TEXT NOT NULL REFERENCES job_sources(id) ON DELETE CASCADE,
    external_id TEXT NOT NULL,
    url TEXT NOT NULL,
    ingested_at TEXT NOT NULL,
    UNIQUE (job_id, source_id, external_id)
);

CREATE INDEX idx_job_posting_sources_job ON job_posting_sources(job_id);

CREATE TABLE job_evaluations (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL UNIQUE REFERENCES job_postings(id) ON DELETE CASCADE,
    profile_version INTEGER NOT NULL,
    evaluation_status TEXT NOT NULL CHECK (evaluation_status IN ('PENDING', 'APPROVED', 'REJECTED', 'ERROR')),
    match_score INTEGER CHECK (match_score IS NULL OR (match_score >= 0 AND match_score <= 100)),
    matching_items_count INTEGER CHECK (matching_items_count IS NULL OR matching_items_count >= 0),
    total_requirements_count INTEGER CHECK (total_requirements_count IS NULL OR total_requirements_count >= 0),
    score_breakdown_json TEXT,
    matched_keywords_json TEXT,
    missing_keywords_json TEXT,
    hard_filter_rejections_json TEXT,
    rules_version TEXT NOT NULL,
    threshold INTEGER NOT NULL CHECK (threshold >= 0 AND threshold <= 100),
    provider TEXT,
    model TEXT,
    evaluated_at TEXT,
    stale INTEGER NOT NULL DEFAULT 0 CHECK (stale IN (0, 1))
);

CREATE INDEX idx_job_evaluations_status ON job_evaluations(evaluation_status);
CREATE INDEX idx_job_evaluations_evaluated_at ON job_evaluations(job_id, evaluated_at);

CREATE TABLE job_applications (
    job_id TEXT PRIMARY KEY REFERENCES job_postings(id) ON DELETE CASCADE,
    lifecycle_state TEXT NOT NULL CHECK (lifecycle_state IN ('DISCOVERED', 'APPLIED', 'IGNORED')),
    applied_at TEXT,
    approval_source TEXT NOT NULL CHECK (approval_source IN ('automatic', 'manual')),
    evaluation_snapshot_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_job_applications_state ON job_applications(lifecycle_state);

CREATE TABLE job_status_events (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES job_postings(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN ('EVALUATED', 'APPROVED', 'REJECTED', 'APPLIED', 'IGNORED', 'RESTORED')),
    old_value TEXT,
    new_value TEXT NOT NULL,
    reason TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_job_status_events_job ON job_status_events(job_id, created_at);

CREATE TABLE execution_runs (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    search_config_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('RUNNING', 'SUCCESS', 'PARTIAL', 'FAILED')),
    total_ingested INTEGER NOT NULL DEFAULT 0 CHECK (total_ingested >= 0),
    duplicates_merged INTEGER NOT NULL DEFAULT 0 CHECK (duplicates_merged >= 0),
    rejected_count INTEGER NOT NULL DEFAULT 0 CHECK (rejected_count >= 0),
    pending_count INTEGER NOT NULL DEFAULT 0 CHECK (pending_count >= 0),
    approved_count INTEGER NOT NULL DEFAULT 0 CHECK (approved_count >= 0),
    error_count INTEGER NOT NULL DEFAULT 0 CHECK (error_count >= 0),
    source_failures_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE run_metrics (
    run_id TEXT PRIMARY KEY REFERENCES execution_runs(id) ON DELETE CASCADE,
    total_ingested INTEGER NOT NULL DEFAULT 0,
    duplicates_merged INTEGER NOT NULL DEFAULT 0,
    rejected_by_reason_json TEXT NOT NULL DEFAULT '{}',
    pending_count INTEGER NOT NULL DEFAULT 0,
    approved_count INTEGER NOT NULL DEFAULT 0,
    applied_count INTEGER NOT NULL DEFAULT 0,
    ignored_count INTEGER NOT NULL DEFAULT 0,
    approval_rate REAL NOT NULL DEFAULT 0,
    application_rate REAL NOT NULL DEFAULT 0,
    score_distribution_json TEXT NOT NULL DEFAULT '{}',
    source_breakdown_json TEXT NOT NULL DEFAULT '{}',
    provider_breakdown_json TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL
);

CREATE TABLE llm_metrics (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES execution_runs(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    operation TEXT NOT NULL CHECK (operation IN ('profile_extraction', 'job_evaluation')),
    latency_ms INTEGER NOT NULL CHECK (latency_ms >= 0),
    outcome TEXT NOT NULL CHECK (outcome IN ('SUCCESS', 'ERROR', 'FALLBACK')),
    error_category TEXT CHECK (error_category IN ('TIMEOUT', 'RATE_LIMIT', 'AUTH', 'NETWORK', 'PARSE', 'CONFIG', 'UNKNOWN')),
    fallback_from TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_llm_metrics_run ON llm_metrics(run_id, created_at);
"#;

/// V2 links every evaluation to the discovery/evaluation run that produced it
/// so analytics can resolve the current run without relying on timestamps.
const SCHEMA_V2: &str = r#"
ALTER TABLE job_evaluations
    ADD COLUMN run_id TEXT REFERENCES execution_runs(id) ON DELETE SET NULL;

CREATE INDEX idx_job_evaluations_run ON job_evaluations(run_id);
"#;

/// V3 flags postings whose canonical identity had to fall back to the URL so
/// the operator can review the missing company/title/location fields.
const SCHEMA_V3: &str = r#"
ALTER TABLE job_postings
    ADD COLUMN needs_review INTEGER NOT NULL DEFAULT 0 CHECK (needs_review IN (0, 1));

CREATE INDEX idx_job_postings_needs_review ON job_postings(needs_review);
"#;

/// V4 stores each resume's validated extraction so deleting a resume can
/// recompute the profile without calling the LLM again or losing other sources.
const SCHEMA_V4: &str = r#"
ALTER TABLE resumes ADD COLUMN extracted_json TEXT;
"#;

pub fn apply(connection: &mut Connection) -> Result<(), AppError> {
    let current: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if current >= SCHEMA_VERSION {
        return Ok(());
    }

    let transaction = connection.transaction()?;
    if current < 1 {
        transaction.execute_batch(SCHEMA_V1)?;
    }
    if current < 2 {
        transaction.execute_batch(SCHEMA_V2)?;
    }
    if current < 3 {
        transaction.execute_batch(SCHEMA_V3)?;
    }
    if current < 4 {
        transaction.execute_batch(SCHEMA_V4)?;
    }
    transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::db::Db;

    #[test]
    fn migration_is_idempotent_and_has_no_raw_description() {
        let db = Db::open_in_memory().expect("db");
        let tables = db.table_names().expect("tables");
        for expected in [
            "job_postings",
            "job_posting_sources",
            "job_evaluations",
            "job_applications",
            "job_status_events",
            "execution_runs",
            "run_metrics",
            "llm_metrics",
            "job_sources",
        ] {
            assert!(tables.iter().any(|name| name == expected), "{expected}");
        }

        let columns: Vec<String> = db
            .with_connection(|connection| {
                let mut statement = connection.prepare("PRAGMA table_info(job_postings)")?;
                let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
                let mut names = Vec::new();
                for row in rows {
                    names.push(row?);
                }
                Ok(names)
            })
            .expect("columns");
        assert!(!columns.iter().any(|column| column == "description_raw"));

        // A second open must not try to recreate the schema.
        let db = Db::open_in_memory().expect("reopen");
        assert!(
            db.table_names()
                .expect("tables")
                .contains(&"job_postings".to_string())
        );
    }
}
