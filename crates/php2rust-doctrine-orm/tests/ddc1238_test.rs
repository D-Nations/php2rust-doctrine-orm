//! Ticket/DDC1238Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1238Test.php

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

use php_value::Value;

use orm::*;

const USER: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1238User";

#[tokio::test]
async fn testIssue() {
    let em = orm::set_up(&[USER]).await;

    let user = new_object(USER);
    set(&user, "name", s("test"));

    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user_id = get(&user, "id");
    em.clear().unwrap();

    let user = em
        .get_reference(USER, user_id.clone())
        .await
        .unwrap()
        .unwrap();
    em.clear().unwrap();

    let user_id2 = get(&user, "id");

    assert_loose_eq(&user_id, &user_id2);
}

#[tokio::test]
async fn testIssueProxyClear() {
    let em = orm::set_up(&[USER]).await;

    let user = new_object(USER);
    set(&user, "name", s("test"));

    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user_id = get(&user, "id");
    em.clear().unwrap();

    let user = em
        .get_reference(USER, user_id.clone())
        .await
        .unwrap()
        .unwrap();
    em.clear().unwrap();

    let user2 = em.get_reference(USER, user_id).await.unwrap().unwrap();

    //$this->initializeObject($user);
    assert!(
        matches!(get(&user, "id"), Value::Int(_)),
        "Even if a proxy is detached, it should still have an identifier"
    );

    em.initialize_object(&user2).await.unwrap();
    assert!(
        matches!(get(&user2, "id"), Value::Int(_)),
        "The managed instance still has an identifier"
    );
}
