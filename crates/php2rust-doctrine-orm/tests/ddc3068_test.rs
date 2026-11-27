//! Ticket/DDC3068Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the taxi models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3068Test.php

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
#[path = "common/taxi.rs"]
mod taxi;

use php_value::{PhpArray, Value};

use orm::{get, same};
use taxi::*;

#[tokio::test]
async fn testFindUsingAnArrayOfObjectAsPrimaryKey() {
    let em = orm::set_up(&TAXI_MODELS).await;

    let foo_bar = driver("Foo Bar");
    em.persist(&foo_bar).unwrap();
    let merc = car("Mercedes", "C-Class");
    em.persist(&merc).unwrap();
    em.flush().await.unwrap();

    em.persist(&ride(RIDE, &foo_bar, &merc)).unwrap();
    em.flush().await.unwrap();

    let mut id = PhpArray::new();
    id.insert("driver", get(&foo_bar, "id"));
    id.insert("car", get(&merc, "brand"));
    let ride1 = em.find(RIDE, Value::Array(id)).await.unwrap();
    assert!(ride1.as_ref().is_some_and(|r| r.instance_of(RIDE)));

    let mut id = PhpArray::new();
    id.insert("driver", Value::Object(foo_bar.clone()));
    id.insert("car", Value::Object(merc.clone()));
    let ride2 = em.find(RIDE, Value::Array(id)).await.unwrap();
    assert!(ride2.as_ref().is_some_and(|r| r.instance_of(RIDE)));
    assert!(same(&ride1.unwrap(), &ride2.unwrap()));
}
