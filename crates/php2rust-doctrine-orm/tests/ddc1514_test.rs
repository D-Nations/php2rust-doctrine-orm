//! Ticket/DDC1514Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1514Test.php

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
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;

use orm::*;
use ticket::*;

/// `new DDC1514EntityA()`.
fn entity_a(title: &str) -> Object {
    let a = new("DDC1514EntityA");
    set(
        &a,
        "entitiesB",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&a, "title", s(title));
    a
}

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC1514EntityA", "DDC1514EntityB", "DDC1514EntityC"]).await;

    let a1 = entity_a("1foo");

    let a2 = entity_a("2bar");

    let b1 = new("DDC1514EntityB");
    set_obj(&b1, "entityAFrom", &a1);
    set_obj(&b1, "entityATo", &a2);

    let b2 = new("DDC1514EntityB");
    set_obj(&b2, "entityAFrom", &a2);
    set_obj(&b2, "entityATo", &a1);

    let c = new("DDC1514EntityC");
    set(&c, "title", s("baz"));
    set_obj(&a2, "entityC", &c);

    em.persist(&a1).unwrap();
    em.persist(&a2).unwrap();
    em.persist(&b1).unwrap();
    em.persist(&b2).unwrap();
    em.persist(&c).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql = format!(
        "SELECT a, b, ba, c FROM {NS}DDC1514EntityA AS a LEFT JOIN a.entitiesB AS b LEFT JOIN b.entityATo AS ba LEFT JOIN a.entityC AS c ORDER BY a.title"
    );
    let results = list(
        &em.create_query(&dql)
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    );

    let (r0, r1) = (obj(&results[0]), obj(&results[1]));
    assert_loose_eq(&get(&a1, "id"), &get(&r0, "id"));
    std::assert_matches!(get(&r0, "entityC"), Value::Null);

    assert_loose_eq(&get(&a2, "id"), &get(&r1, "id"));
    assert_loose_eq(&get(&c, "title"), &get(&obj(&get(&r1, "entityC")), "title"));
}
