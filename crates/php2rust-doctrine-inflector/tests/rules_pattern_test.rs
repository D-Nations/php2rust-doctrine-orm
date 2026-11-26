//! PatternTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's.
//!
//! Upstream: doctrine/inflector tests/Rules/PatternTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::PhpString;
use php2rust_doctrine_inflector::rules::Pattern;

fn set_up() -> Pattern {
    Pattern::new("test").unwrap()
}

#[test]
fn testGetPattern() {
    assert_eq!(&PhpString::from("test"), set_up().get_pattern());
}

#[test]
fn testGetRegex() {
    assert_eq!(&PhpString::from("/test/i"), set_up().get_regex());
}

#[test]
fn testPatternWithExplicitRegex() {
    let pattern = Pattern::new("/test/").unwrap();

    assert_eq!(&PhpString::from("/test/"), pattern.get_regex());
}

#[test]
fn testMatches() {
    assert!(set_up().matches(b"test"));
}
