//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7735Test.php
//!
//! `enableSecondLevelCache()`: the second-level cache fixture of
//! `tests/common/slc_functional.rs` with the recorded models of the test
//! file. Reading a property of a lazy proxy does not load it: the test
//! calls `initialize_object()` where PHP's getter on the proxy would.

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
#[path = "common/ticket.rs"]
mod ticket;

use php_value::Value;

use orm::{get, obj, s, set, set_obj};
use ticket::{class, new};

async fn set_up() -> slc_functional::Fixture {
    let f = slc_functional::set_up_with(&[
        &class("GH7735Car"),
        &class("GH7735Power"),
        &class("GH7735Engine"),
    ])
    .await;

    // new GH7735Car(1, new GH7735Engine(1, 'turbo', new GH7735Power(1)))
    let power = new("GH7735Power");
    set(&power, "id", Value::Int(1));
    let engine = new("GH7735Engine");
    set(&engine, "id", Value::Int(1));
    set(&engine, "model", s("turbo"));
    set_obj(&engine, "power", &power);
    set_obj(&power, "engine", &engine);
    let car = new("GH7735Car");
    set(&car, "id", Value::Int(1));
    set_obj(&car, "engine", &engine);

    f.em.persist(&car).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();
    f
}

/// GH7735
#[tokio::test]
async fn findByReturnsCachedEntity() {
    let f = set_up().await;
    f.em.get_cache()
        .unwrap()
        .evict_entity_region(&class("GH7735Power"))
        .unwrap();

    let car =
        f.em.find(&class("GH7735Car"), Value::Int(1))
            .await
            .unwrap()
            .unwrap();
    assert!(car.instance_of(&class("GH7735Car")));

    let engine = obj(&get(&car, "engine"));
    f.em.initialize_object(&engine).await.unwrap();
    assert!(s("turbo").identical(&get(&engine, "model")));
    let power = obj(&get(&engine, "power"));
    assert!(Value::Int(1).identical(&get(&power, "id")));
}
