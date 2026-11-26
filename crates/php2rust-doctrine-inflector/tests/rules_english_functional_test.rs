//! EnglishFunctionalTest.php at doctrine/inflector
//! 6d6c96277ea252fc1304627204c3d5e6e15faa3b. The provider rows are
//! upstream's, recorded by running its provider (`tests/data/inflector.json`).
//!
//! Upstream: doctrine/inflector tests/Rules/English/EnglishFunctionalTest.php

#![allow(non_snake_case)]

mod support;

use php2rust_doctrine_inflector::Language;
use support::language_functional_test_case::{
    create_inflector, test_inflecting_plurals, test_inflecting_singulars,
};
use support::{provider, text};

#[test]
fn testInflectingSingulars() {
    test_inflecting_singulars(Language::ENGLISH, "EnglishFunctionalTest");
}

#[test]
fn testInflectingPlurals() {
    test_inflecting_plurals(Language::ENGLISH, "EnglishFunctionalTest");
}

/// Singulars as Plural test data.
///
/// A list of singulars that should not yield the given result if passed through `singularize`.
#[test]
fn testSingularsWhenSingularizedShouldBeUninflected() {
    let inflector = create_inflector(Language::ENGLISH);
    for row in provider("EnglishFunctionalTest::dataSingularsUninflectedWhenSingularized") {
        let (singular, not_equals) = (&row[0], &row[1]);
        assert_ne!(
            text(not_equals),
            text(inflector.singularize(singular).as_bytes()),
            "'{}' should not be singularized to '{}'",
            text(singular),
            text(not_equals)
        );
    }
}

/// Words without plural test data.
///
/// List of words that don't have a plural form.
#[test]
fn testPluralsWhenPluralizedShouldBeUninflected() {
    let inflector = create_inflector(Language::ENGLISH);
    for row in provider("EnglishFunctionalTest::dataPluralUninflectedWhenPluralized") {
        let plural = &row[0];
        let pluralized = inflector.pluralize(plural);
        assert_eq!(
            text(plural),
            text(pluralized.as_bytes()),
            "'{}' should not be pluralized to '{}'",
            text(plural),
            text(pluralized.as_bytes())
        );
    }
}
