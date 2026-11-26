//! Ticket/DDC2780Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2780Test.php

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

use php_value::PhpArray;
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC2780User", "DDC2780Project"]).await;

    let user = new("DDC2780User");
    let project = new("DDC2780Project");
    set(
        &project,
        "users",
        ArrayCollectionObject::value(PhpArray::new()),
    );

    set_obj(&user, "project", &project);

    em.persist(&project).unwrap();
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let mut qb = em.create_query_builder();
    qb.select(vec![s("user")])
        .unwrap()
        .from(&class("DDC2780User"), "user", None)
        .unwrap()
        .left_join("user.project", "project", None, None, None)
        .unwrap()
        .r#where(vec![s("project IS NOT NULL")])
        .unwrap();
    let result = qb
        .get_query()
        .unwrap()
        .get_one_or_null_result(None)
        .await
        .unwrap();

    assert_instance_of("DDC2780User", &result);
}
