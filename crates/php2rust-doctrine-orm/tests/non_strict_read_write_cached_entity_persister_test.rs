//! Upstream: doctrine/orm tests/Tests/ORM/Cache/Persister/Entity/NonStrictReadWriteCachedEntityPersisterTest.php
//!
//! `EntityPersisterTestCase`'s tests come from
//! `common/entity_persister_test_case.rs`. `count($queuedCache)` (read by
//! reflection upstream) is `queued_cache_len()`.

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

use php_value::Value;
use php2rust_doctrine_orm::cache::{AbstractEntityPersister, CacheUsage, Region};
use php2rust_doctrine_orm::cache::{
    CacheEntry, CacheKey, CachedPersister, EntityCacheEntry, EntityCacheKey,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::ClassMetadata;
use php2rust_doctrine_orm::persisters::{CommitPersister, EntityPersisterInterface};

use slc::{Arg, RegionMock, Ret};

fn create_persister(
    em: &Arc<EntityManager>,
    persister: Arc<dyn EntityPersisterInterface>,
    region: Arc<dyn Region>,
    metadata: Arc<ClassMetadata>,
) -> AbstractEntityPersister {
    AbstractEntityPersister::new(
        CacheUsage::NonStrictReadWrite,
        persister,
        region,
        em,
        metadata,
    )
    .unwrap()
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
        testInvokeUpdate,
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

fn key() -> CacheKey {
    CacheKey::Entity(EntityCacheKey::new(slc::COUNTRY, &slc::id(1)))
}

fn entry() -> CacheEntry {
    CacheEntry::Entity(EntityCacheEntry::new(
        slc::COUNTRY,
        slc::array(&[("id", Value::Int(1)), ("name", Value::string("Foo"))]),
    ))
}

#[tokio::test]
async fn testTransactionRollBackShouldClearQueue() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();

    f.register_managed(&entity);

    CommitPersister::update(&persister, &entity).await.unwrap();
    CommitPersister::delete(&persister, &entity).await.unwrap();

    assert_eq!(2, persister.queued_cache_len());

    persister.after_transaction_rolled_back().unwrap();

    assert_eq!(0, persister.queued_cache_len());
}

#[tokio::test]
async fn testInsertTransactionCommitShouldPutCache() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();

    f.entity_persister
        .recorder
        .will_return("getInserts", Ret::Objects(vec![entity.clone()]));

    f.register_managed(&entity);

    persister.add_insert(&entity);
    persister.execute_inserts().await.unwrap();

    assert_eq!(1, persister.queued_cache_len());

    persister.after_transaction_complete().unwrap();

    assert_eq!(0, persister.queued_cache_len());

    f.region.recorder.assert_once_with(
        "put",
        &[RegionMock::key(&key()), RegionMock::entry(&entry())],
    );
    f.entity_persister
        .recorder
        .assert_once_with("addInsert", &[Arg::object(&entity)]);
    f.entity_persister.recorder.assert_count("getInserts", 1);
    f.entity_persister
        .recorder
        .assert_count("executeInserts", 1);
}

#[tokio::test]
async fn testUpdateTransactionCommitShouldPutCache() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();

    f.register_managed(&entity);

    CommitPersister::update(&persister, &entity).await.unwrap();

    assert_eq!(1, persister.queued_cache_len());

    persister.after_transaction_complete().unwrap();

    assert_eq!(0, persister.queued_cache_len());

    f.region.recorder.assert_once_with(
        "put",
        &[RegionMock::key(&key()), RegionMock::entry(&entry())],
    );
    f.entity_persister
        .recorder
        .assert_once_with("update", &[Arg::object(&entity)]);
}

#[tokio::test]
async fn testDeleteTransactionCommitShouldEvictCache() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();

    f.register_managed(&entity);

    CommitPersister::delete(&persister, &entity).await.unwrap();

    assert_eq!(1, persister.queued_cache_len());

    persister.after_transaction_complete().unwrap();

    assert_eq!(0, persister.queued_cache_len());

    f.region
        .recorder
        .assert_once_with("evict", &[RegionMock::key(&key())]);
    f.entity_persister
        .recorder
        .assert_once_with("delete", &[Arg::object(&entity)]);
}
