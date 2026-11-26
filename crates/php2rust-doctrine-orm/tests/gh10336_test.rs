//! Ticket/GH10336Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the Models/GH10336 models).
//!
//! Reading `$entity->relation->value` on the detached proxy is
//! `initialize_object()` then the read.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10336Test.php

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

use php_value::Value;

use orm::*;

const ENTITY: &str = "Doctrine\\Tests\\Models\\GH10336\\GH10336Entity";
const RELATION: &str = "Doctrine\\Tests\\Models\\GH10336\\GH10336Relation";

#[tokio::test]
async fn testCanAccessRelationPropertyAfterClear() {
    let em = orm::set_up(&[ENTITY, RELATION]).await;

    let relation = new_object(RELATION);
    set(&relation, "value", s("foo"));

    let entity = new_object(ENTITY);
    set_obj(&entity, "relation", &relation);

    em.persist(&entity).unwrap();
    em.persist(&relation).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let entity = em.find(ENTITY, Value::Int(1)).await.unwrap().unwrap();
    em.clear().unwrap();

    let relation = obj(&get(&entity, "relation"));
    em.initialize_object(&relation).await.unwrap();
    assert_eq!(s("foo"), get(&relation, "value"));
}
