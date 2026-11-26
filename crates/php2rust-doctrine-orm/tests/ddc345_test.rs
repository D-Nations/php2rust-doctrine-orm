//! Ticket/DDC345Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC345Test.php

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

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::object::register_method;

use orm::*;
use ticket::*;

/// The lifecycle callbacks of `DDC345Membership`.
fn register_callbacks() {
    // ++$this->prePersistCallCount; $this->updated = new DateTime();
    register_method(
        &class("DDC345Membership"),
        "doStuffOnPrePersist",
        Arc::new(|entity, _| {
            let Value::Int(count) = get(entity, "prePersistCallCount") else {
                panic!("an int");
            };
            set(entity, "prePersistCallCount", Value::Int(count + 1));
            set(
                entity,
                "updated",
                DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
            );
            Ok(())
        }),
    );
    // ++$this->preUpdateCallCount; $this->updated = new DateTime();
    register_method(
        &class("DDC345Membership"),
        "doStuffOnPreUpdate",
        Arc::new(|entity, _| {
            let Value::Int(count) = get(entity, "preUpdateCallCount") else {
                panic!("an int");
            };
            set(entity, "preUpdateCallCount", Value::Int(count + 1));
            set(
                entity,
                "updated",
                DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
            );
            Ok(())
        }),
    );
}

#[tokio::test]
async fn testTwoIterateHydrations() {
    let em = ticket::set_up(&["DDC345User", "DDC345Group", "DDC345Membership"]).await;
    register_callbacks();

    // Create User
    let user = new("DDC345User");
    set(
        &user,
        "memberships",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&user, "name", s("Test User"));
    em.persist(&user).unwrap(); // $em->flush() does not change much here

    // Create Group
    let group = new("DDC345Group");
    set(
        &group,
        "memberships",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&group, "name", s("Test Group"));
    em.persist(&group).unwrap(); // $em->flush() does not change much here

    let membership = new("DDC345Membership");
    set_obj(&membership, "group", &group);
    set_obj(&membership, "user", &user);
    set(&membership, "state", s("active"));

    coll::add(
        &get(&user, "memberships"),
        Value::Object(membership.clone()),
    )
    .unwrap();
    coll::add(
        &get(&group, "memberships"),
        Value::Object(membership.clone()),
    )
    .unwrap();

    em.flush().await.unwrap();

    assert_loose_eq(&Value::Int(1), &get(&membership, "prePersistCallCount"));
    assert_loose_eq(&Value::Int(0), &get(&membership, "preUpdateCallCount"));
    assert!(
        matches!(get(&membership, "updated"), Value::Object(o) if o.class() == "DateTime" || o.instance_of("DateTime"))
    );
}
