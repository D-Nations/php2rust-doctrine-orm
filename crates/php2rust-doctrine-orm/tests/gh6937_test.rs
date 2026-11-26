//! Ticket/GH6937Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6937Test.php

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_SIMPLEOBJECT;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

/// `setUp()` and the persisted manager's id.
async fn set_up_with_manager() -> (Arc<EntityManager>, Value) {
    let em = ticket::set_up(&["GH6937Person", "GH6937Employee", "GH6937Manager"]).await;

    let manager = new("GH6937Manager");
    set(&manager, "name", s("Kevin"));
    set(&manager, "phoneNumber", s("555-5555"));
    set(&manager, "department", s("Accounting"));

    em.persist(&manager).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let id = get(&manager, "id");
    (em, id)
}

fn assert_manager(persisted_manager: &Object) {
    assert!(s("Kevin").identical(&get(persisted_manager, "name")));
    assert!(s("555-5555").identical(&get(persisted_manager, "phoneNumber")));
    assert!(s("Accounting").identical(&get(persisted_manager, "department")));
}

/// `getRepository(GH6937Person::class)->createQueryBuilder('e')->where('e.id = :id')`.
async fn query_by_id(em: &Arc<EntityManager>, id: Value, hydration_mode: Option<i64>) -> Object {
    let repository = em.get_repository(&class("GH6937Person")).unwrap();
    let mut qb = repository.create_query_builder("e", None).unwrap();
    qb.r#where(vec![s("e.id = :id")])
        .unwrap()
        .set_parameter("id", id, None);
    obj(&qb
        .get_query()
        .unwrap()
        .get_one_or_null_result(hydration_mode)
        .await
        .unwrap())
}

#[tokio::test]
async fn testPhoneNumberIsPopulatedWithFind() {
    let (em, id) = set_up_with_manager().await;

    let persisted_manager = find(&em, "GH6937Person", id).await.unwrap();

    assert_manager(&persisted_manager);
}

#[tokio::test]
async fn testPhoneNumberIsPopulatedWithQueryBuilderUsingSimpleObjectHydrator() {
    let (em, id) = set_up_with_manager().await;

    let persisted_manager = query_by_id(&em, id, Some(HYDRATE_SIMPLEOBJECT)).await;

    assert_manager(&persisted_manager);
}

#[tokio::test]
async fn testPhoneNumberIsPopulatedWithQueryBuilder() {
    let (em, id) = set_up_with_manager().await;

    let persisted_manager = query_by_id(&em, id, None).await;

    assert_manager(&persisted_manager);
}
