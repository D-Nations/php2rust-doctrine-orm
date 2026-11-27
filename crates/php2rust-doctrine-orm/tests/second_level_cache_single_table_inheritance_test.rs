//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheSingleTableInheritanceTest.php

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

use php_value::Object;

use slc_functional::{
    ATTRACTION, BAR, BEACH, CITY, Fixture, RESTAURANT, assert_instance_of, count, entities, get_at,
    id, name, set_up,
};

const PERSISTENT_COLLECTION: &str = "Doctrine\\ORM\\PersistentCollection";

/// `loadFixturesCountries()` … `loadFixturesAttractions()`.
async fn load_fixtures(f: &mut Fixture) {
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.load_fixtures_attractions().await;
}

/// `$this->_em->createQuery($dql)->setCacheable(true)->getResult()`.
async fn cacheable_result(f: &Fixture, dql: &str) -> Vec<Object> {
    let query = f.em.create_query(dql);
    query.set_cacheable(true);
    entities(&query.get_object_result().await.unwrap())
}

/// `self::assertInstanceOf(PersistentCollection::class, $entity->getAttractions())`.
fn assert_persistent_collection(entity: &Object) {
    assert_instance_of(
        PERSISTENT_COLLECTION,
        &orm::obj(&orm::get(entity, "attractions")),
    );
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
    let attraction_region = region_name(ATTRACTION);

    assert_eq!(attraction_region, region_name(RESTAURANT));
    assert_eq!(attraction_region, region_name(BEACH));
    assert_eq!(attraction_region, region_name(BAR));
}

#[tokio::test]
async fn testPutOnPersistSingleTableInheritance() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;

    f.em.clear().unwrap();

    assert!(f.contains_entity(BAR, &f.attractions[0]));
    assert!(f.contains_entity(BAR, &f.attractions[1]));
}

#[tokio::test]
async fn testCountaisRootClass() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;

    f.em.clear().unwrap();

    for attraction in &f.attractions {
        assert!(f.contains_entity(ATTRACTION, attraction));
        assert!(f.contains_entity(attraction.class(), attraction));
    }
}

#[tokio::test]
async fn testPutAndLoadEntities() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;

    f.em.clear().unwrap();

    f.cache.evict_entity_region(ATTRACTION).unwrap();

    let entity_id1 = id(&f.attractions[0]);
    let entity_id2 = id(&f.attractions[1]);

    assert!(!f.contains_entity(ATTRACTION, &f.attractions[0]));
    assert!(!f.contains_entity(ATTRACTION, &f.attractions[1]));
    assert!(!f.contains_entity(BAR, &f.attractions[0]));
    assert!(!f.contains_entity(BAR, &f.attractions[1]));

    let entity1 = f.find(ATTRACTION, entity_id1.clone()).await.unwrap();
    let entity2 = f.find(ATTRACTION, entity_id2.clone()).await.unwrap();

    assert!(f.contains_entity(ATTRACTION, &f.attractions[0]));
    assert!(f.contains_entity(ATTRACTION, &f.attractions[1]));
    assert!(f.contains_entity(BAR, &f.attractions[0]));
    assert!(f.contains_entity(BAR, &f.attractions[1]));

    assert_instance_of(ATTRACTION, &entity1);
    assert_instance_of(ATTRACTION, &entity2);
    assert_instance_of(BAR, &entity1);
    assert_instance_of(BAR, &entity2);

    assert_eq!(id(&f.attractions[0]), id(&entity1));
    assert_eq!(name(&f.attractions[0]), name(&entity1));

    assert_eq!(id(&f.attractions[1]), id(&entity2));
    assert_eq!(name(&f.attractions[1]), name(&entity2));

    f.em.clear().unwrap();

    f.reset_query_log();

    let entity3 = f.find(ATTRACTION, entity_id1).await.unwrap();
    let entity4 = f.find(ATTRACTION, entity_id2).await.unwrap();

    f.assert_query_count(0);

    assert_instance_of(ATTRACTION, &entity3);
    assert_instance_of(ATTRACTION, &entity4);
    assert_instance_of(BAR, &entity3);
    assert_instance_of(BAR, &entity4);

    assert!(!std::sync::Arc::ptr_eq(&entity1, &entity3));
    assert_eq!(id(&entity1), id(&entity3));
    assert_eq!(name(&entity1), name(&entity3));

    assert!(!std::sync::Arc::ptr_eq(&entity2, &entity4));
    assert_eq!(id(&entity2), id(&entity4));
    assert_eq!(name(&entity2), name(&entity4));
}

#[tokio::test]
async fn testQueryCacheFindAll() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;

    f.em.clear().unwrap();

    f.reset_query_log();
    let dql = "SELECT a FROM Doctrine\\Tests\\Models\\Cache\\Attraction a";
    let result1 = cacheable_result(&f, dql).await;

    assert_eq!(f.attractions.len(), result1.len());
    f.assert_query_count(1);

    f.em.clear().unwrap();

    let result2 = cacheable_result(&f, dql).await;

    assert_eq!(f.attractions.len(), result2.len());
    f.assert_query_count(1);

    for entity in &result2 {
        assert_instance_of(ATTRACTION, entity);
    }
}

#[tokio::test]
async fn testShouldNotPutOneToManyRelationOnPersist() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;

    f.em.clear().unwrap();

    for city in &f.cities {
        assert!(f.contains_entity(CITY, city));
        assert!(!f.contains_collection(CITY, "attractions", city));
    }

    for attraction in &f.attractions {
        assert!(f.contains_entity(ATTRACTION, attraction));
    }
}

#[tokio::test]
async fn testOneToManyRelationSingleTable() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;

    f.cache.evict_entity_region(CITY).unwrap();
    f.cache.evict_entity_region(ATTRACTION).unwrap();
    f.cache
        .evict_collection_region(CITY, "attractions")
        .unwrap();

    f.em.clear().unwrap();

    let entity = f.find(CITY, id(&f.cities[0])).await.unwrap();

    assert_instance_of(CITY, &entity);
    assert_persistent_collection(&entity);
    assert_eq!(2, count(&entity, "attractions").await);

    let owner_id = id(&f.cities[0]);
    f.reset_query_log();

    assert!(f.contains_entity(CITY, &f.cities[0]));
    assert!(f.contains_collection(CITY, "attractions", &f.cities[0]));

    assert_instance_of(BAR, &get_at(&entity, "attractions", 0).await);
    assert_instance_of(BAR, &get_at(&entity, "attractions", 1).await);
    assert_eq!(
        name(&f.attractions[0]),
        name(&get_at(&entity, "attractions", 0).await)
    );
    assert_eq!(
        name(&f.attractions[1]),
        name(&get_at(&entity, "attractions", 1).await)
    );

    f.em.clear().unwrap();

    let entity = f.find(CITY, owner_id).await.unwrap();

    assert_instance_of(CITY, &entity);
    assert_persistent_collection(&entity);
    assert_eq!(2, count(&entity, "attractions").await);

    f.assert_query_count(0);

    assert_instance_of(BAR, &get_at(&entity, "attractions", 0).await);
    assert_instance_of(BAR, &get_at(&entity, "attractions", 1).await);
    assert_eq!(
        name(&f.attractions[0]),
        name(&get_at(&entity, "attractions", 0).await)
    );
    assert_eq!(
        name(&f.attractions[1]),
        name(&get_at(&entity, "attractions", 1).await)
    );
}

#[tokio::test]
async fn testQueryCacheShouldBeEvictedOnTimestampUpdate() {
    let mut f = set_up().await;
    load_fixtures(&mut f).await;
    f.em.clear().unwrap();

    f.reset_query_log();
    let dql = "SELECT attraction FROM Doctrine\\Tests\\Models\\Cache\\Attraction attraction";

    let result1 = cacheable_result(&f, dql).await;

    assert_eq!(f.attractions.len(), result1.len());
    f.assert_query_count(1);

    let city = f.find(CITY, id(&f.cities[1])).await.unwrap();
    let contact = slc_functional::attraction(BEACH, "Botafogo", &city);

    f.em.persist(&contact).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    f.reset_query_log();

    let result2 = cacheable_result(&f, dql).await;

    assert_eq!(f.attractions.len() + 1, result2.len());
    f.assert_query_count(1);

    for entity in &result2 {
        assert_instance_of(ATTRACTION, entity);
    }
}
