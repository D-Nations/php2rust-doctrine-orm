//! Port of `TableMappingTest`: `new Table(indexes: [])` and
//! `new Table(uniqueConstraints: [])` trigger the deprecation (the
//! attribute's constructor runs in `attributes::new_instance`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/TableMappingTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::PhpArray;
use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::mapping::attributes::{self, TABLE_INDEXES_DEPRECATION, Table};

fn expect_deprecation(table: Table) {
    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(TABLE_INDEXES_DEPRECATION);
    attributes::new_instance(&table.into());
    assert!(deprecations::count(TABLE_INDEXES_DEPRECATION) > before);
}

#[test]
fn testDeprecationOnIndexesPropertyIsTriggered() {
    expect_deprecation(Table {
        indexes: Some(PhpArray::new()),
        ..Table::default()
    });
}

#[test]
fn testDeprecationOnUniqueConstraintsPropertyIsTriggered() {
    expect_deprecation(Table {
        unique_constraints: Some(PhpArray::new()),
        ..Table::default()
    });
}
