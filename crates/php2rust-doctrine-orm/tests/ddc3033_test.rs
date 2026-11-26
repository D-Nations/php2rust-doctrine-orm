//! Ticket/DDC3033Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `postUpdate()` calls the async `computeChangeSet()` from the sync
//! callback by polling it once: it does not wait on the database here.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3033Test.php

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
use std::task::{Context, Poll, Waker};

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::event::LifecycleEventArgs;
use php2rust_doctrine_orm::object::register_method;

use orm::*;
use ticket::*;

/// `DDC3033Product::postUpdate($eventArgs)`.
fn register_callbacks() {
    register_method(
        &class("DDC3033Product"),
        "preUpdate",
        Arc::new(|_, _| Ok(())),
    );
    register_method(
        &class("DDC3033Product"),
        "postUpdate",
        Arc::new(|this, event_args| {
            let event_args = (event_args as &dyn Any)
                .downcast_ref::<LifecycleEventArgs>()
                .ok_or_else(|| Thrown::new("TypeError", "a PostUpdateEventArgs", 0))?;
            let em = event_args.get_object_manager();
            let uow = em.get_unit_of_work();
            let entity = event_args.get_object();
            let class_metadata = em.get_class_metadata(entity.class())?;
            let mut compute = uow.compute_change_set(&class_metadata, entity);
            match compute
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
            {
                Poll::Ready(result) => result?,
                Poll::Pending => {
                    return Err(Thrown::new("LogicException", "computeChangeSet() waits", 0));
                }
            }
            set(
                this,
                "changeSet",
                Value::Array(uow.get_entity_change_set(entity)),
            );
            Ok(())
        }),
    );
}

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC3033User", "DDC3033Product"]).await;
    register_callbacks();

    let user = new("DDC3033User");
    set(&user, "name", s("Test User"));
    em.persist(&user).unwrap();

    let user2 = new("DDC3033User");
    set(&user2, "name", s("Test User 2"));
    em.persist(&user2).unwrap();

    let product = new("DDC3033Product");
    set(&product, "changeSet", Value::Array(PhpArray::new()));
    set(
        &product,
        "buyers",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&product, "title", s("Test product"));
    append(&product, "buyers", Value::Object(user.clone()));

    em.persist(&product).unwrap();
    em.flush().await.unwrap();

    set(&product, "title", s("Test Change title"));
    append(&product, "buyers", Value::Object(user2.clone()));

    em.persist(&product).unwrap();
    em.flush().await.unwrap();

    let expect = array(&[(
        "title",
        Value::Array(PhpArray::list([s("Test product"), s("Test Change title")])),
    )]);

    assert_loose_eq(&expect, &get(&product, "changeSet"));
}
