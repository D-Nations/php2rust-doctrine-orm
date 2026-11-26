//! SubstitutionsTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's.
//!
//! Upstream: doctrine/inflector tests/Rules/SubstitutionsTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::PhpString;
use php2rust_doctrine_inflector::WordInflector;
use php2rust_doctrine_inflector::rules::{Substitution, Substitutions, Word};

fn set_up() -> Substitutions {
    let substitutions = [Substitution::new(Word::new("spins"), Word::new("spinor"))];
    Substitutions::new(substitutions)
}

#[test]
fn testGetFlippedSubstitutions() {
    let substitutions = set_up().get_flipped_substitutions();

    assert_eq!(PhpString::from("spins"), substitutions.inflect(b"spinor"));
}

#[test]
fn testInflect() {
    assert_eq!(PhpString::from("spinor"), set_up().inflect(b"spins"));
}
