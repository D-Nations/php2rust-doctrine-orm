//! Ticket/GH12172Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models and its own class).
//!
//! `expectNotToPerformAssertions()` is a call that returns `Ok`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12172Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/ticket.rs"]
mod ticket;

use std::sync::Arc;

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::object::StdClass;
use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_persistence::mapping::driver::MappingDriverChain;

use orm::*;

/// `setUp()`.
async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[CMS_USER]).await
}

/// `new stdClass()`, or `(object) $properties`.
fn std_class(properties: &[(&str, Value)]) -> Object {
    let Value::Array(properties) = ticket::array(properties) else {
        unreachable!("array() builds an array")
    };
    let Value::Object(object) = StdClass::object(properties) else {
        unreachable!("an object")
    };
    object
}

/// `new GH12172TestClass()`.
fn test_class() -> Object {
    let object = ticket::new("GH12172TestClass");
    set(&object, "data", s(""));
    object
}

#[tokio::test]
async fn testInitializeObjectNoOpForNonManagedObjects() {
    let em = set_up().await;
    let non_entity = std_class(&[("someProperty", s("test"))]);

    em.initialize_object(&non_entity).await.unwrap();
}

#[tokio::test]
async fn testInitializeObjectNoOpForCustomNonEntityClass() {
    let em = set_up().await;
    let non_entity = test_class();
    set(&non_entity, "data", s("some data"));

    em.initialize_object(&non_entity).await.unwrap();
}

#[tokio::test]
async fn testIsUninitializedObjectNoExceptionForNonManagedObjects() {
    let em = set_up().await;
    let non_entity = std_class(&[]);

    let result = em.is_uninitialized_object(&Value::Object(non_entity));

    assert!(
        !result,
        "Non-managed objects should be considered initialized (not uninitialized)"
    );
}

#[tokio::test]
async fn testInitializeObjectWithValidEntityClassDoesNotBreakMetadataLoading() {
    let em = set_up().await;
    let user = cms_user_with("Test User", "testuser", "active");

    em.initialize_object(&user).await.unwrap();
}

#[tokio::test]
async fn testInitializeObjectWithDifferentTypesOfObjectsConsistentBehavior() {
    let em = set_up().await;
    let Value::Object(date_time) = DateTime::object(DateTimeImmutable::now(TimeZone::Utc)) else {
        unreachable!("an object")
    };
    let test_objects = [
        std_class(&[]),
        test_class(),
        date_time,
        std_class(&[("prop", s("value"))]),
    ];

    for object in &test_objects {
        em.initialize_object(object).await.unwrap();

        let result = em.is_uninitialized_object(&Value::Object(object.clone()));
        assert!(
            !result,
            "Non-entity objects should not be considered uninitialized"
        );
    }
}

/// `createMappingDriverChain()`.
fn create_mapping_driver_chain() -> Arc<MappingDriverChain> {
    let driver_chain = MappingDriverChain::new();
    driver_chain.add_driver(
        Arc::new(AttributeDriver::new(Vec::<String>::new(), true).unwrap()),
        "Doctrine\\Tests\\Models\\CMS",
    );

    Arc::new(driver_chain)
}

/// `$this->getEntityManager(null, $this->createMappingDriverChain())`.
fn entity_manager_with_driver_chain() -> Arc<EntityManager> {
    let driver_chain = create_mapping_driver_chain();
    orm::entity_manager_with(orm::connection(), move |config| {
        config.set_metadata_driver_impl(driver_chain);
    })
}

#[tokio::test]
async fn testInitializeObjectNoOpForNonManagedObjectsWithMappingDriverChain() {
    let _ = set_up().await;
    let em = entity_manager_with_driver_chain();

    em.initialize_object(&std_class(&[])).await.unwrap();
}

#[tokio::test]
async fn testIsUninitializedObjectNoExceptionForNonManagedObjectsWithMappingDriverChain() {
    let _ = set_up().await;
    let em = entity_manager_with_driver_chain();

    assert!(!em.is_uninitialized_object(&Value::Object(std_class(&[]))));
}

