//! Ticket/GH6029Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6029Test.php

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

use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

async fn set_up() -> Arc<EntityManager> {
    ticket::set_up(&[
        "GH6029User",
        "GH6029Group",
        "GH6029Group2",
        "GH6029Product",
        "GH6029Feature",
    ])
    .await
}

/// `$this->_em->persist($entity); $this->_em->flush();`.
async fn persist_and_flush(em: &EntityManager, entity: &Object) -> Result<(), Thrown> {
    em.persist(entity)?;
    em.flush().await
}

fn assert_invalid_argument(e: &Thrown, expected: &str) {
    assert_eq!(
        "Doctrine\\ORM\\ORMInvalidArgumentException", e.class,
        "{e:?}"
    );
    assert!(orm::message(e).contains(expected), "{e:?}");
}

/// Verifies that when wrong entity is persisted via relationship field, the error message does not correctly state
/// the expected class name.
#[tokio::test]
async fn testManyToManyAssociation() {
    let em = set_up().await;
    let user = new("GH6029User");
    set(
        &user,
        "groups",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    coll::add(&get(&user, "groups"), Value::Object(new("GH6029Group2"))).unwrap();

    let e = persist_and_flush(&em, &user).await.unwrap_err();
    assert_invalid_argument(
        &e,
        &format!(
            "Expected value of type \"{}\" for association field \"{}#$groups\", got \"{}\" instead.",
            class("GH6029Group"),
            class("GH6029User"),
            class("GH6029Group2"),
        ),
    );
}

/// Verifies that when wrong entity is persisted via relationship field, the error message does not correctly state
/// the expected class name.
#[tokio::test]
async fn testOneToManyAssociation() {
    let em = set_up().await;
    let product = new("GH6029Product");
    set(
        &product,
        "features",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    coll::add(
        &get(&product, "features"),
        Value::Object(new("GH6029Group2")),
    )
    .unwrap();

    let e = persist_and_flush(&em, &product).await.unwrap_err();
    assert_invalid_argument(
        &e,
        &format!(
            "Expected value of type \"{}\" for association field \"{}#$features\", got \"{}\" instead.",
            class("GH6029Feature"),
            class("GH6029Product"),
            class("GH6029Group2"),
        ),
    );
}
