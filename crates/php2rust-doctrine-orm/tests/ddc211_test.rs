//! Ticket/DDC211Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC211Test.php

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
    let em = ticket::set_up(&["DDC211User", "DDC211Group"]).await;

    let user = new("DDC211User");
    set(
        &user,
        "groups",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&user, "name", s("John Doe"));

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let group_names = ["group 1", "group 2", "group 3", "group 4"];
    for name in group_names {
        let group = new("DDC211Group");
        set(
            &group,
            "users",
            ArrayCollectionObject::value(PhpArray::new()),
        );
        set(&group, "name", s(name));
        em.persist(&group).unwrap();
        em.flush().await.unwrap();

        if !contains(&user, "groups", &group).await {
            coll::add(&get(&user, "groups"), Value::Object(group.clone())).unwrap();
            coll::add(&get(&group, "users"), Value::Object(user.clone())).unwrap();
            em.flush().await.unwrap();
        }
    }

    assert_eq!(4, coll::count(&get(&user, "groups")).await.unwrap());
}
