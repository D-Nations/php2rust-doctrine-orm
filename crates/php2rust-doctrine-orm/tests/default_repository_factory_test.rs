//! Port of `DefaultRepositoryFactoryTest`. The factory takes the entity
//! manager itself, so the stubs are entity managers: each one's
//! configuration has the default repository class, and its metadata
//! factory holds the stubbed metadata of the test class
//! (`setMetadataFor()`), which `getClassMetadata()` then returns.
//! `assertInstanceOf()` of a repository is its repository class;
//! `assertSame()` compares the references.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Repository/DefaultRepositoryFactoryTest.php

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

use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::{ClassMetadata, ClassMetadataInfo};
use php2rust_doctrine_orm::repository::{DefaultRepositoryFactory, RepositoryFactory};
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;

/// `self::class`.
const SELF: &str = "Doctrine\\Tests\\ORM\\Repository\\DefaultRepositoryFactoryTest";
const DDC869_PAYMENT_REPOSITORY: &str = "Doctrine\\Tests\\Models\\DDC869\\DDC869PaymentRepository";
const DDC753_ENTITY_WITH_DEFAULT_CUSTOM_REPOSITORY: &str =
    "Doctrine\\Tests\\Models\\DDC753\\DDC753EntityWithDefaultCustomRepository";
const DDC753_DEFAULT_REPOSITORY: &str = "Doctrine\\Tests\\Models\\DDC753\\DDC753DefaultRepository";

/// `buildClassMetadata()`.
fn build_class_metadata(class_name: &str) -> ClassMetadataInfo {
    let mut metadata = ClassMetadataInfo::new(class_name, None, None);
    metadata.name = class_name.to_owned();

    metadata.custom_repository_class_name = None;

    metadata
}

/// `createEntityManager()`, whose `getClassMetadata()` returns
/// `$metadata` for the test class.
fn create_entity_manager(metadata: ClassMetadataInfo) -> Arc<EntityManager> {
    let em = orm::entity_manager_with(orm::connection(), |config| {
        config
            .set_default_repository_class_name(DDC869_PAYMENT_REPOSITORY)
            .unwrap();
    });
    em.get_metadata_factory()
        .set_metadata_for(SELF, Arc::new(ClassMetadata::from_info(metadata)));
    em
}

#[test]
fn testCreatesRepositoryFromDefaultRepositoryClass() {
    let entity_manager = create_entity_manager(build_class_metadata(SELF));
    let repository_factory = DefaultRepositoryFactory::new();

    assert_eq!(
        DDC869_PAYMENT_REPOSITORY,
        repository_factory
            .get_repository(&entity_manager, SELF)
            .unwrap()
            .get_repository_class()
    );
}

#[test]
fn testCreatedRepositoriesAreCached() {
    let entity_manager = create_entity_manager(build_class_metadata(SELF));
    let repository_factory = DefaultRepositoryFactory::new();

    assert!(Arc::ptr_eq(
        &repository_factory
            .get_repository(&entity_manager, SELF)
            .unwrap(),
        &repository_factory
            .get_repository(&entity_manager, SELF)
            .unwrap()
    ));
}

#[test]
fn testCreatesRepositoryFromCustomClassMetadata() {
    let mut custom_metadata = build_class_metadata(DDC753_ENTITY_WITH_DEFAULT_CUSTOM_REPOSITORY);
    custom_metadata.custom_repository_class_name = Some(DDC753_DEFAULT_REPOSITORY.to_owned());

    let entity_manager = create_entity_manager(custom_metadata);
    let repository_factory = DefaultRepositoryFactory::new();

    assert_eq!(
        DDC753_DEFAULT_REPOSITORY,
        repository_factory
            .get_repository(&entity_manager, SELF)
            .unwrap()
            .get_repository_class()
    );
}

#[test]
fn testCachesDistinctRepositoriesPerDistinctEntityManager() {
    let em1 = create_entity_manager(build_class_metadata(SELF));
    let em2 = create_entity_manager(build_class_metadata(SELF));
    let repository_factory = DefaultRepositoryFactory::new();

    let repo1 = repository_factory.get_repository(&em1, SELF).unwrap();
    let repo2 = repository_factory.get_repository(&em2, SELF).unwrap();

    assert!(Arc::ptr_eq(
        &repo1,
        &repository_factory.get_repository(&em1, SELF).unwrap()
    ));
    assert!(Arc::ptr_eq(
        &repo2,
        &repository_factory.get_repository(&em2, SELF).unwrap()
    ));

    assert!(!Arc::ptr_eq(&repo1, &repo2));
}
