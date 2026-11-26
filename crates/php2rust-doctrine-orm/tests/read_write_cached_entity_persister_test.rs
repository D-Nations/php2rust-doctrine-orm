//! Upstream: doctrine/orm tests/Tests/ORM/Cache/Persister/Entity/ReadWriteCachedEntityPersisterTest.php
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

use php2rust_doctrine_orm::cache::{AbstractEntityPersister, CacheUsage, Region};
use php2rust_doctrine_orm::cache::{CacheKey, CachedPersister, EntityCacheKey, Lock};
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
    AbstractEntityPersister::new(CacheUsage::ReadWrite, persister, region, em, metadata).unwrap()
}

fn create_region() -> Arc<RegionMock> {
    RegionMock::concurrent()
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

#[tokio::test]
async fn testDeleteShouldLockItem() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();
    f.region
        .recorder
        .always_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.register_managed(&entity);

    CommitPersister::delete(&persister, &entity).await.unwrap();

    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
}

#[tokio::test]
async fn testUpdateShouldLockItem() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();
    f.region
        .recorder
        .always_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.register_managed(&entity);

    CommitPersister::update(&persister, &entity).await.unwrap();

    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
}

#[tokio::test]
async fn testUpdateTransactionRollBackShouldEvictItem() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();
    f.region
        .recorder
        .always_return("lock", Ret::Lock(Some(Lock::create_lock_read())));
    f.region.recorder.will_return("evict", Ret::Bool(true));

    f.register_managed(&entity);

    CommitPersister::update(&persister, &entity).await.unwrap();
    persister.after_transaction_rolled_back().unwrap();

    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
    f.region
        .recorder
        .assert_once_with("evict", &[RegionMock::key(&key())]);
}

#[tokio::test]
async fn testDeleteTransactionRollBackShouldEvictItem() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();
    f.region
        .recorder
        .always_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.register_managed(&entity);

    CommitPersister::delete(&persister, &entity).await.unwrap();
    persister.after_transaction_rolled_back().unwrap();

    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
    f.region
        .recorder
        .assert_once_with("evict", &[RegionMock::key(&key())]);
}

#[tokio::test]
async fn testTransactionRollBackShouldClearQueue() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();
    f.region
        .recorder
        .always_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.register_managed(&entity);

    CommitPersister::update(&persister, &entity).await.unwrap();
    CommitPersister::delete(&persister, &entity).await.unwrap();

    assert_eq!(2, persister.queued_cache_len());

    persister.after_transaction_rolled_back().unwrap();

    assert_eq!(0, persister.queued_cache_len());

    assert_eq!(2, f.region.recorder.calls("lock").len());
    for args in f.region.recorder.calls("lock") {
        assert_eq!(vec![RegionMock::key(&key())], args);
    }
    assert_eq!(2, f.region.recorder.calls("evict").len());
    for args in f.region.recorder.calls("evict") {
        assert_eq!(vec![RegionMock::key(&key())], args);
    }
}

#[tokio::test]
async fn testTransactionCommitShouldClearQueue() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();
    f.region
        .recorder
        .always_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.register_managed(&entity);

    CommitPersister::update(&persister, &entity).await.unwrap();
    CommitPersister::delete(&persister, &entity).await.unwrap();

    assert_eq!(2, persister.queued_cache_len());

    persister.after_transaction_complete().unwrap();

    assert_eq!(0, persister.queued_cache_len());

    assert_eq!(2, f.region.recorder.calls("lock").len());
    for args in f.region.recorder.calls("lock") {
        assert_eq!(vec![RegionMock::key(&key())], args);
    }
    assert_eq!(2, f.region.recorder.calls("evict").len());
    for args in f.region.recorder.calls("evict") {
        assert_eq!(vec![RegionMock::key(&key())], args);
    }
}

#[tokio::test]
async fn testDeleteLockFailureShouldIgnoreQueue() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();
    f.region.recorder.will_return("lock", Ret::Lock(None));

    f.register_managed(&entity);

    CommitPersister::delete(&persister, &entity).await.unwrap();
    assert_eq!(0, persister.queued_cache_len());

    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
    f.entity_persister
        .recorder
        .assert_once_with("delete", &[Arg::object(&entity)]);
}

#[tokio::test]
async fn testUpdateLockFailureShouldIgnoreQueue() {
    let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();
    f.region.recorder.will_return("lock", Ret::Lock(None));

    f.register_managed(&entity);

    CommitPersister::update(&persister, &entity).await.unwrap();
    assert_eq!(0, persister.queued_cache_len());

    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
    f.entity_persister
        .recorder
        .assert_once_with("update", &[Arg::object(&entity)]);
}
