//! Ticket/GH10889Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10889Test.php

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

use php_value::Value;
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["GH10889Person", "GH10889Company", "GH10889Resume"]).await;

    let person = new("GH10889Person");
    // new GH10889Resume($person, null)
    let resume = new("GH10889Resume");
    set_obj(&resume, "person", &person);
    set(&resume, "currentCompany", Value::Null);

    em.persist(&person).unwrap();
    em.persist(&resume).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let mut qb = em
        .get_repository(&class("GH10889Resume"))
        .unwrap()
        .create_query_builder("resume", None)
        .unwrap();
    qb.left_join("resume.currentCompany", "company", None, None, None)
        .unwrap()
        .add_select(vec![s("company")])
        .unwrap();
    let resumes = list(
        &qb.get_query()
            .unwrap()
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    );

    assert!(!resumes.is_empty());
    let resume = obj(&resumes[0]);
    assert_loose_eq(&Value::Int(1), &get(&obj(&get(&resume, "person")), "id"));
    assert!(matches!(get(&resume, "currentCompany"), Value::Null));
}
