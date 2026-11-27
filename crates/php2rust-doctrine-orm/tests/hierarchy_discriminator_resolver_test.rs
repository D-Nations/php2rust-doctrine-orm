//! Port of tests/Tests/ORM/Utility/HierarchyDiscriminatorResolverTest.php.
//! The mocked `EntityManagerInterface` is a `QueryContext` that answers
//! `getClassMetadata()` from its map and counts the calls
//! (`expects(self::exactly(n))`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Utility/HierarchyDiscriminatorResolverTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use php_value::{Key, Thrown, Value};
use php2rust_doctrine_dbal::platforms::{AbstractPlatform, SQLitePlatform};
use php2rust_doctrine_orm::mapping::{AssociationMapping, ClassMetadata, QuoteStrategy};
use php2rust_doctrine_orm::query::context::QueryContext;
use php2rust_doctrine_orm::utility::hierarchy_discriminator_resolver::resolve_discriminators_for_class;

/// `$this->createMock(EntityManagerInterface::class)`.
struct EntityManagerMock {
    map: Vec<(String, Arc<ClassMetadata>)>,
    calls: AtomicUsize,
    platform: SQLitePlatform,
}

impl EntityManagerMock {
    fn new(map: Vec<(String, Arc<ClassMetadata>)>) -> Self {
        Self {
            map,
            calls: AtomicUsize::new(0),
            platform: SQLitePlatform::new(),
        }
    }
}

impl QueryContext for EntityManagerMock {
    fn get_class_metadata(&self, class_name: &str) -> Result<Arc<ClassMetadata>, Thrown> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self
            .map
            .iter()
            .find(|(name, _)| name == class_name)
            .map(|(_, metadata)| metadata.clone())
            .expect("a mapped class"))
    }

    fn get_owning_side(&self, _assoc: &AssociationMapping) -> Result<AssociationMapping, Thrown> {
        unreachable!("not called")
    }

    fn get_database_platform(&self) -> &dyn AbstractPlatform {
        &self.platform
    }

    fn quote(&self, _value: &str) -> Result<String, Thrown> {
        unreachable!("not called")
    }

    fn get_quote_strategy(&self) -> Arc<dyn QuoteStrategy> {
        unreachable!("not called")
    }
}

fn metadata(
    entity: &str,
    name: &str,
    discriminator: &str,
    sub_classes: Vec<String>,
) -> Arc<ClassMetadata> {
    let class_metadata = ClassMetadata::new(entity, None, None);
    {
        let mut info = class_metadata.write();
        info.sub_classes = sub_classes;
        name.clone_into(&mut info.name);
        info.discriminator_value = Value::string(discriminator);
    }
    Arc::new(class_metadata)
}

#[test]
fn testResolveDiscriminatorsForClass() {
    let child_class_metadata = metadata(
        "ChildEntity",
        "Some\\Class\\Child\\Name",
        "child-discriminator",
        Vec::new(),
    );

    let class_metadata = metadata(
        "Entity",
        "Some\\Class\\Name",
        "discriminator",
        vec![child_class_metadata.read().name.clone()],
    );

    let em = EntityManagerMock::new(vec![
        (class_metadata.read().name.clone(), class_metadata.clone()),
        (
            child_class_metadata.read().name.clone(),
            child_class_metadata.clone(),
        ),
    ]);

    let discriminators = resolve_discriminators_for_class(&class_metadata, &em).unwrap();

    assert_eq!(em.calls.load(Ordering::SeqCst), 2);
    assert_eq!(discriminators.len(), 2);
    assert!(discriminators.contains_key(&Key::from("discriminator")));
    assert!(discriminators.contains_key(&Key::from("child-discriminator")));
}

#[test]
fn testResolveDiscriminatorsForClassWithNoSubclasses() {
    let class_metadata = metadata("Entity", "Some\\Class\\Name", "discriminator", Vec::new());

    let em = EntityManagerMock::new(vec![(
        class_metadata.read().name.clone(),
        class_metadata.clone(),
    )]);

    let discriminators = resolve_discriminators_for_class(&class_metadata, &em).unwrap();

    assert_eq!(em.calls.load(Ordering::SeqCst), 1);
    assert_eq!(discriminators.len(), 1);
    assert!(discriminators.contains_key(&Key::from("discriminator")));
}
