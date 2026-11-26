//! CachedWordInflectorTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's; the mock is `MockWordInflector`.
//!
//! Upstream: doctrine/inflector tests/CachedWordInflectorTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

mod support;

use std::sync::Arc;

use php_value::PhpString;
use php2rust_doctrine_inflector::{CachedWordInflector, WordInflector};
use support::MockWordInflector;

#[test]
fn testInflect() {
    let word_inflector = Arc::new(MockWordInflector::returning("in", "out"));
    let cached_word_inflector = CachedWordInflector::new(word_inflector.clone());

    assert_eq!(PhpString::from("out"), cached_word_inflector.inflect(b"in"));
    assert_eq!(PhpString::from("out"), cached_word_inflector.inflect(b"in"));
    // expects(self::once())->with('in')
    assert_eq!(vec!["in"], word_inflector.calls());
}
