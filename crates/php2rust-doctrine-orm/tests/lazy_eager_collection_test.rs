//! Ticket/LazyEagerCollectionTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/LazyEagerCollectionTest.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

/// `$user->addPhone($phone)` and `$user->addAddress($address)`.
fn add(user: &Object, field: &str, item: &Object) {
    set_obj(item, "user", user);
    append(user, field, Value::Object(item.clone()));
}

fn with_data(short: &str, data: &str) -> Object {
    let object = new(short);
    set(&object, "data", s(data));
    object
}

#[tokio::test]
async fn testRefreshRefreshesBothLazyAndEagerCollections() {
    let em = ticket::set_up(&[
        "LazyEagerCollectionUser",
        "LazyEagerCollectionAddress",
        "LazyEagerCollectionPhone",
    ])
    .await;
    let user = with_data("LazyEagerCollectionUser", "Guilherme");
    set(
        &user,
        "addresses",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(
        &user,
        "phones",
        ArrayCollectionObject::value(PhpArray::new()),
    );

    let ph = with_data("LazyEagerCollectionPhone", "12345");
    add(&user, "phones", &ph);

    let ad = with_data("LazyEagerCollectionAddress", "6789");
    add(&user, "addresses", &ad);

    em.persist(&user).unwrap();
    em.persist(&ad).unwrap();
    em.persist(&ph).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = find(&em, "LazyEagerCollectionUser", get(&user, "id"))
        .await
        .unwrap();
    let ph = obj(&coll::get(&get(&user, "phones"), 0).await.unwrap());
    let ad = obj(&coll::get(&get(&user, "addresses"), 0).await.unwrap());

    set(&ph, "data", s("abc"));
    set(&ad, "data", s("def"));

    em.refresh(&user).await.unwrap();

    assert!(s("12345").identical(&get(&ph, "data")));
    assert!(s("6789").identical(&get(&ad, "data")));
}
