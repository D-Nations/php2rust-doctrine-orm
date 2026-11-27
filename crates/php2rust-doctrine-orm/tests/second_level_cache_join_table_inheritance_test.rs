//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheJoinTableInheritanceTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/query_log.rs"]
mod query_log;
#[path = "common/slc.rs"]
mod slc;
#[path = "common/slc_functional.rs"]
mod slc_functional;

use std::sync::Arc;

use php_value::Object;

use slc_functional::{
    ATTRACTION, ATTRACTION_CONTACT_INFO, ATTRACTION_INFO, ATTRACTION_LOCATION_INFO, Fixture,
    assert_instance_of, count, entities, get_at, id, set_up,
};

const PERSISTENT_COLLECTION: &str = "Doctrine\\ORM\\PersistentCollection";

/// `loadFixturesCountries()` … `loadFixturesAttractionsInfo()`.
async fn load_fixtures(f: &mut Fixture) {
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.load_fixtures_attractions().await;
    f.load_fixtures_attractions_info().await;
}

/// `$this->_em->createQuery($dql)->setCacheable(true)->getResult()`.
async fn cacheable_result(f: &Fixture, dql: &str) -> Vec<Object> {
    let query = f.em.create_query(dql);
    query.set_cacheable(true);
    entities(&query.get_object_result().await.unwrap())
}

/// `$info->getFone()`.
fn fone(info: &Object) -> php_value::Value {
    orm::get(info, "fone")
}

#[tokio::test]
async fn testUseSameRegion() {
    let f = set_up().await;
    let region_name = |class: &str| {
        f.cache
            .get_entity_cache_region(class)
            .unwrap()
            .unwrap()
            .get_name()
    };
    let info_region = region_name(ATTRACTION_INFO);

    assert_eq!(info_region, region_name(ATTRACTION_CONTACT_INFO));
    assert_eq!(info_region, region_name(ATTRACTION_LOCATION_INFO));
}

#[tokio::test]
async fn testPutOnPersistJoinTableInheritance() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;

    f.em.clear().unwrap();

    for info in &f.attractions_info[..4] {
        assert!(f.contains_entity(ATTRACTION_INFO, info));
    }
}

#[tokio::test]
async fn testJoinTableCountaisRootClass() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;

    f.em.clear().unwrap();

    for info in &f.attractions_info {
        assert!(f.contains_entity(ATTRACTION_INFO, info));
        assert!(f.contains_entity(info.class(), info));
    }
}

#[tokio::test]
async fn testPutAndLoadJoinTableEntities() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;

    f.em.clear().unwrap();

    f.cache.evict_entity_region(ATTRACTION_INFO).unwrap();

    let entity_id1 = id(&f.attractions_info[0]);
    let entity_id2 = id(&f.attractions_info[1]);

    assert!(!f.contains_entity(ATTRACTION_INFO, &f.attractions_info[0]));
    assert!(!f.contains_entity(ATTRACTION_INFO, &f.attractions_info[1]));
    assert!(!f.contains_entity(ATTRACTION_CONTACT_INFO, &f.attractions_info[0]));
    assert!(!f.contains_entity(ATTRACTION_CONTACT_INFO, &f.attractions_info[1]));

    f.reset_query_log();
    let entity1 = f.find(ATTRACTION_INFO, entity_id1.clone()).await.unwrap();
    let entity2 = f.find(ATTRACTION_INFO, entity_id2.clone()).await.unwrap();

    //load entity and relation whit sub classes
    f.assert_query_count(4);

    assert!(f.contains_entity(ATTRACTION_INFO, &f.attractions_info[0]));
    assert!(f.contains_entity(ATTRACTION_INFO, &f.attractions_info[1]));
    assert!(f.contains_entity(ATTRACTION_CONTACT_INFO, &f.attractions_info[0]));
    assert!(f.contains_entity(ATTRACTION_CONTACT_INFO, &f.attractions_info[1]));

    assert_instance_of(ATTRACTION_INFO, &entity1);
    assert_instance_of(ATTRACTION_INFO, &entity2);
    assert_instance_of(ATTRACTION_CONTACT_INFO, &entity1);
    assert_instance_of(ATTRACTION_CONTACT_INFO, &entity2);

    assert_eq!(id(&f.attractions_info[0]), id(&entity1));
    assert_eq!(fone(&f.attractions_info[0]), fone(&entity1));

    assert_eq!(id(&f.attractions_info[1]), id(&entity2));
    assert_eq!(fone(&f.attractions_info[1]), fone(&entity2));

    f.em.clear().unwrap();

    f.reset_query_log();
    let entity3 = f.find(ATTRACTION_INFO, entity_id1).await.unwrap();
    let entity4 = f.find(ATTRACTION_INFO, entity_id2).await.unwrap();

    f.assert_query_count(0);

    assert_instance_of(ATTRACTION_INFO, &entity3);
    assert_instance_of(ATTRACTION_INFO, &entity4);
    assert_instance_of(ATTRACTION_CONTACT_INFO, &entity3);
    assert_instance_of(ATTRACTION_CONTACT_INFO, &entity4);

    assert!(!Arc::ptr_eq(&entity1, &entity3));
    assert_eq!(id(&entity1), id(&entity3));
    assert_eq!(fone(&entity1), fone(&entity3));

    assert!(!Arc::ptr_eq(&entity2, &entity4));
    assert_eq!(id(&entity2), id(&entity4));
    assert_eq!(fone(&entity2), fone(&entity4));
}

#[tokio::test]
async fn testQueryCacheFindAllJoinTableEntities() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;
    f.evict_regions();
    f.em.clear().unwrap();

    f.reset_query_log();
    let dql =
        "SELECT i, a FROM Doctrine\\Tests\\Models\\Cache\\AttractionInfo i JOIN i.attraction a";
    let result1 = cacheable_result(&f, dql).await;

    assert_eq!(f.attractions_info.len(), result1.len());
    f.assert_query_count(1);

    f.em.clear().unwrap();

    let result2 = cacheable_result(&f, dql).await;

    assert_eq!(f.attractions_info.len(), result2.len());
    f.assert_query_count(1);

    for entity in &result2 {
        assert_instance_of(ATTRACTION_INFO, entity);
    }
}

#[tokio::test]
async fn testOneToManyRelationJoinTable() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;
    f.evict_regions();
    f.em.clear().unwrap();

    let entity = f.find(ATTRACTION, id(&f.attractions[0])).await.unwrap();

    assert_instance_of(ATTRACTION, &entity);
    assert_instance_of(
        PERSISTENT_COLLECTION,
        &orm::obj(&orm::get(&entity, "infos")),
    );
    assert_eq!(1, count(&entity, "infos").await);

    assert!(f.contains_entity(ATTRACTION, &f.attractions[0]));
    assert!(f.contains_collection(ATTRACTION, "infos", &f.attractions[0]));

    let info = get_at(&entity, "infos", 0).await;
    assert_instance_of(ATTRACTION_CONTACT_INFO, &info);
    assert_eq!(fone(&f.attractions_info[0]), fone(&info));

    f.em.clear().unwrap();

    let entity = f.find(ATTRACTION, id(&f.attractions[0])).await.unwrap();

    assert_instance_of(ATTRACTION, &entity);
    assert_instance_of(
        PERSISTENT_COLLECTION,
        &orm::obj(&orm::get(&entity, "infos")),
    );
    assert_eq!(1, count(&entity, "infos").await);

    let info = get_at(&entity, "infos", 0).await;
    assert_instance_of(ATTRACTION_CONTACT_INFO, &info);
    assert_eq!(fone(&f.attractions_info[0]), fone(&info));
}

#[tokio::test]
async fn testQueryCacheShouldBeEvictedOnTimestampUpdate() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;
    f.evict_regions();
    f.em.clear().unwrap();

    f.reset_query_log();
    let dql =
        "SELECT attractionInfo FROM Doctrine\\Tests\\Models\\Cache\\AttractionInfo attractionInfo";

    let result1 = cacheable_result(&f, dql).await;

    assert_eq!(f.attractions_info.len(), result1.len());
    f.assert_query_count(5);

    let attraction = f.find(ATTRACTION, id(&f.attractions[5])).await.unwrap();
    let contact =
        slc_functional::attraction_info(ATTRACTION_CONTACT_INFO, "fone", "1234-1234", &attraction);

    f.em.persist(&contact).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    f.reset_query_log();

    let result2 = cacheable_result(&f, dql).await;

    assert_eq!(f.attractions_info.len() + 1, result2.len());
    f.assert_query_count(6);

    for entity in &result2 {
        assert_instance_of(ATTRACTION_INFO, entity);
    }
}
