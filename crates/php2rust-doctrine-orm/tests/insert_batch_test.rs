//! Port of `InsertBatchTest`. The entity manager stub's
//! `getClassMetadata()` is the function `batchByEntityType()` takes. The
//! three empty entity classes are declared here.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Internal/UnitOfWork/InsertBatchTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::{Arc, Once};

use php_value::Object;
use php2rust_doctrine_orm::id::IdGenerator;
use php2rust_doctrine_orm::internal::unit_of_work::InsertBatch;
use php2rust_doctrine_orm::mapping::{ClassMetadata, ClassMetadataInfo};
use php2rust_doctrine_orm::object::EntityObject;
use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};

const ENTITY_A: &str = "Doctrine\\Tests\\ORM\\Internal\\UnitOfWork\\EntityA";
const ENTITY_B: &str = "Doctrine\\Tests\\ORM\\Internal\\UnitOfWork\\EntityB";
const ENTITY_C: &str = "Doctrine\\Tests\\ORM\\Internal\\UnitOfWork\\EntityC";

/// `class EntityA {}`, `class EntityB {}`, `class EntityC {}`.
fn declare() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        for class in [ENTITY_A, ENTITY_B, ENTITY_C] {
            register_class(
                ClassInfo::class(class)
                    .new_instance(Arc::new(move || Ok(EntityObject::object(class).unwrap()))),
            );
        }
    });
}

fn new(class: &str) -> Object {
    EntityObject::object(class).unwrap()
}

struct Stub {
    metadata: Vec<Arc<ClassMetadata>>,
}

/// `setUp()`.
fn set_up() -> Stub {
    declare();
    let metadata = |class: &str, generator: IdGenerator| {
        let mut info = ClassMetadataInfo::new(class, None, None);
        info.set_id_generator(generator);
        Arc::new(ClassMetadata::from_info(info))
    };
    Stub {
        metadata: vec![
            metadata(ENTITY_A, IdGenerator::Assigned),
            metadata(ENTITY_B, IdGenerator::Assigned),
            metadata(ENTITY_C, IdGenerator::Identity),
        ],
    }
}

impl Stub {
    /// `$this->entityManager->getClassMetadata($class)`.
    fn get_class_metadata(&self, class: &str) -> Arc<ClassMetadata> {
        Arc::clone(
            self.metadata
                .iter()
                .find(|metadata| metadata.read().name == class)
                .unwrap(),
        )
    }

    fn batch(&self, entities: Vec<Object>) -> Vec<InsertBatch> {
        InsertBatch::batch_by_entity_type(|class| Ok(self.get_class_metadata(class)), entities)
            .unwrap()
    }
}

fn assert_batch(batch: &InsertBatch, class: &str, count: usize) {
    assert_eq!(class, batch.class.read().name);
    assert_eq!(count, batch.entities.len());
}

#[test]
fn testWillProduceEmptyBatchOnNoGivenEntities() {
    let entity_manager = set_up();
    assert!(entity_manager.batch(vec![]).is_empty());
}

#[test]
fn testWillBatchSameEntityOperationsInSingleBatch() {
    let entity_manager = set_up();
    let batches = entity_manager.batch(vec![new(ENTITY_A), new(ENTITY_A), new(ENTITY_A)]);

    assert_eq!(1, batches.len());
    assert_batch(&batches[0], ENTITY_A, 3);
}

#[test]
fn testWillBatchInterleavedEntityOperationsInGroups() {
    let entity_manager = set_up();
    let batches = entity_manager.batch(vec![
        new(ENTITY_A),
        new(ENTITY_A),
        new(ENTITY_B),
        new(ENTITY_B),
        new(ENTITY_A),
        new(ENTITY_A),
    ]);

    assert_eq!(3, batches.len());
    assert_batch(&batches[0], ENTITY_A, 2);
    assert_batch(&batches[1], ENTITY_B, 2);
    assert_batch(&batches[2], ENTITY_A, 2);
}

#[test]
fn testWillNotBatchOperationsForAGeneratedIdentifierEntity() {
    let entity_manager = set_up();
    let batches = entity_manager.batch(vec![new(ENTITY_C), new(ENTITY_C), new(ENTITY_C)]);

    assert_eq!(3, batches.len());
    assert_batch(&batches[0], ENTITY_C, 1);
    assert_batch(&batches[1], ENTITY_C, 1);
    assert_batch(&batches[2], ENTITY_C, 1);
}

#[test]
fn testWillIsolateBatchesForEntitiesWithGeneratedIdentifiers() {
    let entity_manager = set_up();
    let batches = entity_manager.batch(vec![
        new(ENTITY_A),
        new(ENTITY_A),
        new(ENTITY_C),
        new(ENTITY_C),
        new(ENTITY_A),
        new(ENTITY_A),
    ]);

    assert_eq!(4, batches.len());
    assert_batch(&batches[0], ENTITY_A, 2);
    assert_batch(&batches[1], ENTITY_C, 1);
    assert_batch(&batches[2], ENTITY_C, 1);
    assert_batch(&batches[3], ENTITY_A, 2);
}
