//! Ticket/GH7737Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7737Test.php

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
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

/// `new GH7737Group($id, $name)`.
fn group(id: i64, name: &str) -> Object {
    let group = new("GH7737Group");
    set(&group, "id", Value::Int(id));
    set(&group, "name", s(name));
    group
}

async fn set_up() -> Arc<EntityManager> {
    let em = ticket::set_up(&["GH7737Group", "GH7737Person"]).await;

    let group1 = group(1, "Test 1");
    // new GH7737Person(1)
    let person = new("GH7737Person");
    set(&person, "id", Value::Int(1));
    set(
        &person,
        "groups",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    coll::add(&get(&person, "groups"), Value::Object(group1.clone())).unwrap();

    em.persist(&person).unwrap();
    em.persist(&group1).unwrap();
    em.persist(&group(2, "Test 2")).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

#[tokio::test]
async fn memberOfCriteriaShouldBeCompatibleWithQueryBuilder() {
    let em = set_up().await;
    // `Criteria::ASC` is not defined in collections 3: `Criteria::create()`.
    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().member_of(":group", "person.groups"));
    let mut qb = em.create_query_builder();
    qb.select(vec![s("person")])
        .unwrap()
        .from(&class("GH7737Person"), "person", None)
        .unwrap()
        .add_criteria(&criteria)
        .unwrap();
    let query = qb.get_query().unwrap();

    let group1 = find(&em, "GH7737Group", Value::Int(1)).await.unwrap();
    query.set_parameter("group", Value::Object(group1), None);
    let matching = query.get_one_or_null_result(None).await.unwrap();

    assert_instance_of("GH7737Person", &matching);
    assert!(Value::Int(1).identical(&get(&obj(&matching), "id")));

    let group2 = find(&em, "GH7737Group", Value::Int(2)).await.unwrap();
    query.set_parameter("group", Value::Object(group2), None);
    let not_matching = query.get_one_or_null_result(None).await.unwrap();

    assert!(not_matching.is_null());
}
