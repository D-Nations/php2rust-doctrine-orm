//! TransformationTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's.
//!
//! Upstream: doctrine/inflector tests/Rules/TransformationTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::PhpString;
use php2rust_doctrine_inflector::WordInflector;
use php2rust_doctrine_inflector::rules::{Pattern, Transformation};

fn set_up() -> Transformation {
    Transformation::new(Pattern::new("s$").unwrap(), "")
}

#[test]
fn testGetPattern() {
    assert_eq!(&PhpString::from("s$"), set_up().get_pattern().get_pattern());
}

#[test]
fn testGetReplacement() {
    assert_eq!(&PhpString::from(""), set_up().get_replacement());
}

#[test]
fn testInflect() {
    assert_eq!(PhpString::from("test"), set_up().inflect(b"tests"));
}
