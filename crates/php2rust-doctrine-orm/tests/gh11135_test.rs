//! Ticket/GH11135Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11135Test.php

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
async fn testOverrideInheritsDeclaringClass() {
    let em = set_up(&[
        "GH11135MappedSuperclass",
        "GH11135EntityWithOverride",
        "GH11135EntityWithoutOverride",
    ])
    .await;

    let cm1 = em
        .get_class_metadata(&class("GH11135EntityWithOverride"))
        .unwrap();
    let cm1 = cm1.read();
    let cm2 = em
        .get_class_metadata(&class("GH11135EntityWithoutOverride"))
        .unwrap();
    let cm2 = cm2.read();

    assert_eq!(
        cm1.get_field_mapping("id").unwrap().declared,
        cm2.get_field_mapping("id").unwrap().declared
    );
    assert_eq!(
        cm1.get_association_mapping("ref").unwrap().declared,
        cm2.get_association_mapping("ref").unwrap().declared
    );
}
