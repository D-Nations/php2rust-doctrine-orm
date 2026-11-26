//! NoopWordInflectorTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's.
//!
//! Upstream: doctrine/inflector tests/NoopWordInflectorTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::PhpString;
use php2rust_doctrine_inflector::{NoopWordInflector, WordInflector};

#[test]
fn testInflect() {
    let inflector = NoopWordInflector;
    assert_eq!(PhpString::from("foo"), inflector.inflect(b"foo"));
    assert_eq!(PhpString::from("bar"), inflector.inflect(b"bar"));
}
