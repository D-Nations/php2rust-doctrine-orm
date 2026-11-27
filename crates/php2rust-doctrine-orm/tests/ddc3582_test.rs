//! Ticket/DDC3582Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3582Test.php

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

use orm::{get, get_object, s, set};
use ticket::*;

#[tokio::test]
async fn testNestedEmbeddablesAreHydratedWithProperClass() {
    let em = set_up(&["DDC3582Entity"]).await;

    // new DDC3582Entity('foo'): its constructors make the embeddables.
    let embeddable3 = new("DDC3582Embeddable3");
    let embeddable2 = new("DDC3582Embeddable2");
    set(&embeddable2, "embeddable3", Value::Object(embeddable3));
    let embeddable1 = new("DDC3582Embeddable1");
    set(&embeddable1, "embeddable2", Value::Object(embeddable2));
    let entity = new("DDC3582Entity");
    set(&entity, "id", s("foo"));
    set(&entity, "embeddable1", Value::Object(embeddable1));

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let entity = find(&em, "DDC3582Entity", s("foo")).await.unwrap();
    assert_instance_of("DDC3582Entity", &Value::Object(entity.clone()));
    let embeddable1 = get_object(&entity, "embeddable1");
    assert_instance_of("DDC3582Embeddable1", &get(&entity, "embeddable1"));
    let embeddable2 = get_object(embeddable1.as_ref().unwrap(), "embeddable2");
    assert_instance_of(
        "DDC3582Embeddable2",
        &get(embeddable1.as_ref().unwrap(), "embeddable2"),
    );
    assert_instance_of(
        "DDC3582Embeddable3",
        &get(embeddable2.as_ref().unwrap(), "embeddable3"),
    );
}
