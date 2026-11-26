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

#[test]
fn testImplementsEntityPersister() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testImplementsEntityPersister(&f);
}

#[test]
fn testInvokeAddInsert() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeAddInsert(&f);
}

#[test]
fn testInvokeGetInserts() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeGetInserts(&f);
}

#[test]
fn testInvokeGetSelectSQL() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeGetSelectSQL(&f);
}

#[test]
fn testInvokeGetInsertSQL() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeGetInsertSQL(&f);
}

#[test]
fn testInvokeExpandParameters() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeExpandParameters(&f);
}

#[test]
fn testInvokeExpandCriteriaParameters() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeExpandCriteriaParameters(&f);
}

#[test]
fn testInvokeSelectConditionStatementSQL() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeSelectConditionStatementSQL(&f);
}

#[test]
fn testInvokeGetOwningTable() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeGetOwningTable(&f);
}

#[tokio::test]
async fn testInvokeExecuteInserts() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeExecuteInserts(&f).await;
}

#[tokio::test]
async fn testInvokeDelete() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeDelete(&f).await;
}

#[tokio::test]
async fn testInvokeLoad() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeLoad(&f).await;
}

#[tokio::test]
async fn testInvokeLoadAll() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeLoadAll(&f).await;
}

#[tokio::test]
async fn testInvokeLoadById() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeLoadById(&f).await;
}

#[tokio::test]
async fn testInvokeLoadOneToOneEntity() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeLoadOneToOneEntity(&f).await;
}

#[tokio::test]
async fn testInvokeRefresh() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeRefresh(&f).await;
}

#[tokio::test]
async fn testInvokeLoadCriteria() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeLoadCriteria(&f).await;
}

#[tokio::test]
async fn testInvokeGetManyToManyCollection() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeGetManyToManyCollection(&f).await;
}

#[tokio::test]
async fn testInvokeGetOneToManyCollection() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeGetOneToManyCollection(&f).await;
}

#[tokio::test]
async fn testInvokeLoadManyToManyCollection() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeLoadManyToManyCollection(&f).await;
}

#[tokio::test]
async fn testInvokeLoadOneToManyCollection() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeLoadOneToManyCollection(&f).await;
}

#[tokio::test]
async fn testInvokeLock() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeLock(&f).await;
}

#[tokio::test]
async fn testInvokeExists() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    entity_persister_test_case::testInvokeExists(&f).await;
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
