//! TransformationsTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's.
//!
//! Upstream: doctrine/inflector tests/Rules/TransformationsTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::PhpString;
use php2rust_doctrine_inflector::WordInflector;
use php2rust_doctrine_inflector::rules::{Pattern, Transformation, Transformations};

#[test]
fn testInflect() {
    let transformations = Transformations::new([Transformation::new(
        Pattern::new("/^(custom)$/i").unwrap(),
        r"\1izables",
    )]);

    assert_eq!(
        PhpString::from("customizables"),
        transformations.inflect(b"custom")
    );
}
