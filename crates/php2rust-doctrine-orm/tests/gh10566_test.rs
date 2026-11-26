//! Ticket/GH10566Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! A data provider's rows run in one test. `fetchOne()`'s `false` for
//! no row is `None`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10566Test.php

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

use php_value::{Object, Value};

use orm::*;
use ticket::*;

/// `provideEntityClasses()`.
const ENTITY_CLASSES: [&str; 3] = ["GH10566A", "GH10566B", "GH10566C"];

/// `$a->other = $b; $b->other = $c; $c->other = $a;`
fn cycle() -> [Object; 3] {
    let a = new("GH10566A");
    let b = new("GH10566B");
    let c = new("GH10566C");

    set_obj(&a, "other", &b);
    set_obj(&b, "other", &c);
    set_obj(&c, "other", &a);
    [a, b, c]
}

#[tokio::test]
async fn testInsertion() {
    for start_entity_class in ENTITY_CLASSES {
        let em = ticket::set_up(&ENTITY_CLASSES).await;
        let entities = cycle();

        for candidate in &entities {
            if candidate.instance_of(&class(start_entity_class)) {
                em.persist(candidate).unwrap();
            }
        }

        // Since all associations are nullable, the ORM has no problem finding an insert order,
        // it can always schedule "deferred updates" to fill missing foreign key values.
        em.flush().await.unwrap();

        for entity in &entities {
            assert!(!matches!(get(entity, "id"), Value::Null));
        }
    }
}

#[tokio::test]
async fn testRemoval() {
    for start_entity_class in ENTITY_CLASSES {
        let em = ticket::set_up(&ENTITY_CLASSES).await;
        let entities = cycle();

        em.persist(&entities[0]).unwrap();
        em.flush().await.unwrap();

        let ids: Vec<Value> = entities.iter().map(|e| get(e, "id")).collect();

        // Only A -> B can be used to break the cycle. So, regardless which
        // entity we start with, the ORM-level cascade will always remove all three entities.
        for candidate in &entities {
            if candidate.instance_of(&class(start_entity_class)) {
                em.remove(candidate).await.unwrap();
            }
        }

        em.flush().await.unwrap();

        for (table, id) in ["gh10566_a", "gh10566_b", "gh10566_c"].iter().zip(ids) {
            let found = em
                .get_connection()
                .fetch_one(
                    &format!("SELECT id FROM {table} WHERE id = ?"),
                    &php_value::PhpArray::list([id]),
                    &php2rust_doctrine_dbal::ParameterTypes::new(),
                )
                .await
                .unwrap();
            assert!(found.is_none(), "{start_entity_class}: {table}");
        }
    }
}
