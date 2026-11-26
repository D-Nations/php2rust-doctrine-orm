//! Port of `DefaultQuoteStrategyTest`. PHPUnit's platform stub is
//! `TestPlatform` (tests/common/platform.rs); the test entity manager's
//! `getClassMetadata()` is the metadata factory on the attribute driver.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/DefaultQuoteStrategyTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;
#[path = "common/platform.rs"]
mod platform;

use std::sync::Arc;

use php_value::php_array;
use php2rust_doctrine_dbal::platforms::SQLitePlatform;
use php2rust_doctrine_orm::mapping::class_metadata::ClassMetadataInfo;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::{
    ClassMetadataFactory, DefaultQuoteStrategy, FactoryConfiguration, QuoteStrategy,
};
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;
use platform::TestPlatform;

#[test]
fn testGetJoinTableName() {
    models::register();
    let cmf = ClassMetadataFactory::new();
    cmf.set_entity_manager(FactoryConfiguration::new(Arc::new(
        AttributeDriver::new(Vec::<String>::new(), true).unwrap(),
    )));
    let metadata = cmf
        .get_metadata_for("Doctrine\\Tests\\Models\\NonPublicSchemaJoins\\User")
        .unwrap();
    let metadata = metadata.read();
    let strategy = DefaultQuoteStrategy::new();
    let platform = SQLitePlatform::new();

    assert_eq!(
        "readers.author_reader",
        strategy.get_join_table_name(
            metadata.association_mappings.get("readers").unwrap(),
            &metadata,
            &platform
        )
    );
}

/// `fullTableNameProvider()`.
const FULL_TABLE_NAMES: [(&str, &str, &str); 4] = [
    (
        "quoted table name with schema",
        "✌️custom✌️.✌️reserved✌️",
        "custom.`reserved`",
    ),
    (
        "unquoted table name with schema",
        "custom.non_reserved",
        "custom.non_reserved",
    ),
    (
        "quoted table name without schema",
        "✌️reserved✌️",
        "`reserved`",
    ),
    (
        "unquoted table name without schema",
        "non_reserved",
        "non_reserved",
    ),
];

#[test]
fn testGetTableName() {
    for (data_set, expected_full_table_name, table_name) in FULL_TABLE_NAMES {
        let mut class_metadata = ClassMetadataInfo::new(
            "Doctrine\\Tests\\ORM\\Mapping\\DefaultQuoteStrategyTest",
            None,
            None,
        );
        class_metadata.set_primary_table(&php_array! { "name" => table_name });

        let platform =
            TestPlatform::new("Doctrine\\DBAL\\Platforms\\AbstractPlatform").quoting("✌️%s✌️");

        let quoted_table_name =
            DefaultQuoteStrategy::new().get_table_name(&class_metadata, &platform);

        assert_eq!(expected_full_table_name, quoted_table_name, "{data_set}");
    }
}
