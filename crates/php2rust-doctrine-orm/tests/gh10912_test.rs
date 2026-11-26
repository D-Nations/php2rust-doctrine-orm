//! Ticket/GH10912Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10912Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let classes = [
        class("GH10912User"),
        class("GH10912Profile"),
        class("GH10912Room"),
    ];
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    let (em, log) = query_log::set_up_logged(&classes).await;

    let user = new("GH10912User");
    set(
        &user,
        "rooms",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    let profile = new("GH10912Profile");
    let room = new("GH10912Room");

    coll::add(&get(&user, "rooms"), Value::Object(room.clone())).unwrap();
    set_obj(&user, "profile", &profile);
    set_obj(&profile, "user", &user);
    set_obj(&room, "user", &user);

    em.persist(&room).unwrap();
    em.persist(&user).unwrap();
    em.persist(&profile).unwrap();
    em.flush().await.unwrap();

    // We need to delete $room before $user since $room.user_id references
    // $user.id; the other two may go in either order.
    em.remove(&profile).await.unwrap();
    em.remove(&user).await.unwrap();
    em.remove(&room).await.unwrap();

    log.reset_and_enable();
    em.flush().await.unwrap();

    let queries: Vec<String> = log
        .queries()
        .into_iter()
        .filter(|sql| sql.starts_with("DELETE"))
        .collect();

    assert_eq!(3, queries.len());

    // we do not care about the order of $user vs. $profile, so do not check them.
    assert_eq!(
        "DELETE FROM GH10912Room WHERE id = ?", queries[0],
        "$room deletion is the first query"
    );

    // The EntityManager is aware that all three entities have been deleted (sanity check)
    let im = em.get_unit_of_work().get_identity_map();
    for short in ["GH10912Profile", "GH10912User", "GH10912Room"] {
        let class = class(short);
        assert!(
            im.iter()
                .filter(|(name, _)| *name == class)
                .all(|(_, entities)| entities.is_empty())
        );
    }
}
