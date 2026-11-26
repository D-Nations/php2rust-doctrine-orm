//! Ticket/GH10049/GH10049Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10049/GH10049Test.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10049/ReadOnlyPropertyInheritor.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10049/ReadOnlyPropertyOwner.php

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

const READ_ONLY_PROPERTY_OWNER: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH10049\\ReadOnlyPropertyOwner";
const READ_ONLY_PROPERTY_INHERITOR: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH10049\\ReadOnlyPropertyInheritor";

#[tokio::test]
async fn testInheritedReadOnlyPropertyValueCanBeSet() {
    let em = orm::set_up(&[READ_ONLY_PROPERTY_OWNER, READ_ONLY_PROPERTY_INHERITOR]).await;
    let child = new_object(READ_ONLY_PROPERTY_INHERITOR);
    set(&child, "id", Value::Int(10049));
    em.persist(&child).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    em.find(READ_ONLY_PROPERTY_INHERITOR, Value::Int(10049))
        .await
        .unwrap();
}
