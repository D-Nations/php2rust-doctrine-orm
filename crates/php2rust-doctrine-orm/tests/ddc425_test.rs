//! Ticket/DDC425Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC425Test.php

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
use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC425Entity"]).await;

    let query = em.create_query(&format!(
        "DELETE {NS}DDC425Entity e WHERE e.someDatetimeField > ?1"
    ));
    query.set_parameter(
        1,
        DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
        Some("datetime".into()),
    );
    let num = query.get_result(HYDRATE_OBJECT).await.unwrap();
    assert_loose_eq(&Value::Int(0), &num);
}
