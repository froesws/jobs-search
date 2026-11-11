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

/// Disable database sources that are no longer present in `sources.yaml` and
/// clear their credential reference. Lineage rows are preserved.
pub fn deactivate_missing(db: &Db, active_ids: &[String]) -> Result<(), AppError> {
    let active = active_ids
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    db.with_connection(|connection| {
        let mut statement = connection.prepare("SELECT id FROM job_sources")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut removed = Vec::new();
        for row in rows {
            let id = row?;
            if !active.contains(id.as_str()) {
                removed.push(id);
            }
        }
        for id in removed {
            connection.execute(
                "UPDATE job_sources
                 SET enabled = 0, credential_ref = NULL
                 WHERE id = ?1",
                [id],
            )?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RateLimitConfig, SourceType};
    use crate::db::Db;

    fn source(id: &str) -> SourceConfig {
        SourceConfig {
            id: id.to_string(),
            name: id.to_string(),
            source_type: SourceType::Api,
            enabled: true,
            config: serde_yaml::from_str("base_url: https://example.com/api").expect("config"),
            rate_limit: RateLimitConfig::default(),
            credential_ref: Some("SOURCE_KEY".to_string()),
        }
    }

    #[test]
    fn removed_sources_are_disabled_and_lose_credentials() {
        let db = Db::open_in_memory().expect("db");
        upsert_source(&db, &source("keep")).expect("keep");
        upsert_source(&db, &source("remove")).expect("remove");

        deactivate_missing(&db, &["keep".to_string()]).expect("deactivate");

        let (enabled, credential): (i64, Option<String>) = db
            .with_connection(|connection| {
                Ok(connection.query_row(
                    "SELECT enabled, credential_ref FROM job_sources WHERE id = 'remove'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?)
            })
            .expect("source");
        assert_eq!(enabled, 0);
        assert_eq!(credential, None);
    }
}
