//! Bounded oldest-first event queue.
#![allow(missing_docs)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone, Debug)]
pub struct BoundedQueue {
    inner: Arc<QueueInner>,
}

#[derive(Debug)]
struct QueueInner {
    items: Mutex<VecDeque<Vec<u8>>>,
    ready: Condvar,
    capacity: usize,
    closed: AtomicBool,
    dropped: AtomicU64,
}

impl BoundedQueue {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "queue capacity must be positive");
        Self {
            inner: Arc::new(QueueInner {
                items: Mutex::new(VecDeque::with_capacity(capacity)),
                ready: Condvar::new(),
                capacity,
                closed: AtomicBool::new(false),
                dropped: AtomicU64::new(0),
            }),
        }
    }

    pub fn push(&self, line: Vec<u8>) {
        let mut items = self.inner.items.lock().expect("telemetry queue poisoned");
        if items.len() >= self.inner.capacity {
            items.pop_front();
            self.inner.dropped.fetch_add(1, Ordering::Relaxed);
        }
        items.push_back(line);
        self.inner.ready.notify_one();
    }

    pub fn try_pop_batch(&self, max: usize) -> Vec<Vec<u8>> {
        let mut items = self.inner.items.lock().expect("telemetry queue poisoned");
        let count = items.len().min(max);
        items.drain(..count).collect()
    }

    pub fn pop_batch(&self, max: usize) -> Option<Vec<Vec<u8>>> {
        let mut items = self.inner.items.lock().expect("telemetry queue poisoned");
        loop {
            if !items.is_empty() {
                let count = items.len().min(max);
                return Some(items.drain(..count).collect());
            }
            if self.inner.closed.load(Ordering::SeqCst) {
                return None;
            }
            items = self
                .inner
                .ready
                .wait(items)
                .expect("telemetry queue poisoned");
        }
    }

    pub fn close(&self) {
        self.inner.closed.store(true, Ordering::SeqCst);
        self.inner.ready.notify_all();
    }

    pub fn depth(&self) -> usize {
        self.inner
            .items
            .lock()
            .expect("telemetry queue poisoned")
            .len()
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity
    }

    pub fn dropped_count(&self) -> u64 {
        self.inner.dropped.load(Ordering::Relaxed)
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_oldest_when_full() {
        let queue = BoundedQueue::new(3);
        for index in 0..5 {
            queue.push(format!("line-{index}").into_bytes());
        }

        assert_eq!(queue.depth(), 3);
        assert_eq!(queue.dropped_count(), 2);

        let batch = queue.try_pop_batch(10);
        let lines: Vec<String> = batch
            .into_iter()
            .map(|line| String::from_utf8(line).unwrap())
            .collect();
        assert_eq!(lines, vec!["line-2", "line-3", "line-4"]);
    }

    #[test]
    fn close_releases_waiters() {
        let queue = BoundedQueue::new(1);
        let clone = queue.clone();
        let handle = std::thread::spawn(move || clone.pop_batch(1));
        queue.close();
        assert!(handle.join().unwrap().is_none());
    }
}
