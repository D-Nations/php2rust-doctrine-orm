//! Ticket/DDC2074Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the ecommerce models).
//! `clone $product` copies the properties and runs
//! `ECommerceProduct::__clone()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2074Test.php

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

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_collections::ArrayCollection;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;

/// `new ECommerceProduct()`.
fn product() -> Object {
    let product = ecommerce_product("");
    set(&product, "name", Value::Null);
    product
}

/// `clone $product`: the properties, then `ECommerceProduct::__clone()`.
async fn clone_product(product: &Object) -> Object {
    let clone = new_object(product.class());
    for (key, value) in product.properties().iter() {
        set(
            &clone,
            &String::from_utf8_lossy(&key.bytes()),
            value.clone(),
        );
    }
    set(&clone, "isCloned", Value::Bool(true));
    let categories = get(&clone, "categories");
    if categories.is_truthy() {
        let copy = PersistentCollection::from_value(&categories)
            .unwrap()
            .clone_collection()
            .await
            .unwrap();
        set(&clone, "categories", copy.value());
    }
    clone
}

#[tokio::test]
async fn testShouldNotScheduleDeletionOnClonedInstances() {
    let em: Arc<EntityManager> = orm::set_up(&ECOMMERCE_MODELS).await;
    let class = em.get_class_metadata(ECOMMERCE_PRODUCT).unwrap();
    let product = product();
    let category = ecommerce_category("");
    set(&category, "name", Value::Null);
    let collection = PersistentCollection::new(
        Some(&em),
        Some(class.clone()),
        ArrayCollection::new(PhpArray::list([Value::Object(category)])),
    );
    let assoc = class
        .read()
        .association_mappings
        .get("categories")
        .unwrap()
        .clone();
    collection.set_owner(&product, &assoc);
    let uow = em.get_unit_of_work();

    let cloned_collection = collection.clone_collection().await.unwrap();
    coll::clear(&cloned_collection.value()).await.unwrap();

    assert_eq!(0, uow.get_scheduled_collection_deletions().len());
}

#[tokio::test]
async fn testSavingClonedPersistentCollection() {
    let em = orm::set_up(&ECOMMERCE_MODELS).await;

    let product = product();
    let category = ecommerce_category("foo");

    add_category(&product, &category).await;

    em.persist(&product).unwrap();
    em.persist(&category).unwrap();
    em.flush().await.unwrap();

    let new_product = clone_product(&product).await;

    em.persist(&new_product).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let product1 = em
        .find(ECOMMERCE_PRODUCT, get(&product, "id"))
        .await
        .unwrap()
        .unwrap();
    let product2 = em
        .find(ECOMMERCE_PRODUCT, get(&new_product, "id"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(1, count_of(&product1, "categories").await);
    assert_eq!(1, count_of(&product2, "categories").await);

    let category1 = coll::get(&get(&product1, "categories"), 0).await.unwrap();
    let category2 = coll::get(&get(&product2, "categories"), 0).await.unwrap();
    assert!(same_value(&category1, &obj(&category2)));
}
