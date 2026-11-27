//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC4003Test.php
//!
//! A `SecondLevelCacheFunctionalTestCase` (`tests/common/slc_functional.rs`).
//! `uniqid()` is a name no fixture has.

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

use slc_functional::{BAR, assert_instance_of, id, name};

#[tokio::test]
async fn testReadsThroughRepositorySameDataThatItWroteInCache() {
    let mut f = slc_functional::set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.load_fixtures_attractions().await;

    // Get the id of the first bar
    let id = id(&f.attractions[0]);

    let repository = f.em.get_repository(BAR).unwrap();
    let mut criteria = PhpArray::new();
    criteria.insert("id", id);

    // This instance is fresh new, no QueryCache, so the full entity gets loaded from DB.
    // It will be saved in the WRONG KEY (notice the cache.bar at the end):
    // doctrine_tests_models_cache_attraction[doctrine_tests_models_cache_attraction_doctrine.tests.models.cache.bar_1][1]
    let bar = repository
        .find_one_by(&criteria, None)
        .await
        .unwrap()
        .unwrap();
    assert_instance_of(BAR, &bar);

    // Let's change it so that we can compare its state
    let new_name = orm::s("6593c0a1e7f3d");
    orm::set(&bar, "name", new_name.clone());
    f.em.persist(&bar).unwrap();
    f.em.flush().await.unwrap();

    // Flush did 2 important things for us:
    // 1: saved the entity in its right key (notice the cache.attraction at the end):
    //     doctrine_tests_models_cache_attraction[doctrine_tests_models_cache_attraction_doctrine.tests.models.cache.attraction_1][1]
    // 2: Updated the TimestampRegion cache, so that the QueryCache is actually discarded.
    //
    // So, first findOneBy will hit DB, and its entity will not be loaded from cache.
    repository.find_one_by(&criteria, None).await.unwrap();

    // Lets clear EM so that we don't hit IdentityMap at all.
    f.em.clear().unwrap();

    // Here's the failing step:
    // Right now QueryCache will HIT, as nothing changed between the last one and now.
    // QueryCache holds a reference to the WRONG KEY, as we saw was formed in line 24 of this test.
    // So this instance won't be updated and will have the original name ("Boteco São Bento"), and not the uniqid().
    let cached = repository
        .find_one_by(&criteria, None)
        .await
        .unwrap()
        .unwrap();
    assert_instance_of(BAR, &cached);

    orm::assert_loose_eq(&new_name, &name(&cached));
}
