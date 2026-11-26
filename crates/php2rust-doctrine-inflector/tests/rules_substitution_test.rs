//! SubstitutionTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's.
//!
//! Upstream: doctrine/inflector tests/Rules/SubstitutionTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::PhpString;
use php2rust_doctrine_inflector::rules::{Substitution, Word};

fn set_up() -> Substitution {
    Substitution::new(Word::new("from"), Word::new("to"))
}

#[test]
fn testGetFrom() {
    assert_eq!(&PhpString::from("from"), set_up().get_from().get_word());
}

#[test]
fn testGetTo() {
    assert_eq!(&PhpString::from("to"), set_up().get_to().get_word());
}
