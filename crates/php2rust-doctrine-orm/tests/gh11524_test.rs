//! Ticket/GH11524Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the Models/GH11524 models).
//!
//! `GH11524Listener` is an `EventListener`; the proxy's first method call
//! (`getTranslation()`) is `initialize_object()`, and the method is
//! written out in the test.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11524Test.php
//! Upstream: doctrine/orm tests/Tests/Models/GH11524/GH11524Listener.php
//! Upstream: doctrine/orm tests/Tests/Models/GH11524/GH11524Relation.php

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

use std::any::Any;
use std::sync::Arc;

use php_value::{Object, Thrown, Value};
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, ListenerError,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::event::LifecycleEventArgs;
use php2rust_doctrine_orm::events;

use orm::*;

const GH11524_ENTITY: &str = "Doctrine\\Tests\\Models\\GH11524\\GH11524Entity";
const GH11524_RELATION: &str = "Doctrine\\Tests\\Models\\GH11524\\GH11524Relation";

/// `GH11524Listener`.
struct GH11524Listener;

impl EventListener for GH11524Listener {
    fn call(&self, _event_name: &str, event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        let any: &dyn Any = event_args;
        let event_args = any
            .downcast_ref::<LifecycleEventArgs>()
            .expect("PostLoadEventArgs");
        let object = event_args.get_object();

        if !object.instance_of(GH11524_RELATION) {
            return Ok(());
        }

        // $object->setCurrentLocale('en')
        set(object, "currentLocale", s("en"));
        Ok(())
    }
}

/// `GH11524Relation::getTranslation()`.
async fn get_translation(em: &EntityManager, relation: &Object) -> Result<Value, Thrown> {
    em.initialize_object(relation).await?;
    if get(relation, "currentLocale").is_null() {
        return Err(Thrown::new(
            "LogicException",
            "The current locale must be set to retrieve translation.",
            0,
        ));
    }

    Ok(s("fake"))
}

#[tokio::test]
async fn testPostLoadCalledOnProxy() {
    let em = orm::set_up(&[GH11524_ENTITY, GH11524_RELATION]).await;
    let listener: Arc<dyn EventListener> = Arc::new(GH11524Listener);
    em.with_event_manager(|evm| {
        evm.add_event_listener(&[events::POST_LOAD], listener);
    });

    let relation = new_object(GH11524_RELATION);
    set(&relation, "name", s("test"));
    em.persist(&relation).unwrap();

    let entity = new_object(GH11524_ENTITY);
    set_obj(&entity, "relation", &relation);

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let reloaded_entity = em
        .find(GH11524_ENTITY, get(&entity, "id"))
        .await
        .unwrap()
        .unwrap();

    let reloaded_relation = get(&reloaded_entity, "relation");

    assert!(em.is_uninitialized_object(&reloaded_relation));

    assert_eq!(
        s("fake"),
        get_translation(&em, &obj(&reloaded_relation))
            .await
            .unwrap(),
        "The property set by the postLoad listener must get initialized on usage."
    );
}
