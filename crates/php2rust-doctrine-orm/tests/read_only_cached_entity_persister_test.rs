//! Upstream: doctrine/orm tests/Tests/ORM/Cache/Persister/Entity/ReadOnlyCachedEntityPersisterTest.php
//!
//! `EntityPersisterTestCase`'s tests come from
//! `common/entity_persister_test_case.rs`; this file's own
//! `testInvokeUpdate()` replaces the shared one.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/entity_persister_test_case.rs"]
mod entity_persister_test_case;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/slc.rs"]
mod slc;

use std::sync::Arc;

use php2rust_doctrine_orm::cache::{AbstractEntityPersister, CacheUsage, Region};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::ClassMetadata;
use php2rust_doctrine_orm::persisters::{CommitPersister, EntityPersisterInterface};

use slc::RegionMock;

fn create_persister(
    em: &Arc<EntityManager>,
    persister: Arc<dyn EntityPersisterInterface>,
    region: Arc<dyn Region>,
    metadata: Arc<ClassMetadata>,
) -> AbstractEntityPersister {
    AbstractEntityPersister::new(CacheUsage::ReadOnly, persister, region, em, metadata).unwrap()
}

fn create_region() -> Arc<RegionMock> {
    RegionMock::new()
}

entity_persister_test_case! {
    sync: [
        testImplementsEntityPersister,
        testInvokeAddInsert,
        testInvokeGetInserts,
        testInvokeGetSelectSQL,
        testInvokeGetInsertSQL,
        testInvokeExpandParameters,
        testInvokeExpandCriteriaParameters,
        testInvokeSelectConditionStatementSQL,
        testInvokeGetOwningTable,
    ],
    async: [
        testInvokeExecuteInserts,
        testInvokeDelete,
        testInvokeLoad,
        testInvokeLoadAll,
        testInvokeLoadById,
        testInvokeLoadOneToOneEntity,
        testInvokeRefresh,
        testInvokeLoadCriteria,
        testInvokeGetManyToManyCollection,
        testInvokeGetOneToManyCollection,
        testInvokeLoadManyToManyCollection,
        testInvokeLoadOneToManyCollection,
        testInvokeLock,
        testInvokeExists,
    ],
}

#[tokio::test]
async fn testInvokeUpdate() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");

    let thrown = CommitPersister::update(&persister, &entity)
        .await
        .unwrap_err();
    assert_eq!(
        "Doctrine\\ORM\\Cache\\Exception\\CannotUpdateReadOnlyEntity",
        thrown.class
    );
    assert_eq!(
        "Cannot update a readonly entity \"Doctrine\\Tests\\Models\\Cache\\Country\"",
        thrown.message.to_string()
    );
}
