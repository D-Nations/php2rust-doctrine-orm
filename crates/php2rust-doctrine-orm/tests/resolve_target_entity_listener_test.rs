//! Port of `ResolveTargetEntityListenerTest`. `addEventSubscriber()` and
//! `addEventListener(Events::loadClassMetadata, ...)` add the listener's
//! hooks to the entity manager's metadata factory. `assertSame()` of two
//! metadata compares the references.
//!
//! `testDoesResolveTargetEntitiesInDQLAlsoWithInterfaces` is pending: its
//! SQL loads the metadata of `Target`, which it does not resolve, and an
//! interface has none. Upstream passes because `OrmTestCase`'s metadata
//! cache is shared between the tests and still holds `ResolveTargetEntity`
//! with `Target` resolved by an earlier test; on its own it throws, as
//! the port does.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/ResolveTargetEntityListenerTest.php

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

use std::sync::Arc;

use php_value::PhpArray;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::tools::ResolveTargetEntityListener;
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;

const RESOLVE_TARGET: &str = "Doctrine\\Tests\\ORM\\Tools\\ResolveTarget";
const TARGET: &str = "Doctrine\\Tests\\ORM\\Tools\\Target";
const RESOLVE_TARGET_ENTITY: &str = "Doctrine\\Tests\\ORM\\Tools\\ResolveTargetEntity";
const TARGET_ENTITY: &str = "Doctrine\\Tests\\ORM\\Tools\\TargetEntity";

/// `setUp()`.
fn set_up() -> (Arc<EntityManager>, Arc<ResolveTargetEntityListener>) {
    models::register();
    let em = orm::entity_manager(orm::connection(), None);
    (em, Arc::new(ResolveTargetEntityListener::new()))
}

fn target_entity(em: &EntityManager, class: &str, field: &str) -> String {
    em.get_metadata_factory()
        .get_metadata_for(class)
        .unwrap()
        .read()
        .association_mappings
        .get(field)
        .unwrap()
        .target_entity
        .clone()
}

#[test]
fn testResolveTargetEntityListenerCanResolveTargetEntity() {
    let (em, listener) = set_up();
    listener.add_resolve_target_entity(RESOLVE_TARGET, RESOLVE_TARGET_ENTITY, PhpArray::new());
    listener.add_resolve_target_entity(TARGET, TARGET_ENTITY, PhpArray::new());
    listener.subscribe(&em).unwrap();

    let factory = em.get_metadata_factory();
    let cm = factory.get_metadata_for(RESOLVE_TARGET_ENTITY).unwrap();

    assert_eq!(
        TARGET_ENTITY,
        target_entity(&em, RESOLVE_TARGET_ENTITY, "manyToMany")
    );
    assert_eq!(
        RESOLVE_TARGET_ENTITY,
        target_entity(&em, RESOLVE_TARGET_ENTITY, "manyToOne")
    );
    assert_eq!(
        RESOLVE_TARGET_ENTITY,
        target_entity(&em, RESOLVE_TARGET_ENTITY, "oneToMany")
    );
    assert_eq!(
        TARGET_ENTITY,
        target_entity(&em, RESOLVE_TARGET_ENTITY, "oneToOne")
    );

    assert!(Arc::ptr_eq(
        &cm,
        &factory.get_metadata_for(RESOLVE_TARGET).unwrap()
    ));
}

#[test]
fn testResolveTargetEntityListenerCanRetrieveTargetEntityByInterfaceName() {
    let (em, listener) = set_up();
    listener.add_resolve_target_entity(RESOLVE_TARGET, RESOLVE_TARGET_ENTITY, PhpArray::new());

    listener.subscribe(&em).unwrap();

    let factory = em.get_metadata_factory();
    let cm = factory.get_metadata_for(RESOLVE_TARGET).unwrap();

    assert!(Arc::ptr_eq(
        &factory.get_metadata_for(RESOLVE_TARGET_ENTITY).unwrap(),
        &cm
    ));
}

#[test]
fn testAssertTableColumnsAreNotAddedInManyToMany() {
    let (em, listener) = set_up();
    listener.add_resolve_target_entity(RESOLVE_TARGET, RESOLVE_TARGET_ENTITY, PhpArray::new());
    listener.add_resolve_target_entity(TARGET, TARGET_ENTITY, PhpArray::new());

    listener.listen_to_load_class_metadata(&em).unwrap();
    let cm = em
        .get_metadata_factory()
        .get_metadata_for(RESOLVE_TARGET_ENTITY)
        .unwrap();
    let cm = cm.read();
    let meta = cm.association_mappings.get("manyToMany").unwrap();

    assert_eq!(TARGET_ENTITY, meta.target_entity);
    assert_eq!(
        vec!["resolvetargetentity_id".to_owned(), "target_id".to_owned()],
        meta.join_table_columns
    );
}
