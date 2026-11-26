//! Ticket/DDC2775Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2775Test.php

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

use php_value::Value;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssueCascadeRemove() {
    let em = ticket::set_up(&["User", "Role", "AdminRole", "Authorization"]).await;

    let role = new("AdminRole");
    let user = new("User");
    // User::addRole()
    append(&user, "roles", Value::Object(role.clone()));
    set_obj(&role, "user", &user);

    let authorization = new("Authorization");
    // User::addAuthorization()
    append(
        &user,
        "authorizations",
        Value::Object(authorization.clone()),
    );
    set_obj(&authorization, "user", &user);
    // Role::addAuthorization()
    append(
        &role,
        "authorizations",
        Value::Object(authorization.clone()),
    );
    set_obj(&authorization, "role", &role);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    // Need to clear so that associations are lazy-loaded
    em.clear().unwrap();

    let user = find(&em, "User", get(&user, "id")).await.unwrap();

    em.remove(&user).await.unwrap();
    em.flush().await.unwrap();

    assert!(find_all(&em, "Authorization").await.is_empty());

    // With the bug, the second flush throws an error because the cascade remove didn't work correctly
    em.flush().await.unwrap();
}
