//! Port of `StandardEntityPersisterTest`, on `SQLite` with the `ecommerce`
//! and `cms` model sets.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/StandardEntityPersisterTest.php

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

use php_value::{Object, PhpArray};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::unit_of_work::Hints;

use orm::{
    CMS_MODELS, ECOMMERCE_CART, ECOMMERCE_CUSTOMER, ECOMMERCE_MODELS, ECOMMERCE_PRODUCT,
    add_feature, assert_loose_eq, count_of, ecommerce_feature, elements, get, list, new_object,
    obj, s, same, set, set_obj,
};

/// `new ECommerceProduct()`.
fn new_ecommerce_product() -> Object {
    let product = new_object(ECOMMERCE_PRODUCT);
    for collection in ["features", "categories", "related"] {
        set(
            &product,
            collection,
            ArrayCollectionObject::value(PhpArray::new()),
        );
    }
    product
}

async fn set_up() -> Arc<EntityManager> {
    let classes: Vec<&str> = ECOMMERCE_MODELS
        .iter()
        .chain(CMS_MODELS.iter())
        .copied()
        .collect();
    orm::set_up(&classes).await
}

#[tokio::test]
async fn testAcceptsForeignKeysAsCriteria() {
    let em = set_up().await;
    let customer = new_object(ECOMMERCE_CUSTOMER);
    set(&customer, "name", s("John Doe"));
    let cart = new_object(ECOMMERCE_CART);
    set(&cart, "payment", s("Credit card"));
    // $customer->setCart($cart)
    set_obj(&customer, "cart", &cart);
    set_obj(&cart, "customer", &customer);
    em.persist(&customer).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    let card_id = get(&cart, "id");
    drop(cart);

    let class = em.get_class_metadata(ECOMMERCE_CART).unwrap();

    let persister = em
        .get_unit_of_work()
        .get_entity_persister(ECOMMERCE_CART)
        .unwrap();
    let new_cart = new_object(ECOMMERCE_CART);
    let mut id = PhpArray::new();
    id.insert("id", card_id);
    em.get_unit_of_work()
        .register_managed(&new_cart, id, PhpArray::new())
        .unwrap();
    let mut criteria = PhpArray::new();
    criteria.insert("customer_id", get(&customer, "id"));
    let customer_mapping = class
        .read()
        .association_mappings
        .get("customer")
        .unwrap()
        .clone();
    persister
        .load(
            &criteria,
            Some(&new_cart),
            Some(&customer_mapping),
            Hints::default(),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_loose_eq(&s("Credit card"), &get(&new_cart, "payment"));
}

/// Ticket #2478 from Damon Jones (dljones)
#[tokio::test]
async fn testAddPersistRetrieve() {
    let em = set_up().await;
    let f1 = ecommerce_feature("AC-3");
    let f2 = ecommerce_feature("DTS");

    let p = new_ecommerce_product();
    add_feature(&p, &f1);
    add_feature(&p, &f2);
    em.persist(&p).unwrap();

    em.flush().await.unwrap();

    assert_eq!(count_of(&p, "features").await, 2);
    assert!(PersistentCollection::from_value(&get(&p, "features")).is_some());

    let q = em.create_query(
        "SELECT p, f
               FROM Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct p
               JOIN p.features f",
    );

    q.get_result(HYDRATE_OBJECT).await.unwrap();

    assert_eq!(count_of(&p, "features").await, 2);
    assert!(PersistentCollection::from_value(&get(&p, "features")).is_some());

    // Check that the features are the same instances still
    for feature in elements(&p, "features").await {
        if get(&feature, "description").identical(&s("AC-3")) {
            assert!(same(&feature, &f1));
        } else {
            assert!(same(&feature, &f2));
        }
    }

    // Now we test how Hydrator affects IdentityMap
    // (change from ArrayCollection to PersistentCollection)
    let f3 = ecommerce_feature("XVID");
    add_feature(&p, &f3);

    // Now we persist the Feature #3
    em.persist(&p).unwrap();
    em.flush().await.unwrap();

    let q = em.create_query(
        "SELECT p, f
               FROM Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct p
               JOIN p.features f",
    );

    let res = list(&q.get_result(HYDRATE_OBJECT).await.unwrap());

    // Persisted Product now must have 3 Feature items
    assert_eq!(count_of(&obj(&res[0]), "features").await, 3);
}
