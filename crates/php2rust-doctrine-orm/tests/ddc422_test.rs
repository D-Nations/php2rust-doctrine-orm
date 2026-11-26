//! Ticket/DDC422Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC422Test.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC422Guest", "DDC422Customer", "DDC422Contact"]).await;

    let customer = new("DDC422Customer");
    set(
        &customer,
        "contacts",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    em.persist(&customer).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let customer = find(&em, "DDC422Customer", get(&customer, "id"))
        .await
        .unwrap();

    let contacts = get(&customer, "contacts");
    let collection = PersistentCollection::from_value(&contacts).unwrap();
    assert!(!collection.is_initialized());
    let contact = new("DDC422Contact");
    coll::add(&contacts, Value::Object(contact)).unwrap();
    assert!(collection.is_dirty());
    assert!(!collection.is_initialized());
    em.flush().await.unwrap();

    assert_loose_eq(
        &Value::Int(1),
        &fetch_one(&em, "select count(*) from ddc422_customers_contacts").await,
    );
}
