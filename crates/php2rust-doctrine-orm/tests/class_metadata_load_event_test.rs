//! Port of `ClassMetadataLoadEventTest`: the `loadClassMetadata` listener
//! is a hook of the factory's configuration.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/ClassMetadataLoadEventTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use std::sync::Arc;

use php_value::php_array;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::{ClassMetadataFactory, FactoryConfiguration};
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;

#[test]
fn testEvent() {
    models::register();
    let mut configuration = FactoryConfiguration::new(Arc::new(
        AttributeDriver::new(Vec::<String>::new(), true).unwrap(),
    ));
    // loadClassMetadata()
    configuration
        .load_class_metadata
        .push(Arc::new(|class_metadata| {
            let field = php_array! {
                "fieldName" => "about",
                "type" => "string",
                "length" => 255,
            };
            class_metadata.write().map_field(field)
        }));
    let metadata_factory = ClassMetadataFactory::new();
    metadata_factory.set_entity_manager(configuration);

    let class_metadata = metadata_factory
        .get_metadata_for("Doctrine\\Tests\\ORM\\Mapping\\LoadEventTestEntity")
        .unwrap();
    let class_metadata = class_metadata.read();
    assert!(class_metadata.has_field("about"));
    // A PropertyAccessor, by the map's type.
    assert!(class_metadata.property_accessors.get("about").is_some());
}
