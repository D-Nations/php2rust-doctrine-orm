//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ManyToManyBidirectionalAssociationTest.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/AbstractManyToManyAssociationTestCase.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::similar_names,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use php_value::Value;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

async fn count(em: &EntityManager, first: Value, second: Value) -> usize {
    count_foreign_keys(
        em,
        "ecommerce_products_categories",
        "product_id",
        "category_id",
        first,
        second,
    )
    .await
}

#[tokio::test]
async fn testSavesAManyToManyAssociationWithCascadeSaveSet() {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = ecommerce_product("First Product");
    let first_category = ecommerce_category("Business");
    let second_category = ecommerce_category("Home");

    add_category(&first_product, &first_category).await;
    add_category(&first_product, &second_category).await;
    em.persist(&first_product).unwrap();
    em.flush().await.unwrap();

    assert_eq!(
        count(&em, get(&first_product, "id"), get(&first_category, "id")).await,
        1
    );
    assert_eq!(
        count(&em, get(&first_product, "id"), get(&second_category, "id")).await,
        1
    );
}

#[tokio::test]
async fn testRemovesAManyToManyAssociation() {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = ecommerce_product("First Product");
    let first_category = ecommerce_category("Business");
    let second_category = ecommerce_category("Home");

    add_category(&first_product, &first_category).await;
    add_category(&first_product, &second_category).await;
    em.persist(&first_product).unwrap();
    remove_category(&first_product, &first_category).await;
    em.flush().await.unwrap();

    assert_eq!(
        count(&em, get(&first_product, "id"), get(&first_category, "id")).await,
        0
    );
    assert_eq!(
        count(&em, get(&first_product, "id"), get(&second_category, "id")).await,
        1
    );

    coll::remove(&get(&first_product, "categories"), 1)
        .await
        .unwrap();
    em.flush().await.unwrap();

    assert_eq!(
        count(&em, get(&first_product, "id"), get(&second_category, "id")).await,
        0
    );
}

/// `setUp()` and `createLoadingFixture()`.
async fn create_loading_fixture() -> std::sync::Arc<EntityManager> {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = ecommerce_product("First Product");
    let second_product = ecommerce_product("Second Product");
    let first_category = ecommerce_category("Business");
    let second_category = ecommerce_category("Home");

    add_category(&first_product, &first_category).await;
    add_category(&first_product, &second_category).await;
    add_category(&second_product, &first_category).await;
    add_category(&second_product, &second_category).await;
    em.persist(&first_product).unwrap();
    em.persist(&second_product).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

fn collection(
    object: &php_value::Object,
    name: &str,
) -> std::sync::Arc<php2rust_doctrine_orm::persistent_collection::PersistentCollection> {
    php2rust_doctrine_orm::persistent_collection::PersistentCollection::from_value(&get(
        object, name,
    ))
    .expect("a persistent collection")
}

/// `assertCollectionEquals()`: every element of the first is in the
/// second.
async fn assert_collection_equals(
    first: &php_value::Object,
    second: &php_value::Object,
    name: &str,
) {
    for element in elements(first, name).await {
        assert!(contains(second, name, &element).await);
    }
}

async fn find_products(em: &std::sync::Arc<EntityManager>) -> Value {
    let query = em.create_query(
        "SELECT p, c FROM Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct p LEFT JOIN p.categories c ORDER BY p.id, c.id",
    );
    let result = query.get_object_result().await.unwrap();
    assert_eq!(list(&result).len(), 2);
    let first = obj(&at(&result, 0));
    let second = obj(&at(&result, 1));
    assert!(collection(&first, "categories").is_initialized());
    assert!(collection(&second, "categories").is_initialized());
    let cats1 = elements(&first, "categories").await;
    let cats2 = elements(&second, "categories").await;
    assert!(!collection(&cats1[0], "products").is_initialized());
    assert!(!collection(&cats2[0], "products").is_initialized());

    result
}

async fn find_categories(em: &std::sync::Arc<EntityManager>) -> Value {
    let query = em.create_query(
        "SELECT c, p FROM Doctrine\\Tests\\Models\\ECommerce\\ECommerceCategory c LEFT JOIN c.products p ORDER BY c.id, p.id",
    );
    let result = query.get_object_result().await.unwrap();
    assert_eq!(list(&result).len(), 2);
    let first = obj(&at(&result, 0));
    let second = obj(&at(&result, 1));
    assert_eq!(first.class(), ECOMMERCE_CATEGORY);
    assert_eq!(second.class(), ECOMMERCE_CATEGORY);
    assert!(collection(&first, "products").is_initialized());
    assert!(collection(&second, "products").is_initialized());

    let prods1 = elements(&first, "products").await;
    let prods2 = elements(&second, "products").await;
    assert!(!collection(&prods1[0], "categories").is_initialized());
    assert!(!collection(&prods2[0], "categories").is_initialized());

    result
}

/// `assertLazyLoadFromOwningSide()` and `assertLazyLoadFromInverseSide()`:
/// the collection `loaded` of the two owners, the collections `lazy` of
/// its elements, of `class`.
async fn assert_lazy_load(owners: &Value, loaded: &str, lazy: &str, class: &str) {
    let first_owner = obj(&at(owners, 0));
    let second_owner = obj(&at(owners, 1));

    assert_eq!(count_of(&first_owner, loaded).await, 2);
    assert_eq!(count_of(&second_owner, loaded).await, 2);

    let first_elements = elements(&first_owner, loaded).await;
    let second_elements = elements(&second_owner, loaded).await;
    assert!(same(&first_elements[0], &second_elements[0]));
    assert!(same(&first_elements[1], &second_elements[1]));

    let first_lazy = collection(&first_elements[0], lazy);
    let second_lazy = collection(&first_elements[1], lazy);

    assert!(!first_lazy.is_initialized());
    assert!(!second_lazy.is_initialized());
    assert_eq!(0, first_lazy.unwrap().len());
    assert_eq!(0, second_lazy.unwrap().len());

    assert_eq!(count_of(&first_elements[0], lazy).await, 2); // lazy-load
    assert!(first_lazy.is_initialized());
    assert!(!second_lazy.is_initialized());
    assert_eq!(count_of(&first_elements[1], lazy).await, 2); // lazy-load
    assert!(second_lazy.is_initialized());

    for element in elements(&first_elements[0], lazy)
        .await
        .iter()
        .chain(&elements(&first_elements[1], lazy).await)
    {
        assert_eq!(element.class(), class);
    }

    assert_collection_equals(&first_elements[0], &first_elements[1], lazy).await;
}

#[tokio::test]
async fn testEagerLoadFromInverseSideAndLazyLoadFromOwningSide() {
    let em = create_loading_fixture().await;
    let categories = find_categories(&em).await;
    assert_lazy_load(&categories, "products", "categories", ECOMMERCE_CATEGORY).await;
}

#[tokio::test]
async fn testEagerLoadFromOwningSideAndLazyLoadFromInverseSide() {
    let em = create_loading_fixture().await;
    let products = find_products(&em).await;
    assert_lazy_load(&products, "categories", "products", ECOMMERCE_PRODUCT).await;
}
