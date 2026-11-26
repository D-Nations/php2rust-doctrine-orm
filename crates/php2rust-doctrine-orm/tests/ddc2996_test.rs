//! Ticket/DDC2996Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//! `preFlush()`'s body is declared with `register_method()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2996Test.php

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

use std::any::Any;
use std::sync::Arc;

use php_value::Value;
use php2rust_doctrine_orm::event::ManagerEventArgs;
use php2rust_doctrine_orm::object::register_method;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC2996User", "DDC2996UserPreference"]).await;
    // DDC2996UserPreference::preFlush($event)
    register_method(
        &class("DDC2996UserPreference"),
        "preFlush",
        Arc::new(|this, event| {
            let event = (event as &dyn Any)
                .downcast_ref::<ManagerEventArgs>()
                .expect("PreFlushEventArgs");
            let em = event.get_object_manager();
            let uow = em.get_unit_of_work();
            let user = obj(&get(this, "user"));
            if !uow.get_original_entity_data(&user).is_empty() {
                // $this->user->counter++
                let counter = match get(&user, "counter") {
                    Value::Int(counter) => counter,
                    _ => 0,
                };
                set(&user, "counter", Value::Int(counter + 1));
                uow.recompute_single_entity_change_set(
                    &em.get_class_metadata(user.class())?,
                    &user,
                )?;
            }
            Ok(())
        }),
    );

    let pref = new("DDC2996UserPreference");
    set_obj(&pref, "user", &new("DDC2996User"));
    set(&pref, "value", s("foo"));

    em.persist(&pref).unwrap();
    em.persist(&obj(&get(&pref, "user"))).unwrap();
    em.flush().await.unwrap();

    set(&pref, "value", s("bar"));
    em.flush().await.unwrap();

    assert_loose_eq(&Value::Int(1), &get(&obj(&get(&pref, "user")), "counter"));

    em.clear().unwrap();

    let pref = find(&em, "DDC2996UserPreference", get(&pref, "id"))
        .await
        .unwrap();
    let user = obj(&get(&pref, "user"));
    em.initialize_object(&user).await.unwrap();
    assert_loose_eq(&Value::Int(1), &get(&user, "counter"));
}
