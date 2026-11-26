//! Ticket/DDC163Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the company models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC163Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/company.rs"]
mod company;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php2rust_doctrine_orm::entity_manager::EntityManager;

async fn set_up() -> Arc<EntityManager> {
    company::register_listeners();
    orm::set_up(&company::COMPANY_MODELS).await
}

use php_value::Value;

use company::{add_friend, company_person, set_spouse};
use orm::{assert_loose_eq, s};

#[tokio::test]
async fn testQueryWithOrConditionUsingTwoRelationOnSameEntity() {
    let em = set_up().await;
    let p1 = company_person("p1");

    let p2 = company_person("p2");

    let p3 = company_person("p3");

    let p4 = company_person("p4");

    set_spouse(&p1, &p3);
    add_friend(&p1, &p2).await;
    add_friend(&p2, &p3).await;

    add_friend(&p3, &p4).await;

    em.persist(&p1).unwrap();
    em.persist(&p2).unwrap();
    em.persist(&p3).unwrap();
    em.persist(&p4).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql = "SELECT PARTIAL person.{id,name}, PARTIAL spouse.{id,name}, PARTIAL friend.{id,name}
            FROM  Doctrine\\Tests\\Models\\Company\\CompanyPerson person
            LEFT JOIN person.spouse spouse
            LEFT JOIN person.friends friend
            LEFT JOIN spouse.friends spouse_friend
            LEFT JOIN friend.friends friend_friend
            WHERE person.name=:name AND (spouse_friend.name=:name2 OR friend_friend.name=:name2)";

    let q = em.create_query(dql);
    q.set_parameter("name", s("p1"), None);
    q.set_parameter("name2", s("p4"), None);
    let result = q.get_scalar_result().await.unwrap();

    let Value::Array(result) = result else {
        panic!("a list: {result:?}");
    };
    let Some(Value::Array(row)) = result.values().next() else {
        panic!("a row: {result:?}");
    };
    assert_loose_eq(
        &s("p3"),
        &row.get(&"spouse_name".into()).cloned().unwrap_or_default(),
    );
    assert_loose_eq(
        &s("p1"),
        &row.get(&"person_name".into()).cloned().unwrap_or_default(),
    );
    assert_loose_eq(
        &s("p2"),
        &row.get(&"friend_name".into()).cloned().unwrap_or_default(),
    );
}
