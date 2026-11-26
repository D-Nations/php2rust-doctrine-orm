//! Ticket/GH7661Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models, in `Doctrine\Tests\Functional\Ticket`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7661Test.php

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;

const NS: &str = "Doctrine\\Tests\\Functional\\Ticket\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// `new GH7661Participant($user, $event)`.
fn participant(user: &Object, event: &Object) -> Object {
    let participant = new_object(&class("GH7661Participant"));
    set_obj(&participant, "user", user);
    set_obj(&participant, "event", event);
    participant
}

#[tokio::test]
async fn testIndexByAssociation() {
    let classes = [
        class("GH7661User"),
        class("GH7661Event"),
        class("GH7661Participant"),
    ];
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    let em = orm::set_up(&classes).await;

    let u1 = new_object(&class("GH7661User"));
    let u2 = new_object(&class("GH7661User"));
    let e = new_object(&class("GH7661Event"));
    for entity in [&u1, &u2, &e, &participant(&u1, &e), &participant(&u2, &e)] {
        em.persist(entity).unwrap();
    }
    em.flush().await.unwrap();
    em.clear().unwrap();

    let e = em
        .find(&class("GH7661Event"), Value::Int(1))
        .await
        .unwrap()
        .unwrap();
    let participants = PersistentCollection::from_value(&get(&e, "participants"))
        .expect("a PersistentCollection");
    let keys = participants.get_keys().await.unwrap();
    assert_loose_eq(
        &Value::Array(php_value::PhpArray::list([Value::Int(1), Value::Int(2)])),
        &Value::Array(keys),
    );

    let participants = em
        .create_query(&format!(
            "SELECT p FROM {} p INDEX BY p.user",
            class("GH7661Participant")
        ))
        .get_object_result()
        .await
        .unwrap();
    let Value::Array(participants) = participants else {
        panic!("an array");
    };
    let keys: Vec<Value> = participants.keys().map(php_value::Key::to_value).collect();
    assert_loose_eq(
        &Value::Array(php_value::PhpArray::list([Value::Int(1), Value::Int(2)])),
        &Value::Array(php_value::PhpArray::list(keys)),
    );
}
