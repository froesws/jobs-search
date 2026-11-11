use rusqlite::params;

use crate::config::SourceConfig;
use crate::db::Db;
use crate::error::AppError;

pub fn upsert_source(db: &Db, source: &SourceConfig) -> Result<(), AppError> {
    let config_json = source.config_json()?;
    let rate_limit_json = serde_json::to_string(&source.rate_limit)?;
    db.with_connection(|connection| {
        connection.execute(
            "INSERT INTO job_sources
             (id, name, source_type, config_json, enabled, rate_limit_json, credential_ref)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                source_type = excluded.source_type,
                config_json = excluded.config_json,
                enabled = excluded.enabled,
                rate_limit_json = excluded.rate_limit_json,
                credential_ref = excluded.credential_ref",
            params![
                source.id,
                source.name,
                source.source_type.as_str(),
                config_json,
                i64::from(source.enabled),
                rate_limit_json,
                source.credential_ref,
            ],
        )?;
        Ok(())
    })
}

pub fn set_last_run_status(db: &Db, source_id: &str, status: &str) -> Result<(), AppError> {
    db.with_connection(|connection| {
        connection.execute(
            "UPDATE job_sources SET last_run_status = ?1 WHERE id = ?2",
            params![status, source_id],
        )?;
        Ok(())
    })
}
