//! Ticket/DDC2790Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2790Test.php

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

use php_value::Value;
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, EventManager, ListenerError,
};
use php2rust_doctrine_orm::event::ManagerEventArgs;
use php2rust_doctrine_orm::events;

use orm::*;

/// `OnFlushListener`.
struct OnFlushListener;

impl EventListener for OnFlushListener {
    // onFLush listener that tries to cancel deletions by calling persist if the entity is listed
    // as updated in UoW
    fn call(&self, event_name: &str, event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        assert_eq!(events::ON_FLUSH, event_name);
        let args = (event_args as &dyn Any)
            .downcast_ref::<ManagerEventArgs>()
            .expect("OnFlushEventArgs");
        let em = args.get_object_manager();
        let uow = em.get_unit_of_work();
        let deletions = uow.get_scheduled_entity_deletions();
        let updates = uow.get_scheduled_entity_updates();

        // array_intersect_key($deletions, $updates): both are keyed by the
        // object id.
        let undelete = deletions
            .into_iter()
            .filter(|d| updates.iter().any(|u| same(d, u)));
        for d in undelete {
            em.persist(&d)?;
        }
        Ok(())
    }
}

async fn count(em: &Arc<php2rust_doctrine_orm::entity_manager::EntityManager>) -> Value {
    let mut qb = em.create_query_builder();
    qb.from(CMS_USER, "c", None).unwrap();
    qb.select(vec![s("count(c)")]).unwrap();
    qb.get_query()
        .unwrap()
        .get_single_scalar_result()
        .await
        .unwrap()
}

// Verifies that entities scheduled for deletion are not treated as updated by UoW,
// even if their properties are changed after the remove() call
#[tokio::test]
async fn testIssue() {
    let mut event_manager = EventManager::new();
    event_manager.add_event_listener(&[events::ON_FLUSH], Arc::new(OnFlushListener));
    let em = entity_manager(connection(), Some(event_manager));
    create_schema(&em, &CMS_MODELS).await;

    let entity = cms_user();
    set(&entity, "username", s("romanb"));
    set(&entity, "name", s("Roman"));

    // intval() of the count; assertEquals() compares loosely.
    let initial = count(&em).await;

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    em.remove(&entity).await.unwrap();
    // in Doctrine <2.5, this causes an UPDATE statement to be added before the DELETE statement
    // (and consequently also triggers preUpdate/postUpdate for the entity in question)
    set(&entity, "name", s("Robin"));

    em.flush().await.unwrap();

    let count = count(&em).await;
    assert_loose_eq(&initial, &count);
}
