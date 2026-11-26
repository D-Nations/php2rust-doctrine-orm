//! Upstream: doctrine/orm tests/Tests/ORM/Cache/Persister/Collection/ReadOnlyCachedCollectionPersisterTest.php
//!
//! `CollectionPersisterTestCase`'s tests come from
//! `common/collection_persister_test_case.rs`.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/collection_persister_test_case.rs"]
mod collection_persister_test_case;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/slc.rs"]
mod slc;

use std::sync::Arc;

use php2rust_doctrine_orm::cache::{AbstractCollectionPersister, CacheUsage, Region};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::AssociationMapping;
use php2rust_doctrine_orm::persisters::collection::CollectionPersister;

use slc::RegionMock;

fn create_persister(
    em: &Arc<EntityManager>,
    persister: Arc<dyn CollectionPersister>,
    region: Arc<dyn Region>,
    mapping: AssociationMapping,
) -> AbstractCollectionPersister {
    AbstractCollectionPersister::new(CacheUsage::ReadOnly, persister, region, em, mapping).unwrap()
}

fn create_region() -> Arc<RegionMock> {
    RegionMock::new()
}

#[test]
fn testImplementsEntityPersister() {
    let f = collection_persister_test_case::Fixture::new(create_persister, create_region());
    collection_persister_test_case::testImplementsEntityPersister(&f);
}

#[tokio::test]
async fn testInvokeDelete() {
    let f = collection_persister_test_case::Fixture::new(create_persister, create_region());
    collection_persister_test_case::testInvokeDelete(&f).await;
}

#[tokio::test]
async fn testInvokeUpdate() {
    let f = collection_persister_test_case::Fixture::new(create_persister, create_region());
    collection_persister_test_case::testInvokeUpdate(&f).await;
}

#[tokio::test]
async fn testInvokeCount() {
    let f = collection_persister_test_case::Fixture::new(create_persister, create_region());
    collection_persister_test_case::testInvokeCount(&f).await;
}

#[tokio::test]
async fn testInvokeSlice() {
    let f = collection_persister_test_case::Fixture::new(create_persister, create_region());
    collection_persister_test_case::testInvokeSlice(&f).await;
}

#[tokio::test]
async fn testInvokeContains() {
    let f = collection_persister_test_case::Fixture::new(create_persister, create_region());
    collection_persister_test_case::testInvokeContains(&f).await;
}

#[tokio::test]
async fn testInvokeContainsKey() {
    let f = collection_persister_test_case::Fixture::new(create_persister, create_region());
    collection_persister_test_case::testInvokeContainsKey(&f).await;
}

#[tokio::test]
async fn testInvokeGet() {
    let f = collection_persister_test_case::Fixture::new(create_persister, create_region());
    collection_persister_test_case::testInvokeGet(&f).await;
}
