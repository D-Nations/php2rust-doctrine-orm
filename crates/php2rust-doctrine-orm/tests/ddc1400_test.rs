//! Ticket/DDC1400Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1400Test.php

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

use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testFailingCase() {
    let classes = [
        class("DDC1400Article"),
        class("DDC1400User"),
        class("DDC1400UserState"),
    ];
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    let (em, log) = query_log::set_up_logged(&classes).await;

    let article = new("DDC1400Article");
    let user1 = new("DDC1400User");
    let user2 = new("DDC1400User");

    em.persist(&article).unwrap();
    em.persist(&user1).unwrap();
    em.persist(&user2).unwrap();
    em.flush().await.unwrap();

    let user_state1 = new("DDC1400UserState");
    set_obj(&user_state1, "article", &article);
    set(&user_state1, "articleId", get(&article, "id"));
    set_obj(&user_state1, "user", &user1);
    set(&user_state1, "userId", get(&user1, "id"));

    let user_state2 = new("DDC1400UserState");
    set_obj(&user_state2, "article", &article);
    set(&user_state2, "articleId", get(&article, "id"));
    set_obj(&user_state2, "user", &user2);
    set(&user_state2, "userId", get(&user2, "id"));

    em.persist(&user_state1).unwrap();
    em.persist(&user_state2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let user1 = em
        .get_reference(&class("DDC1400User"), get(&user1, "id"))
        .await
        .unwrap()
        .unwrap();

    let query = em.create_query(&format!(
        "SELECT a, s FROM {} a JOIN a.userStates s WITH s.user = :activeUser",
        class("DDC1400Article")
    ));
    query.set_parameter("activeUser", php_value::Value::Object(user1), None);
    query.get_result(HYDRATE_OBJECT).await.unwrap();

    log.reset_and_enable();

    em.flush().await.unwrap();

    assert_eq!(
        0,
        log.queries().len(),
        "No query should be executed during flush in this case"
    );
}
