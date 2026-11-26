//! Ticket/DDC3644Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3644Test.php

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

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

async fn set_up() -> Arc<EntityManager> {
    ticket::set_up(&[
        "DDC3644User",
        "DDC3644Address",
        "DDC3644Animal",
        "DDC3644Pet",
    ])
    .await
}

/// `new DDC3644User()`.
fn user() -> Object {
    let user = new("DDC3644User");
    set(&user, "addresses", Value::Array(PhpArray::new()));
    set(&user, "pets", Value::Array(PhpArray::new()));
    user
}

/// `new DDC3644Address($address)`.
fn address(address: &str) -> Object {
    let object = new("DDC3644Address");
    set(&object, "address", s(address));
    object
}

/// `new DDC3644Pet($name)`.
fn pet(name: &str) -> Object {
    let pet = new("DDC3644Pet");
    set(&pet, "name", s(name));
    pet
}

/// `$user->setAddresses(new ArrayCollection($addresses))` (or
/// `setPets()`): the collection, with each element's owner set.
fn set_collection(user: &Object, field: &str, owner_field: &str, elements: &[&Object]) {
    let values = elements.iter().map(|e| Value::Object((*e).clone()));
    set(
        user,
        field,
        ArrayCollectionObject::value(PhpArray::list(values)),
    );
    for element in elements {
        set_obj(element, owner_field, user);
    }
}

#[tokio::test]
async fn testIssueWithRegularEntity() {
    let em = set_up().await;

    // Define initial dataset
    let current = address("Sao Paulo, SP, Brazil");
    let previous = address("Rio de Janeiro, RJ, Brazil");
    let initial = address("Sao Carlos, SP, Brazil");
    let user = user();

    set(&user, "name", s("Guilherme Blanco"));
    set_collection(&user, "addresses", "user", &[&current, &previous, &initial]);

    em.persist(&user).unwrap();
    em.persist(&current).unwrap();
    em.persist(&previous).unwrap();
    em.persist(&initial).unwrap();

    em.flush().await.unwrap();

    let user_id = get(&user, "id");

    em.clear().unwrap();

    // Replace entire collection (this should trigger OneToManyPersister::remove())
    let current = address("Toronto, ON, Canada");
    let user = find(&em, "DDC3644User", user_id.clone()).await.unwrap();

    set_collection(&user, "addresses", "user", &[&current]);

    em.persist(&user).unwrap();
    em.persist(&current).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    // We should only have 1 item in the collection list now
    let user = find(&em, "DDC3644User", user_id).await.unwrap();

    assert_eq!(1, count_of(&user, "addresses").await);

    // We should only have 1 item in the addresses table too
    let addresses = find_all(&em, "DDC3644Address").await;

    assert_eq!(1, addresses.len());
}

#[tokio::test]
async fn testIssueWithJoinedEntity() {
    let em = set_up().await;

    // Define initial dataset
    let actual = pet("Catharina");
    let past = pet("Nanny");
    let user = user();

    set(&user, "name", s("Guilherme Blanco"));
    set_collection(&user, "pets", "owner", &[&actual, &past]);

    em.persist(&user).unwrap();
    em.persist(&actual).unwrap();
    em.persist(&past).unwrap();

    em.flush().await.unwrap();

    let user_id = get(&user, "id");

    em.clear().unwrap();

    // Replace entire collection (this should trigger OneToManyPersister::remove())
    let actual = pet("Valentina");
    let user = find(&em, "DDC3644User", user_id.clone()).await.unwrap();

    set_collection(&user, "pets", "owner", &[&actual]);

    em.persist(&user).unwrap();
    em.persist(&actual).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    // We should only have 1 item in the collection list now
    let user = find(&em, "DDC3644User", user_id).await.unwrap();

    assert_eq!(1, count_of(&user, "pets").await);

    // We should only have 1 item in the pets table too
    let pets = find_all(&em, "DDC3644Pet").await;

    assert_eq!(1, pets.len());
}
