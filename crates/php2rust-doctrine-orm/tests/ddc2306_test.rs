//! Ticket/DDC2306Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2306Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&[
        "DDC2306Zone",
        "DDC2306User",
        "DDC2306Address",
        "DDC2306UserAddress",
    ])
    .await;

    let zone = new("DDC2306Zone");
    let user = new("DDC2306User");
    set(
        &user,
        "addresses",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    let address = new("DDC2306Address");
    set(
        &address,
        "users",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    // new DDC2306UserAddress($user, $address)
    let user_address = new("DDC2306UserAddress");
    set_obj(&user_address, "user", &user);
    set_obj(&user_address, "address", &address);
    coll::add(
        &get(&user, "addresses"),
        Value::Object(user_address.clone()),
    )
    .unwrap();
    coll::add(&get(&address, "users"), Value::Object(user_address.clone())).unwrap();
    set_obj(&user, "zone", &zone);
    set_obj(&address, "zone", &zone);

    em.persist(&zone).unwrap();
    em.persist(&user).unwrap();
    em.persist(&address).unwrap();
    em.persist(&user_address).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let address = find(&em, "DDC2306Address", get(&address, "id"))
        .await
        .unwrap();
    let first = coll::first(&get(&address, "users")).await.unwrap();
    let user = get(&obj(&first), "user");

    assert!(is_uninitialized(&em, &user));
    assert_instance_of("DDC2306User", &user);

    let user = obj(&user);
    let user_id = get(&user, "id");

    assert!(!matches!(user_id, Value::Null));

    em.get_unit_of_work()
        .initialize_object(&user)
        .await
        .unwrap();

    assert_loose_eq(&user_id, &get(&user, "id"));
}
