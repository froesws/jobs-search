//! Log store health snapshot.
#![allow(missing_docs)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::telemetry::queue::BoundedQueue;
use crate::telemetry::sanitize::sanitize_message;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogHealth {
    pub available: bool,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub dropped_count: u64,
    pub last_error: Option<String>,
    pub last_success_at: Option<String>,
    pub current_file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct HealthState {
    inner: Arc<HealthInner>,
}

#[derive(Debug)]
struct HealthInner {
    queue: BoundedQueue,
    available: AtomicBool,
    last_error: Mutex<Option<String>>,
    last_success_at: Mutex<Option<String>>,
    current_file: Mutex<Option<String>>,
}

impl HealthState {
    pub fn new(queue: BoundedQueue) -> Self {
        Self {
            inner: Arc::new(HealthInner {
                queue,
                available: AtomicBool::new(true),
                last_error: Mutex::new(None),
                last_success_at: Mutex::new(None),
                current_file: Mutex::new(None),
            }),
        }
    }

    pub fn snapshot(&self) -> LogHealth {
        LogHealth {
            available: self.inner.available.load(Ordering::Relaxed),
            queue_depth: self.inner.queue.depth(),
            queue_capacity: self.inner.queue.capacity(),
            dropped_count: self.inner.queue.dropped_count(),
            last_error: self
                .inner
                .last_error
                .lock()
                .expect("health mutex poisoned")
                .clone(),
            last_success_at: self
                .inner
                .last_success_at
                .lock()
                .expect("health mutex poisoned")
                .clone(),
            current_file: self
                .inner
                .current_file
                .lock()
                .expect("health mutex poisoned")
                .clone(),
        }
    }

    pub fn mark_available(&self, available: bool) {
        self.inner.available.store(available, Ordering::Relaxed);
    }

    pub fn mark_success(&self) {
        self.inner.available.store(true, Ordering::Relaxed);
        *self
            .inner
            .last_success_at
            .lock()
            .expect("health mutex poisoned") = Some(now());
    }

    pub fn mark_error(&self, message: &str) {
        self.inner.available.store(false, Ordering::Relaxed);
        let safe = sanitize_message(message).unwrap_or_else(|| "[redacted]".to_string());
        *self.inner.last_error.lock().expect("health mutex poisoned") = Some(safe);
    }

    pub fn set_current_file(&self, file: Option<String>) {
        *self
            .inner
            .current_file
            .lock()
            .expect("health mutex poisoned") = file;
    }

    pub fn mark_unavailable(&self, message: &str) {
        self.mark_error(message);
        self.mark_available(false);
    }
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reflects_queue_and_errors() {
        let queue = BoundedQueue::new(2);
        queue.push(b"one".to_vec());
        let health = HealthState::new(queue.clone());
        health.set_current_file(Some("jobs-search.2026-11-10.log".to_string()));
        health.mark_error("disk full");

        let snapshot = health.snapshot();
        assert!(!snapshot.available);
        assert_eq!(snapshot.queue_depth, 1);
        assert_eq!(snapshot.queue_capacity, 2);
        assert_eq!(snapshot.last_error.as_deref(), Some("disk full"));
        assert_eq!(
            snapshot.current_file.as_deref(),
            Some("jobs-search.2026-11-10.log")
        );
    }

    #[test]
    fn success_clears_availability() {
        let health = HealthState::new(BoundedQueue::new(1));
        health.mark_error("temporary");
        health.mark_success();
        assert!(health.snapshot().available);
        assert!(health.snapshot().last_success_at.is_some());
    }
}
