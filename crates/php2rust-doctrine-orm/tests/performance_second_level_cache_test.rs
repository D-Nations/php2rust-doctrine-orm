//! Upstream: doctrine/orm tests/Tests/ORM/Performance/SecondLevelCacheTest.php
//!
//! A `...WithCache` test builds its entity manager with the second-level
//! cache and without a cache logger (`enableSecondLevelCache(false)`).
//! PHP calls `enableSecondLevelCache()` after `parent::setUp()` has
//! created the entity manager, where the flag takes effect only for the
//! next entity manager; the expected counts are those of a cached run.
//! The timings are printed as PHP echoes them.
//!
//! The query log counts statements only. PHP's counts (the `performance`
//! group, not run by default) are two more per flush than the statements
//! these tests run; the counts asserted here are the statements: PHP's
//! count minus two for each `flush()` (one flush in each test, three in
//! `findEntityOneToMany()`).

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
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
use std::time::Instant;

use php_value::Value;
use php2rust_doctrine_orm::cache::pool::ArrayAdapter;
use php2rust_doctrine_orm::cache::{CacheConfiguration, DefaultCacheFactory};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::driver::attribute_driver::Paths;
use php2rust_doctrine_persistence::mapping::driver::ClassLocator;

use query_log::{QueryLog, logging_connection};
use slc_functional::{CACHE_MODELS, COUNTRY, FunctionalModelsLocator, STATE, id};

/// `setUp()` with `useModelSet('cache')`; `enableSecondLevelCache(false)`
/// when `second_level_cache`.
async fn set_up(second_level_cache: bool) -> (Arc<EntityManager>, Arc<QueryLog>) {
    let (conn, log) = logging_connection();
    let em = orm::entity_manager_with(conn, move |c| {
        let locator: Arc<dyn ClassLocator> = Arc::new(FunctionalModelsLocator);
        c.set_metadata_driver_impl(Arc::new(
            AttributeDriver::new(Paths::Locator(locator), true).unwrap(),
        ));
        if second_level_cache {
            let cache_config = Arc::new(CacheConfiguration::new());
            let factory = Arc::new(DefaultCacheFactory::new(
                cache_config.get_regions_configuration(),
                Arc::new(ArrayAdapter::new()),
            ));
            cache_config.set_cache_factory(factory);
            c.set_second_level_cache_enabled(true);
            c.set_second_level_cache_configuration(cache_config);
        }
    });
    orm::create_schema(&em, &CACHE_MODELS).await;
    (em, log)
}

/// `$this->assertQueryCount($expectedCount)`.
#[track_caller]
fn assert_query_count(log: &QueryLog, expected_count: usize) {
    assert_eq!(expected_count, log.queries().len());
}

fn elapsed(start: Instant) -> String {
    format!("{:.6}", start.elapsed().as_secs_f64())
}

fn rule() {
    print!("\n{}\n", "-".repeat(50));
}

async fn query_entity(em: &Arc<EntityManager>, label: &str) {
    let times = 100;
    let size = 500;
    let start_persist = Instant::now();

    print!("\n{label}");

    for i in 0..size {
        em.persist(&slc::country(&format!("Country {i}"))).unwrap();
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    print!("\n[{}] persist {size} countries", elapsed(start_persist));

    let dql = "SELECT c FROM Doctrine\\Tests\\Models\\Cache\\Country c WHERE c.name LIKE :name";
    let start_find = Instant::now();

    for _ in 0..times {
        let query = em.create_query(dql);
        query
            .set_parameter("name", Value::string("%Country%"), None)
            .set_cacheable(true);
        query.get_object_result().await.unwrap();
    }

    print!(
        "\n[{}] select {size} countries ({times} times)",
        elapsed(start_find)
    );
    rule();
}

async fn find_entity_one_to_many(em: &Arc<EntityManager>, label: &str) {
    let times = 50;
    let size = 30;
    let mut states = Vec::new();
    let mut cities = Vec::new();
    let start_persist = Instant::now();
    let country = slc::country("Country");

    print!("\n{label}");

    em.persist(&country).unwrap();
    em.flush().await.unwrap();

    for i in 0..size / 2 {
        let state = slc::state_in(&format!("State {i}"), Some(&country));

        em.persist(&state).unwrap();

        states.push(state);
    }

    em.flush().await.unwrap();

    for (key, state) in states.iter().enumerate() {
        for i in 0..size {
            let city = slc::city(&format!("City {key} - {i}"), Some(state));

            em.persist(&city).unwrap();

            slc::add_city(state, &city);

            cities.push(city);
        }
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    print!(
        "\n[{}] persist {} states and {} cities",
        elapsed(start_persist),
        states.len(),
        cities.len()
    );

    let start_find = Instant::now();

    for _ in 0..times {
        for state in &states {
            let state = em.find(STATE, id(state)).await.unwrap().unwrap();

            let cities = orm::get(&state, "cities");
            for city in php2rust_doctrine_orm::collection_object::get_values(&cities)
                .await
                .unwrap()
            {
                let _ = orm::get(&orm::obj(&city), "name");
            }
        }
    }

    print!(
        "\n[{}] find {} states and {} cities ({times} times)",
        elapsed(start_find),
        states.len(),
        cities.len()
    );
    rule();
}

async fn find_entity(em: &Arc<EntityManager>, label: &str) {
    let times = 10;
    let size = 500;
    let mut countries = Vec::new();
    let start_persist = Instant::now();

    print!("\n{label}");

    for i in 0..size {
        let country = slc::country(&format!("Country {i}"));

        em.persist(&country).unwrap();

        countries.push(country);
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    print!("\n[{}] persist {size} countries", elapsed(start_persist));

    let start_find = Instant::now();

    for _ in 0..=times {
        for country in &countries {
            em.find(COUNTRY, id(country)).await.unwrap();
            em.clear().unwrap();
        }
    }

    print!(
        "\n[{}] find {size} countries ({times} times)",
        elapsed(start_find)
    );
    rule();
}

async fn find_all_entity(em: &Arc<EntityManager>, label: &str) {
    let times = 100;
    let size = 50;
    let start_persist = Instant::now();
    let rep = em.get_repository(COUNTRY).unwrap();

    print!("\n{label}");

    for i in 0..size {
        em.persist(&slc::country(&format!("Country {i}"))).unwrap();
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    print!("\n[{}] persist {size} countries", elapsed(start_persist));

    let start_find = Instant::now();

    for _ in 0..=times {
        let list = rep.find_all().await.unwrap();
        em.clear().unwrap();

        assert_eq!(size, list.len());
    }

    print!(
        "\n[{}] find {size} countries ({times} times)",
        elapsed(start_find)
    );
    rule();
}

#[tokio::test]
async fn testFindEntityWithoutCache() {
    let (em, log) = set_up(false).await;
    log.reset_and_enable();
    find_entity(&em, "testFindEntityWithoutCache").await;

    assert_query_count(&log, 6000); // PHP: 6002
}

#[tokio::test]
async fn testFindEntityWithCache() {
    let (em, log) = set_up(true).await;

    log.reset_and_enable();
    find_entity(&em, "testFindEntityWithCache").await;

    assert_query_count(&log, 500); // PHP: 502
}

#[tokio::test]
async fn testFindAllEntityWithoutCache() {
    let (em, log) = set_up(false).await;
    log.reset_and_enable();
    find_all_entity(&em, "testFindAllEntityWithoutCache").await;

    assert_query_count(&log, 151); // PHP: 153
}

#[tokio::test]
async fn testFindAllEntityWithCache() {
    let (em, log) = set_up(true).await;

    log.reset_and_enable();
    find_all_entity(&em, "testFindAllEntityWithCache").await;

    assert_query_count(&log, 51); // PHP: 53
}

#[tokio::test]
async fn testFindEntityOneToManyWithoutCache() {
    let (em, log) = set_up(false).await;
    log.reset_and_enable();
    find_entity_one_to_many(&em, "testFindEntityOneToManyWithoutCache").await;

    assert_query_count(&log, 496); // PHP: 502
}

#[tokio::test]
async fn testFindEntityOneToManyWithCache() {
    let (em, log) = set_up(true).await;

    log.reset_and_enable();
    find_entity_one_to_many(&em, "testFindEntityOneToManyWithCache").await;

    assert_query_count(&log, 466); // PHP: 472
}

#[tokio::test]
async fn testQueryEntityWithoutCache() {
    let (em, log) = set_up(false).await;
    log.reset_and_enable();
    query_entity(&em, "testQueryEntityWithoutCache").await;

    assert_query_count(&log, 600); // PHP: 602
}

#[tokio::test]
async fn testQueryEntityWithCache() {
    let (em, log) = set_up(true).await;

    log.reset_and_enable();
    query_entity(&em, "testQueryEntityWithCache").await;

    assert_query_count(&log, 501); // PHP: 503
}
