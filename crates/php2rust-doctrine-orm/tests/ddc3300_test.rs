//! Port of `DDC3300Test`, on `SQLite`: the listener's `addEventSubscriber()`
//! is `ResolveTargetEntityListener::subscribe()`, and `assertEquals()` of
//! two entities compares their class and properties.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3300Test.php

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

use std::sync::Arc;

use php_value::{Object, PhpArray};
use php2rust_doctrine_orm::tools::ResolveTargetEntityListener;

/// `assertEquals($expected, $actual)` of two entities.
fn assert_entity_equals(expected: &Object, actual: &Object, column: &str) {
    assert_eq!(expected.class(), actual.class());
    for property in ["id", column] {
        assert_eq!(orm::get(expected, property), orm::get(actual, property));
    }
}

#[tokio::test]
async fn testResolveTargetEntitiesChangesDiscriminatorMapValues() {
    models::register();
    let em = orm::entity_manager(orm::connection(), None);
    let resolve_target_entity = Arc::new(ResolveTargetEntityListener::new());

    resolve_target_entity.add_resolve_target_entity(
        &ticket::class("DDC3300Boss"),
        &ticket::class("DDC3300HumanBoss"),
        PhpArray::new(),
    );

    resolve_target_entity.add_resolve_target_entity(
        &ticket::class("DDC3300Employee"),
        &ticket::class("DDC3300HumanEmployee"),
        PhpArray::new(),
    );

    resolve_target_entity.subscribe(&em).unwrap();

    orm::create_schema(&em, &[&ticket::class("DDC3300Person")]).await;

    let boss = ticket::new("DDC3300HumanBoss");
    orm::set(&boss, "bossCol", orm::s("boss"));
    let employee = ticket::new("DDC3300HumanEmployee");
    orm::set(&employee, "employeeCol", orm::s("employee"));

    em.persist(&boss).unwrap();
    em.persist(&employee).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let found = ticket::find(&em, "DDC3300Boss", orm::get(&boss, "id"))
        .await
        .unwrap();
    assert_entity_equals(&boss, &found, "bossCol");
    let found = ticket::find(&em, "DDC3300Employee", orm::get(&employee, "id"))
        .await
        .unwrap();
    assert_entity_equals(&employee, &found, "employeeCol");
}
