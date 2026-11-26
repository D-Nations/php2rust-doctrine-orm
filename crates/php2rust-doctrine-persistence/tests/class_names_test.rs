//! Port of tests/Mapping/Driver/ClassNamesTest.php at doctrine/persistence
//! d8293831c5b99449b2f63eb1e1bbdc202118daa6.
//!
//! Upstream: doctrine/persistence tests/Mapping/Driver/ClassNamesTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

mod common;

use common::{ENTITY, ENTITY_FIXTURE};
use php2rust_doctrine_persistence::mapping::driver::{ClassLocator, ClassNames};

#[test]
fn testGetClassNames() {
    let class_names = vec![ENTITY.to_owned(), ENTITY_FIXTURE.to_owned()];

    let locator = ClassNames::new(class_names.clone());

    assert_eq!(class_names, locator.get_class_names().unwrap());
}
