//! Ticket/GH11149/GH11149Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models). `assertNotInstanceOf(Proxy::class)` is that the translation
//! is no uninitialized lazy object: native lazy objects have no proxy class.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11149/GH11149Test.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11149/EagerProduct.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11149/EagerProductTranslation.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11149/Locale.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11149\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

#[tokio::test]
async fn testFetchEagerModeWithIndexBy() {
    let em = orm::set_up(&[
        &class("Locale"),
        &class("EagerProduct"),
        &class("EagerProductTranslation"),
    ])
    .await;

    // Load entities into database
    let product = new_object(&class("EagerProduct"));
    set(&product, "id", Value::Int(11149));
    set(
        &product,
        "translations",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    em.persist(&product).unwrap();
    let locale = new_object(&class("Locale"));
    set(&locale, "code", s("fr_FR"));
    em.persist(&locale).unwrap();
    let translation = new_object(&class("EagerProductTranslation"));
    set(&translation, "id", Value::Int(11149));
    set_obj(&translation, "product", &product);
    set_obj(&translation, "locale", &locale);
    em.persist(&translation).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    // Fetch entity from database
    let product = em
        .find(&class("EagerProduct"), Value::Int(11149))
        .await
        .unwrap()
        .expect("an EagerProduct");

    // Assert associated entity is loaded eagerly
    assert!(product.instance_of(&class("EagerProduct")));
    let translations = get(&product, "translations");
    let collection =
        PersistentCollection::from_value(&translations).expect("a PersistentCollection");
    assert!(collection.is_initialized());
    assert_eq!(1, coll::count(&translations).await.unwrap());

    // Assert associated entity is indexed by given property
    let translation = coll::get(&translations, "fr_FR").await.unwrap();
    assert!(obj(&translation).instance_of(&class("EagerProductTranslation")));
    assert!(!em.is_uninitialized_object(&translation));
}
