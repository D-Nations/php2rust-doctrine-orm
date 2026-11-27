//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH5562Test.php
//!
//! `enableSecondLevelCache()`: the second-level cache fixture of
//! `tests/common/slc_functional.rs` with the recorded models of the test
//! file. `random_int(0, mt_getrandmax())` is a fixed integer. Reading a
//! property of a lazy proxy does not load it: the test calls
//! `initialize_object()` where PHP's property access on the proxy would.

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

use orm::{assert_loose_eq, get, obj, s, set, set_obj};
use ticket::{class, new};

/// GH-5562
#[tokio::test]
async fn testCacheShouldBeUpdatedWhenAssociationChanges() {
    let f = slc_functional::set_up_with(&[
        &class("GH5562User"),
        &class("GH5562Manager"),
        &class("GH5562Merchant"),
    ])
    .await;

    let manager = new("GH5562Manager");
    let merchant = new("GH5562Merchant");

    set(&manager, "username", s("username"));
    set_obj(&manager, "merchant", &merchant);
    set_obj(&merchant, "manager", &manager);

    set(&merchant, "name", s("Merchant"));

    f.em.persist(&merchant).unwrap();
    f.em.persist(&manager).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let merchant =
        f.em.find(&class("GH5562Merchant"), get(&merchant, "id"))
            .await
            .unwrap()
            .unwrap();

    set(&merchant, "name", Value::Int(1_804_289_383));
    let manager = obj(&get(&merchant, "manager"));
    f.em.initialize_object(&manager).await.unwrap();
    set(&manager, "username", s("usernameUPDATE"));

    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let merchant =
        f.em.find(&class("GH5562Merchant"), get(&merchant, "id"))
            .await
            .unwrap()
            .unwrap();

    let manager = obj(&get(&merchant, "manager"));
    f.em.initialize_object(&manager).await.unwrap();
    assert_loose_eq(&s("usernameUPDATE"), &get(&manager, "username"));
}
