//! Port of tests/Mapping/StaticPHPDriverTest.php at doctrine/persistence
//! d8293831c5b99449b2f63eb1e1bbdc202118daa6, over a copy of the fixture
//! files in `tests/data/Mapping`. `Entity::loadMetadata()` is the
//! fixture class's `ClassInfo::load_metadata`.
//!
//! Upstream: doctrine/persistence tests/Mapping/StaticPHPDriverTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(
    clippy::doc_markdown,
    clippy::disallowed_names,
    reason = "upstream's test, class and field names"
)]
#![allow(non_snake_case)]

mod common;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use common::{ENTITY, StubClassMetadata, mapping_dir};
use php2rust_doctrine_persistence::mapping::driver::{
    ClassNames, MappingDriver, StaticPHPDriver, StaticPHPDriverPaths,
};

#[test]
fn testLoadMetadata() {
    common::register_fixtures();
    let metadata = StubClassMetadata::new();

    let driver = StaticPHPDriver::new(StaticPHPDriverPaths::Paths(Vec::new()));
    driver.load_metadata_for_class(ENTITY, &*metadata).unwrap();

    // $metadata->expects(self::once())->method('getFieldNames')
    assert_eq!(metadata.get_field_names_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn testGetAllClassNames() {
    common::register_fixtures();
    let driver = StaticPHPDriver::new(StaticPHPDriverPaths::Paths(vec![format!(
        "{}/_files/colocated/",
        mapping_dir()
    )]));
    let class_names = driver.get_all_class_names().unwrap();

    assert!(class_names.iter().any(|c| c == ENTITY));
}

#[test]
fn testGetAllClassesNamesWithClassLocator() {
    common::register_fixtures();
    let driver = StaticPHPDriver::new(StaticPHPDriverPaths::Locator(Arc::new(ClassNames::new([
        ENTITY,
    ]))));
    let class_names = driver.get_all_class_names().unwrap();

    assert_eq!(vec![ENTITY.to_owned()], class_names);
}
