//! JSON Lines writer with rotation and retention.
#![allow(missing_docs)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use chrono::{DateTime, NaiveDate, Utc};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

use crate::error::AppError;
use crate::telemetry::config::TelemetryConfig;
use crate::telemetry::health::HealthState;
use crate::telemetry::queue::BoundedQueue;

pub struct WriterHandle {
    pub thread: JoinHandle<()>,
    pub queue: BoundedQueue,
    pub health: HealthState,
}

pub fn spawn_writer(
    config: &TelemetryConfig,
    queue: BoundedQueue,
    health: HealthState,
) -> Result<WriterHandle, AppError> {
    fs::create_dir_all(&config.log_dir)?;
    set_dir_permissions(&config.log_dir)?;

    cleanup_logs(
        &config.log_dir,
        Utc::now(),
        config.retention_days,
        config.max_total_bytes,
    )?;

    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("jobs-search")
        .filename_suffix("log")
        .build(&config.log_dir)
        .map_err(|error| AppError::Telemetry(format!("could not create log appender: {error}")))?;

    let worker_queue = queue.clone();
    let worker_health = health.clone();
    let log_dir = config.log_dir.clone();
    let retention_days = config.retention_days;
    let max_total_bytes = config.max_total_bytes;

    let thread = thread::Builder::new()
        .name("jobs-search-telemetry-writer".to_string())
        .spawn(move || {
            let mut appender = appender;
            let mut last_cleanup_day = Utc::now().date_naive();

            loop {
                let Some(batch) = worker_queue.pop_batch(256) else {
                    let _ = appender.flush();
                    break;
                };

                let today = Utc::now().date_naive();
                worker_health.set_current_file(Some(format!(
                    "jobs-search.{}.log",
                    today.format("%Y-%m-%d")
                )));

                if today != last_cleanup_day {
                    last_cleanup_day = today;
                    let _ = cleanup_logs(&log_dir, Utc::now(), retention_days, max_total_bytes);
                }

                for line in batch {
                    match appender
                        .write_all(&line)
                        .and_then(|_| appender.write_all(b"\n").and_then(|_| appender.flush()))
                    {
                        Ok(()) => worker_health.mark_success(),
                        Err(error) => worker_health.mark_error(&error.to_string()),
                    }
                }
            }

            let _ = appender.flush();
        })
        .map_err(|error| AppError::Telemetry(format!("could not start log writer: {error}")))?;

    Ok(WriterHandle {
        thread,
        queue,
        health,
    })
}

/// Delete expired log files and then enforce the total-size cap by removing the
/// oldest daily files first.
pub fn cleanup_logs(
    directory: &Path,
    now: DateTime<Utc>,
    retention_days: u64,
    max_total_bytes: u64,
) -> Result<u64, AppError> {
    let mut files: Vec<(PathBuf, NaiveDate, u64)> = Vec::new();

    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(AppError::Io(error.to_string())),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name,
            None => continue,
        };
        if let Some(date) = parse_log_date(name) {
            files.push((path, date, entry.metadata()?.len()));
        }
    }

    let cutoff = now.date_naive() - chrono::Duration::days(retention_days as i64);
    let mut removed = 0;
    files.retain(|(path, date, _)| {
        if *date < cutoff {
            if fs::remove_file(path).is_ok() {
                removed += 1;
            }
            false
        } else {
            true
        }
    });

    let mut total: u64 = files.iter().map(|(_, _, size)| *size).sum();
    files.sort_by_key(|(_, date, _)| *date);
    for (path, _, size) in files {
        if total <= max_total_bytes {
            break;
        }
        if fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(size);
            removed += 1;
        }
    }

    Ok(removed)
}

pub fn parse_log_date(name: &str) -> Option<NaiveDate> {
    let stem = name.strip_prefix("jobs-search.")?.strip_suffix(".log")?;
    NaiveDate::parse_from_str(stem, "%Y-%m-%d").ok()
}

#[cfg(unix)]
fn set_dir_permissions(path: &Path) -> Result<(), AppError> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_dir_permissions(_path: &Path) -> Result<(), AppError> {
    Ok(())
}

/// Wait for the writer thread to finish, bounded by the spec's 5-second limit.
pub fn join_writer(handle: JoinHandle<()>, timeout: Duration) {
    let started = Instant::now();
    let (sender, receiver) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let _ = handle.join();
        let _ = sender.send(());
    });
    let remaining = timeout.saturating_sub(started.elapsed());
    let _ = receiver.recv_timeout(remaining);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_daily_file_names() {
        assert!(parse_log_date("jobs-search.2026-11-10.log").is_some());
        assert!(parse_log_date("jobs-search.log.2026-11-10").is_none());
        assert!(parse_log_date("other.2026-11-10.log").is_none());
    }

    #[test]
    fn removes_expired_and_oversized_files() {
        let dir = tempdir().unwrap();
        let old = dir.path().join("jobs-search.2020-01-01.log");
        let recent_old = dir.path().join("jobs-search.2026-11-08.log");
        let recent_new = dir.path().join("jobs-search.2026-11-09.log");
        fs::write(&old, vec![b'a'; 8]).unwrap();
        fs::write(&recent_old, vec![b'b'; 8]).unwrap();
        fs::write(&recent_new, vec![b'c'; 8]).unwrap();

        let now = DateTime::parse_from_rfc3339("2026-11-10T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        cleanup_logs(dir.path(), now, 7, 8).unwrap();

        assert!(!old.exists());
        assert!(!recent_old.exists());
        assert!(recent_new.exists());
    }

    #[test]
    fn writes_jsonl_and_flushes() {
        let dir = tempdir().unwrap();
        let queue = BoundedQueue::new(10);
        let health = HealthState::new(queue.clone());
        let config = TelemetryConfig::new(
            dir.path().to_path_buf(),
            crate::telemetry::event::Level::Info,
        );
        let writer = spawn_writer(&config, queue.clone(), health.clone()).unwrap();
        queue.push(b"{\"event_type\":\"APP_START\"}".to_vec());
        queue.close();
        join_writer(writer.thread, Duration::from_secs(5));

        let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert!(!entries.is_empty());
        let path = entries[0].as_ref().unwrap().path();
        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("APP_START"));
    }
}
