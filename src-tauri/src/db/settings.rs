//! Operator-editable application settings.
//!
//! Settings are stored as key/value rows in `app_settings`. Credentials are
//! never stored here; they live in the OS credential store.

use rusqlite::{OptionalExtension, params};

use crate::db::{Db, now_rfc3339};
use crate::error::AppError;

/// Read a setting value, returning `None` when the key is absent.
pub fn get(db: &Db, key: &str) -> Result<Option<String>, AppError> {
    db.with_connection(|connection| {
        let value = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value)
    })
}

/// Insert or replace a setting value.
pub fn put(db: &Db, key: &str, value: &str) -> Result<(), AppError> {
    db.with_connection(|connection| {
        connection.execute(
            "INSERT INTO app_settings (key, value, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at",
            params![key, value, now_rfc3339()],
        )?;
        Ok(())
    })
}

/// Delete a setting. Deleting a missing key is not an error.
pub fn delete(db: &Db, key: &str) -> Result<(), AppError> {
    db.with_connection(|connection| {
        connection.execute("DELETE FROM app_settings WHERE key = ?1", params![key])?;
        Ok(())
    })
}

/// List every setting whose key starts with `prefix`, ordered by key.
pub fn list_prefix(db: &Db, prefix: &str) -> Result<Vec<(String, String)>, AppError> {
    db.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT key, value FROM app_settings
             WHERE key LIKE ?1 ESCAPE '\\'
             ORDER BY key",
        )?;
        let pattern = format!("{}%", escape_like(prefix));
        let rows = statement.query_map(params![pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut values = Vec::new();
        for row in rows {
            values.push(row?);
        }
        Ok(values)
    })
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get_and_delete_round_trip() {
        let db = Db::open_in_memory().expect("db");
        assert_eq!(get(&db, "provider.order").expect("get"), None);

        put(&db, "provider.order", "[\"gemini\"]").expect("put");
        assert_eq!(
            get(&db, "provider.order").expect("get"),
            Some("[\"gemini\"]".to_string())
        );

        put(&db, "provider.order", "[\"deepseek\"]").expect("upsert");
        assert_eq!(
            get(&db, "provider.order").expect("get"),
            Some("[\"deepseek\"]".to_string())
        );

        delete(&db, "provider.order").expect("delete");
        assert_eq!(get(&db, "provider.order").expect("get"), None);
        delete(&db, "provider.order").expect("delete is idempotent");
    }

    #[test]
    fn list_prefix_returns_only_matching_keys_in_order() {
        let db = Db::open_in_memory().expect("db");
        put(&db, "provider.gemini.model", "a").expect("put");
        put(&db, "provider.gemini.attempts", "3").expect("put");
        put(&db, "provider.deepseek.model", "b").expect("put");
        put(&db, "other.key", "c").expect("put");

        let values = list_prefix(&db, "provider.gemini.").expect("list");
        assert_eq!(
            values,
            vec![
                ("provider.gemini.attempts".to_string(), "3".to_string()),
                ("provider.gemini.model".to_string(), "a".to_string()),
            ]
        );
    }

    #[test]
    fn list_prefix_escapes_like_wildcards() {
        let db = Db::open_in_memory().expect("db");
        put(&db, "a_b", "1").expect("put");
        put(&db, "axb", "2").expect("put");

        let values = list_prefix(&db, "a_").expect("list");
        assert_eq!(values, vec![("a_b".to_string(), "1".to_string())]);
    }
}
