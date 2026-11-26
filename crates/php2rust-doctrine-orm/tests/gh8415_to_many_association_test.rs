//! Ticket/GH8415ToManyAssociationTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; upstream's `OrmTestCase` with `getTestEntityManager()`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH8415ToManyAssociationTest.php

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

use ticket::*;

#[tokio::test]
async fn testToManyAssociationOnBaseClassAllowedWhenThereAreMappedSuperclassesAsChildren() {
    let em = orm::entity_manager(orm::connection(), None);
    em.get_class_metadata(&class("GH8415ToManyLeafClass"))
        .unwrap();
}
