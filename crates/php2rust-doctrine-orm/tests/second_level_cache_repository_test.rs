//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheRepositoryTest.php

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

use php_value::{Object, PhpArray};

use slc_functional::{COUNTRY, STATE, assert_instance_of, get_one, id, name, set_up};

/// `$entities[0]` and `$entities[1]` are states with countries.
fn assert_states(entities: &[Object]) {
    assert_instance_of(STATE, &entities[0]);
    assert_instance_of(STATE, &entities[1]);
    assert_instance_of(COUNTRY, &get_one(&entities[0], "country"));
    assert_instance_of(COUNTRY, &get_one(&entities[1], "country"));
}

#[tokio::test]
async fn testRepositoryCacheFind() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;

    f.logger.clear_stats();
    f.em.clear().unwrap();

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    f.reset_query_log();
    let repository = f.em.get_repository(COUNTRY).unwrap();
    let country1 = repository.find(id(&f.countries[0])).await.unwrap().unwrap();
    let country2 = repository.find(id(&f.countries[1])).await.unwrap().unwrap();

    f.assert_query_count(0);

    assert_instance_of(COUNTRY, &country1);
    assert_instance_of(COUNTRY, &country2);

    assert_eq!(2, f.logger.get_hit_count());
    assert_eq!(0, f.logger.get_miss_count());
    assert_eq!(
        2,
        f.logger.get_region_hit_count(&f.get_entity_region(COUNTRY))
    );
}

#[tokio::test]
async fn testRepositoryCacheFindAll() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.evict_regions();
    f.logger.clear_stats();
    f.em.clear().unwrap();

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(!f.contains_entity(COUNTRY, &f.countries[1]));

    let repository = f.em.get_repository(COUNTRY).unwrap();
    f.reset_query_log();

    assert_eq!(2, repository.find_all().await.unwrap().len());
    f.assert_query_count(1);

    f.reset_query_log();
    let countries = repository.find_all().await.unwrap();

    f.assert_query_count(0);

    assert_instance_of(COUNTRY, &countries[0]);
    assert_instance_of(COUNTRY, &countries[1]);

    assert_eq!(3, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_miss_count());

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));
}

#[tokio::test]
async fn testRepositoryCacheFindAllInvalidation() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.evict_regions();
    f.logger.clear_stats();
    f.em.clear().unwrap();

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(!f.contains_entity(COUNTRY, &f.countries[1]));

    let repository = f.em.get_repository(COUNTRY).unwrap();
    f.reset_query_log();

    assert_eq!(2, repository.find_all().await.unwrap().len());
    f.assert_query_count(1);

    f.reset_query_log();
    let countries = repository.find_all().await.unwrap();

    f.assert_query_count(0);

    assert_eq!(2, countries.len());
    assert_instance_of(COUNTRY, &countries[0]);
    assert_instance_of(COUNTRY, &countries[1]);

    let country = slc::country("foo");

    f.em.persist(&country).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    f.reset_query_log();

    assert_eq!(3, repository.find_all().await.unwrap().len());
    f.assert_query_count(1);

    let country = repository.find(id(&country)).await.unwrap().unwrap();

    f.em.remove(&country).await.unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    f.reset_query_log();

    assert_eq!(2, repository.find_all().await.unwrap().len());
    f.assert_query_count(1);
}

#[tokio::test]
async fn testRepositoryCacheFindBy() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.evict_regions();
    f.logger.clear_stats();
    f.em.clear().unwrap();

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));

    let mut criteria = PhpArray::new();
    criteria.insert("name", name(&f.countries[0]));
    let repository = f.em.get_repository(COUNTRY).unwrap();
    f.reset_query_log();

    assert_eq!(
        1,
        repository
            .find_by(&criteria, None, None, None)
            .await
            .unwrap()
            .len()
    );
    f.assert_query_count(1);

    f.reset_query_log();
    let countries = repository
        .find_by(&criteria, None, None, None)
        .await
        .unwrap();

    f.assert_query_count(0);

    assert_eq!(1, countries.len());
    assert_instance_of(COUNTRY, &countries[0]);

    assert_eq!(2, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_miss_count());

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
}

#[tokio::test]
async fn testRepositoryCacheFindOneBy() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.evict_regions();
    f.logger.clear_stats();
    f.em.clear().unwrap();

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));

    let mut criteria = PhpArray::new();
    criteria.insert("name", name(&f.countries[0]));
    let repository = f.em.get_repository(COUNTRY).unwrap();
    f.reset_query_log();

    assert!(
        repository
            .find_one_by(&criteria, None)
            .await
            .unwrap()
            .is_some()
    );
    f.assert_query_count(1);

    f.reset_query_log();
    let country = repository.find_one_by(&criteria, None).await.unwrap();

    f.assert_query_count(0);

    assert_instance_of(COUNTRY, &country.unwrap());

    assert_eq!(2, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_miss_count());

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
}

#[tokio::test]
async fn testRepositoryCacheFindAllToOneAssociation() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;

    f.evict_regions();

    f.logger.clear_stats();
    f.em.clear().unwrap();

    // load from database
    let repository = f.em.get_repository(STATE).unwrap();
    f.reset_query_log();
    let entities = repository.find_all().await.unwrap();

    assert_eq!(4, entities.len());
    f.assert_query_count(1);

    assert_states(&entities);

    // load from cache
    f.reset_query_log();
    let entities = repository.find_all().await.unwrap();

    assert_eq!(4, entities.len());
    f.assert_query_count(0);

    assert_states(&entities);

    // invalidate cache
    let country = f.find(COUNTRY, id(&f.countries[0])).await.unwrap();
    f.em.persist(&slc::state_in("foo", Some(&country))).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    // load from database
    f.reset_query_log();
    let entities = repository.find_all().await.unwrap();

    assert_eq!(5, entities.len());
    f.assert_query_count(1);

    assert_states(&entities);

    // load from cache
    f.reset_query_log();
    let entities = repository.find_all().await.unwrap();

    assert_eq!(5, entities.len());
    f.assert_query_count(0);

    assert_states(&entities);
}
