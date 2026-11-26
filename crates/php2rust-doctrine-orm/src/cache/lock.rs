//! `Doctrine\ORM\Cache\Lock`: a lock's value and the time it was taken.
//!
//! `uniqid((string) time(), true)` is the time prefix, the hexadecimal
//! seconds and microseconds, and a number from a per-process sequence in
//! place of the combined LCG.
//!
//! Upstream: doctrine/orm src/Cache/Lock.php

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    pub value: String,
    pub time: i64,
}

impl Lock {
    /// `new Lock($value, $time)`: a time of `null` is `time()`.
    pub fn new(value: impl Into<String>, time: Option<i64>) -> Self {
        Self {
            value: value.into(),
            time: time.unwrap_or_else(php_value::clock::time),
        }
    }

    /// `Lock::createLockRead()`.
    pub fn create_lock_read() -> Self {
        Self::new(uniqid(&php_value::clock::time().to_string()), None)
    }
}

/// `uniqid($prefix, true)`.
pub fn uniqid(prefix: &str) -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!(
        "{prefix}{:08x}{:05x}.{:08}",
        now.as_secs(),
        now.subsec_micros(),
        sequence % 100_000_000
    )
}
