//! Ticket/DDC2214Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//! `getLastLoggedQuery()['types']` is the types the last statement bound.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2214Test.php

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
#[path = "common/query_log.rs"]
mod query_log;
#[path = "common/ticket.rs"]
mod ticket;

use php_value::{Key, PhpArray, Value};
use php2rust_doctrine_dbal::ParameterType;
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let classes = [class("DDC2214Foo"), class("DDC2214Bar")];
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    let (em, log) = query_log::set_up_logged(&classes).await;
    // The functional test case logs every statement.
    log.reset_and_enable();

    let foo = new("DDC2214Foo");
    let bar = new("DDC2214Bar");

    set_obj(&foo, "bar", &bar);

    em.persist(&foo).unwrap();
    em.persist(&bar).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let foo = find(&em, "DDC2214Foo", get(&foo, "id")).await.unwrap();
    let bar = get(&foo, "bar");

    let query = em.create_query(&format!(
        "SELECT b FROM {NS}DDC2214Bar b WHERE b.id IN(:ids)"
    ));
    query.set_parameter("ids", Value::Array(PhpArray::list([bar])), None);
    query.get_result(HYDRATE_OBJECT).await.unwrap();

    assert_eq!(
        vec![(Key::Int(1), ParameterType::Integer)],
        log.last_types()
    );
}
