//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3967Test.php
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

use php_value::Value;
use php2rust_doctrine_orm::utility::to_text;

use slc_functional::{COUNTRY, Fixture, assert_instance_of, id};

async fn set_up() -> Fixture {
    let mut f = slc_functional::set_up().await;

    f.load_fixtures_countries().await;
    f.em.get_cache()
        .unwrap()
        .evict_entity_region(COUNTRY)
        .unwrap();
    f.em.clear().unwrap();
    f
}

#[tokio::test]
async fn testIdentifierCachedWithProperType() {
    let mut f = set_up().await;
    let country = f.countries.pop().unwrap();
    let id = id(&country);

    // First time, loaded from database
    f.em.find(COUNTRY, Value::from(to_text(&id).as_str()))
        .await
        .unwrap();
    f.em.clear().unwrap();

    let country =
        f.em.find(COUNTRY, Value::from(to_text(&id).as_str()))
            .await
            .unwrap()
            .unwrap();
    assert_instance_of(COUNTRY, &country);

    // Identifier type should be integer
    assert!(slc_functional::id(&country).identical(&id));
}
