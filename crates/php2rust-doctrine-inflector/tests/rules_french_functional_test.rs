//! FrenchFunctionalTest.php at doctrine/inflector
//! 6d6c96277ea252fc1304627204c3d5e6e15faa3b. The provider rows are
//! upstream's, recorded by running its provider (`tests/data/inflector.json`).
//!
//! Upstream: doctrine/inflector tests/Rules/French/FrenchFunctionalTest.php

#![allow(non_snake_case)]

mod support;

use php2rust_doctrine_inflector::Language;
use support::language_functional_test_case::{test_inflecting_plurals, test_inflecting_singulars};

#[test]
fn testInflectingSingulars() {
    test_inflecting_singulars(Language::FRENCH, "FrenchFunctionalTest");
}

#[test]
fn testInflectingPlurals() {
    test_inflecting_plurals(Language::FRENCH, "FrenchFunctionalTest");
}
