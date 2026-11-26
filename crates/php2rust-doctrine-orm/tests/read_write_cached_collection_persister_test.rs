//! Upstream: doctrine/orm tests/Tests/ORM/Cache/Persister/Collection/ReadWriteCachedCollectionPersisterTest.php
//!
//! `CollectionPersisterTestCase`'s tests come from
//! `common/collection_persister_test_case.rs`. `count($queuedCache)` (read
//! by reflection upstream) is `queued_cache_len()`.

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

use php2rust_doctrine_orm::cache::{
    AbstractCollectionPersister, CacheKey, CacheUsage, CachedPersister, CollectionCacheKey, Lock,
    Region,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::AssociationMapping;
use php2rust_doctrine_orm::persisters::collection::CollectionPersister;

use collection_persister_test_case::Fixture;
use slc::{CollectionPersisterMock, RegionMock, Ret};

fn create_persister(
    em: &Arc<EntityManager>,
    persister: Arc<dyn CollectionPersister>,
    region: Arc<dyn Region>,
    mapping: AssociationMapping,
) -> AbstractCollectionPersister {
    AbstractCollectionPersister::new(CacheUsage::ReadWrite, persister, region, em, mapping).unwrap()
}

fn create_region() -> Arc<RegionMock> {
    RegionMock::concurrent()
}

fn fixture() -> Fixture {
    Fixture::new(create_persister, create_region())
}

/// `new CollectionCacheKey(State::class, 'cities', ['id' => 1])`.
fn key() -> CacheKey {
    CacheKey::Collection(CollectionCacheKey::new(slc::STATE, "cities", &slc::id(1)))
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

#[tokio::test]
async fn testDeleteShouldLockItem() {
    let f = fixture();
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);
    f.region
        .recorder
        .will_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.register_managed(&entity);

    CollectionPersister::delete(&persister, &collection)
        .await
        .unwrap();

    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
}

#[tokio::test]
async fn testUpdateShouldLockItem() {
    let f = fixture();
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);
    f.region
        .recorder
        .will_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.register_managed(&entity);

    CollectionPersister::update(&persister, &collection)
        .await
        .unwrap();

    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
}

#[tokio::test]
async fn testUpdateTransactionRollBackShouldEvictItem() {
    let f = fixture();
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);
    f.region
        .recorder
        .will_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.region.recorder.will_return("evict", Ret::Bool(true));

    f.register_managed(&entity);

    CollectionPersister::update(&persister, &collection)
        .await
        .unwrap();
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
    let f = fixture();
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);
    f.region
        .recorder
        .will_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.region.recorder.will_return("evict", Ret::Bool(true));

    f.register_managed(&entity);

    CollectionPersister::delete(&persister, &collection)
        .await
        .unwrap();
    persister.after_transaction_rolled_back().unwrap();

    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
    f.region
        .recorder
        .assert_once_with("evict", &[RegionMock::key(&key())]);
}

#[tokio::test]
async fn testTransactionRollBackDeleteShouldClearQueue() {
    let f = fixture();
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);
    f.region
        .recorder
        .will_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.region.recorder.will_return("evict", Ret::Bool(true));

    f.register_managed(&entity);

    CollectionPersister::delete(&persister, &collection)
        .await
        .unwrap();

    assert_eq!(1, persister.queued_cache_len());

    persister.after_transaction_rolled_back().unwrap();

    assert_eq!(0, persister.queued_cache_len());
    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
    f.region
        .recorder
        .assert_once_with("evict", &[RegionMock::key(&key())]);
}

#[tokio::test]
async fn testTransactionRollBackUpdateShouldClearQueue() {
    let f = fixture();
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);
    f.region
        .recorder
        .will_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.region.recorder.will_return("evict", Ret::Bool(true));

    f.register_managed(&entity);

    CollectionPersister::update(&persister, &collection)
        .await
        .unwrap();

    assert_eq!(1, persister.queued_cache_len());

    persister.after_transaction_rolled_back().unwrap();

    assert_eq!(0, persister.queued_cache_len());
    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
    f.region
        .recorder
        .assert_once_with("evict", &[RegionMock::key(&key())]);
}

#[tokio::test]
async fn testTransactionRollCommitDeleteShouldClearQueue() {
    let f = fixture();
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);
    f.region
        .recorder
        .will_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.region.recorder.will_return("evict", Ret::Bool(true));

    f.register_managed(&entity);

    CollectionPersister::delete(&persister, &collection)
        .await
        .unwrap();

    assert_eq!(1, persister.queued_cache_len());

    persister.after_transaction_complete().unwrap();

    assert_eq!(0, persister.queued_cache_len());
    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
    f.region
        .recorder
        .assert_once_with("evict", &[RegionMock::key(&key())]);
}

#[tokio::test]
async fn testTransactionRollCommitUpdateShouldClearQueue() {
    let f = fixture();
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);
    f.region
        .recorder
        .will_return("lock", Ret::Lock(Some(Lock::create_lock_read())));

    f.region.recorder.will_return("evict", Ret::Bool(true));

    f.register_managed(&entity);

    CollectionPersister::update(&persister, &collection)
        .await
        .unwrap();

    assert_eq!(1, persister.queued_cache_len());

    persister.after_transaction_complete().unwrap();

    assert_eq!(0, persister.queued_cache_len());
    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
    f.region
        .recorder
        .assert_once_with("evict", &[RegionMock::key(&key())]);
}

#[tokio::test]
async fn testDeleteLockFailureShouldIgnoreQueue() {
    let f = fixture();
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);
    f.region.recorder.will_return("lock", Ret::Lock(None));

    f.register_managed(&entity);

    CollectionPersister::delete(&persister, &collection)
        .await
        .unwrap();
    assert_eq!(0, persister.queued_cache_len());
    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
    f.collection_persister.recorder.assert_once_with(
        "delete",
        &[CollectionPersisterMock::collection(&collection)],
    );
}

#[tokio::test]
async fn testUpdateLockFailureShouldIgnoreQueue() {
    let f = fixture();
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);
    f.region.recorder.will_return("lock", Ret::Lock(None));

    f.register_managed(&entity);

    CollectionPersister::update(&persister, &collection)
        .await
        .unwrap();
    assert_eq!(0, persister.queued_cache_len());
    f.region
        .recorder
        .assert_once_with("lock", &[RegionMock::key(&key())]);
    f.collection_persister.recorder.assert_once_with(
        "update",
        &[CollectionPersisterMock::collection(&collection)],
    );
}
