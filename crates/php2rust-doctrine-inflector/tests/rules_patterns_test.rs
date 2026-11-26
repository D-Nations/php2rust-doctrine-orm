//! PatternsTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's.
//!
//! Upstream: doctrine/inflector tests/Rules/PatternsTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php2rust_doctrine_inflector::rules::{Pattern, Patterns, WordMatcher};

#[test]
fn testMatches() {
    let patterns = Patterns::new([Pattern::new("test1").unwrap()]).unwrap();

    assert!(patterns.matches(b"test1"));
    assert!(!patterns.matches(b"test2"));
}
