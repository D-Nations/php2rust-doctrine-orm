//! InflectorFunctionalTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's; the provider rows are recorded by running upstream's providers.
//!
//! Upstream: doctrine/inflector tests/InflectorFunctionalTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

mod support;

use php2rust_doctrine_inflector::{Inflector, InflectorFactory};
use support::{provider, text};

fn create_inflector() -> Inflector {
    InflectorFactory::create().unwrap().build().unwrap()
}

#[test]
fn testCapitalize() {
    assert_eq!(
        "Top-O-The-Morning To All_of_you!",
        text(
            create_inflector()
                .capitalize(b"top-o-the-morning to all_of_you!", None)
                .as_bytes()
        )
    );
}

#[test]
fn testCapitalizeWithCustomDelimiters() {
    assert_eq!(
        "Top-O-The-Morning To All_Of_You!",
        text(
            create_inflector()
                .capitalize(b"top-o-the-morning to all_of_you!", Some(b"-_ "))
                .as_bytes()
        )
    );
}

#[test]
fn testTableize() {
    for row in provider("InflectorFunctionalTest::dataStringsTableize") {
        let (expected, word) = (&row[0], &row[1]);
        assert_eq!(
            text(expected),
            text(create_inflector().tableize(word).unwrap().as_bytes())
        );
    }
}

#[test]
fn testClassify() {
    for row in provider("InflectorFunctionalTest::dataStringsClassify") {
        let (expected, word) = (&row[0], &row[1]);
        assert_eq!(
            text(expected),
            text(create_inflector().classify(word).as_bytes())
        );
    }
}

#[test]
fn testCamelize() {
    for row in provider("InflectorFunctionalTest::dataStringsCamelize") {
        let (expected, word) = (&row[0], &row[1]);
        assert_eq!(
            text(expected),
            text(create_inflector().camelize(word).as_bytes())
        );
    }
}
