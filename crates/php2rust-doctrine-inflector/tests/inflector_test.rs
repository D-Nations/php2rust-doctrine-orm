//! InflectorTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's; the mocks are `MockWordInflector`s.
//!
//! Upstream: doctrine/inflector tests/InflectorTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

mod support;

use std::sync::Arc;

use php2rust_doctrine_inflector::Inflector;
use support::{MockWordInflector, provider, text};

struct TestCase {
    singular_inflector: Arc<MockWordInflector>,
    plural_inflector: Arc<MockWordInflector>,
    inflector: Inflector,
}

impl TestCase {
    fn set_up_with(singular: MockWordInflector, plural: MockWordInflector) -> Self {
        let singular_inflector = Arc::new(singular);
        let plural_inflector = Arc::new(plural);
        let inflector = Inflector::new(singular_inflector.clone(), plural_inflector.clone());
        Self {
            singular_inflector,
            plural_inflector,
            inflector,
        }
    }

    fn set_up() -> Self {
        Self::set_up_with(MockWordInflector::default(), MockWordInflector::default())
    }
}

fn s(bytes: impl AsRef<[u8]>) -> String {
    text(bytes.as_ref())
}

#[test]
fn testTableize() {
    let t = TestCase::set_up();
    assert_eq!("model_name", s(t.inflector.tableize(b"ModelName").unwrap()));
    assert_eq!("model_name", s(t.inflector.tableize(b"modelName").unwrap()));
    assert_eq!(
        "model_name",
        s(t.inflector.tableize(b"model_name").unwrap())
    );
}

#[test]
fn testClassify() {
    let t = TestCase::set_up();
    assert_eq!("ModelName", s(t.inflector.classify(b"model_name")));
    assert_eq!("ModelName", s(t.inflector.classify(b"modelName")));
    assert_eq!("ModelName", s(t.inflector.classify(b"ModelName")));
}

#[test]
fn testCamelize() {
    let t = TestCase::set_up();
    assert_eq!("modelName", s(t.inflector.camelize(b"ModelName")));
    assert_eq!("modelName", s(t.inflector.camelize(b"model_name")));
    assert_eq!("modelName", s(t.inflector.camelize(b"modelName")));
}

#[test]
fn testCapitalize() {
    let t = TestCase::set_up();
    assert_eq!(
        "Top-O-The-Morning To All_of_you!",
        s(t.inflector
            .capitalize(b"top-o-the-morning to all_of_you!", None))
    );
}

#[test]
fn testSeemsUtf8() {
    let t = TestCase::set_up();
    assert!(t.inflector.seems_utf8("teléfono".as_bytes()));
    assert!(t.inflector.seems_utf8("král".as_bytes()));
    assert!(t.inflector.seems_utf8(b"telephone"));
}

#[test]
fn testUnaccent() {
    let t = TestCase::set_up();
    assert_eq!("telefono", s(t.inflector.unaccent("teléfono".as_bytes())));
    assert_eq!("telephone", s(t.inflector.unaccent(b"telephone")));
}

#[test]
fn testUrlize() {
    let t = TestCase::set_up();
    for row in provider("InflectorTest::dataStringsUrlize") {
        let (expected, string) = (&row[0], &row[1]);
        assert_eq!(s(expected), s(t.inflector.urlize(string).unwrap()));
    }
}

#[test]
fn testPluralize() {
    let t = TestCase::set_up_with(
        MockWordInflector::default(),
        MockWordInflector::returning("in", "out"),
    );

    assert_eq!("out", s(t.inflector.pluralize(b"in")));
    assert_eq!(vec!["in"], t.plural_inflector.calls());
}

#[test]
fn testSingularize() {
    let t = TestCase::set_up_with(
        MockWordInflector::returning("in", "out"),
        MockWordInflector::default(),
    );

    assert_eq!("out", s(t.inflector.singularize(b"in")));
    assert_eq!(vec!["in"], t.singular_inflector.calls());
}
