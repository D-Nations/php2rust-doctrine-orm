//! Ticket/GH10752Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream's `TestUtil` adds DBAL's `EnableForeignKeys` middleware on
//! `SQLite`; the DBAL port has no middlewares, so the test runs its
//! statement, `PRAGMA foreign_keys = ON`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10752Test.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

const FOREIGN_KEY_CONSTRAINT_VIOLATION: &str =
    "Doctrine\\DBAL\\Exception\\ForeignKeyConstraintViolationException";

async fn set_up() -> Arc<EntityManager> {
    let em = ticket::set_up(&["GH10752Order", "GH10752Promotion"]).await;
    em.get_connection()
        .execute_statement(
            "PRAGMA foreign_keys = ON",
            &PhpArray::new(),
            &php2rust_doctrine_dbal::ParameterTypes::new(),
        )
        .await
        .unwrap();
    em
}

/// An order with a promotion, persisted.
async fn order_with_promotion(em: &EntityManager) -> Object {
    let order = new("GH10752Order");
    set(
        &order,
        "promotions",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    let promotion = new("GH10752Promotion");
    // GH10752Order::addPromotion()
    let promotions = get(&order, "promotions");
    if !coll::contains(&promotions, &Value::Object(promotion.clone()))
        .await
        .unwrap()
    {
        coll::add(&promotions, Value::Object(promotion.clone())).unwrap();
    }

    em.persist(&order).unwrap();
    em.persist(&promotion).unwrap();
    em.flush().await.unwrap();
    promotion
}

#[tokio::test]
async fn testThrowExceptionWhenRemovingPromotionThatIsInUse() {
    let em = set_up().await;
    let promotion = order_with_promotion(&em).await;

    em.remove(&promotion).await.unwrap();

    let error = em
        .flush()
        .await
        .expect_err("a ForeignKeyConstraintViolationException");
    assert_eq!(FOREIGN_KEY_CONSTRAINT_VIOLATION, error.class);
}

#[tokio::test]
async fn testThrowExceptionWhenRemovingPromotionThatIsInUseAndOrderIsNotInMemory() {
    let em = set_up().await;
    let promotion = order_with_promotion(&em).await;
    em.clear().unwrap();

    let promotion = find(&em, "GH10752Promotion", get(&promotion, "id"))
        .await
        .unwrap();
    em.remove(&promotion).await.unwrap();

    let error = em
        .flush()
        .await
        .expect_err("a ForeignKeyConstraintViolationException");
    assert_eq!(FOREIGN_KEY_CONSTRAINT_VIOLATION, error.class);
}
