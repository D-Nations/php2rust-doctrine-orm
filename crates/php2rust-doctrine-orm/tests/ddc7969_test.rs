//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC7969Test.php
//!
//! A `SecondLevelCacheFunctionalTestCase` (`tests/common/slc_functional.rs`).

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
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

use php_value::PhpArray;

use slc_functional::{ATTRACTION, BAR, assert_instance_of, name};

#[tokio::test]
async fn testChildEntityRetrievedFromCache() {
    let mut f = slc_functional::set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.load_fixtures_attractions().await;

    // Entities are already cached due to fixtures - hence flush before testing
    f.cache
        .get_entity_cache_region(ATTRACTION)
        .unwrap()
        .unwrap()
        .evict_all()
        .unwrap();

    let bar = f.attractions[0].clone();
    assert_instance_of(BAR, &bar);

    let repository = f.em.get_repository(BAR).unwrap();

    assert!(!f.contains_entity(BAR, &bar));
    assert!(!f.contains_entity(ATTRACTION, &bar));

    let mut criteria = PhpArray::new();
    criteria.insert("name", name(&bar));
    repository.find_one_by(&criteria, None).await.unwrap();

    assert!(f.contains_entity(BAR, &bar));

    repository.find_one_by(&criteria, None).await.unwrap();

    // One hit for entity cache, one hit for query cache
    assert_eq!(2, f.logger.get_hit_count());
}
