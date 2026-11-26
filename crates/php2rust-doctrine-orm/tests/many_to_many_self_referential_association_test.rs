//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ManyToManySelfReferentialAssociationTest.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

async fn count(em: &EntityManager, first: Value, second: Value) -> usize {
    count_foreign_keys(
        em,
        "ecommerce_products_related",
        "product_id",
        "related_id",
        first,
        second,
    )
    .await
}

fn unnamed_product() -> php_value::Object {
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

#[tokio::test]
async fn testSavesAManyToManyAssociationWithCascadeSaveSet() {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = unnamed_product();
    let first_related = ecommerce_product("Business");
    let second_related = ecommerce_product("Home");

    add_related(&first_product, &first_related).await;
    add_related(&first_product, &second_related).await;
    em.persist(&first_product).unwrap();
    em.flush().await.unwrap();

    assert_eq!(
        count(&em, get(&first_product, "id"), get(&first_related, "id")).await,
        1
    );
    assert_eq!(
        count(&em, get(&first_product, "id"), get(&second_related, "id")).await,
        1
    );
}

#[tokio::test]
async fn testRemovesAManyToManyAssociation() {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = unnamed_product();
    let first_related = ecommerce_product("Business");
    let second_related = ecommerce_product("Home");

    add_related(&first_product, &first_related).await;
    add_related(&first_product, &second_related).await;
    em.persist(&first_product).unwrap();
    remove_related(&first_product, &first_related).await;
    em.flush().await.unwrap();

    assert_eq!(
        count(&em, get(&first_product, "id"), get(&first_related, "id")).await,
        0
    );
    assert_eq!(
        count(&em, get(&first_product, "id"), get(&second_related, "id")).await,
        1
    );
}

/// `setUp()` and `createLoadingFixture()`.
async fn create_loading_fixture() -> std::sync::Arc<EntityManager> {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = unnamed_product();
    let second_product = unnamed_product();
    let first_related = ecommerce_product("Business");
    let second_related = ecommerce_product("Home");

    add_related(&first_product, &first_related).await;
    add_related(&first_product, &second_related).await;
    add_related(&second_product, &first_related).await;
    add_related(&second_product, &second_related).await;
    em.persist(&first_product).unwrap();
    em.persist(&second_product).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
    em
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

async fn assert_loading_of_owning_side(products: &Value) {
    let first_product = obj(&at(products, 0));
    let second_product = obj(&at(products, 1));
    assert_eq!(count_of(&first_product, "related").await, 2);
    assert_eq!(count_of(&second_product, "related").await, 2);

    let categories = elements(&first_product, "related").await;
    let first_related_by = elements(&categories[0], "related").await;
    let second_related_by = elements(&categories[1], "related").await;

    assert_eq!(first_related_by.len(), 2);
    assert_eq!(second_related_by.len(), 2);

    for product in first_related_by.iter().chain(&second_related_by) {
        assert_eq!(product.class(), ECOMMERCE_PRODUCT);
    }

    assert_collection_equals(&categories[0], &categories[1], "related").await;
}

#[tokio::test]
async fn testEagerLoadsOwningSide() {
    let em = create_loading_fixture().await;
    // findProducts()
    let query = em.create_query(
        "SELECT p, r FROM Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct p LEFT JOIN p.related r ORDER BY p.id, r.id",
    );
    let products = query.get_object_result().await.unwrap();
    assert_loading_of_owning_side(&products).await;
}

#[tokio::test]
async fn testLazyLoadsOwningSide() {
    let em = create_loading_fixture().await;

    em.get_class_metadata(ECOMMERCE_PRODUCT)
        .unwrap()
        .write()
        .association_mappings
        .get_mut("related")
        .unwrap()
        .fetch = Some(php2rust_doctrine_orm::mapping::class_metadata::FETCH_LAZY);

    let query =
        em.create_query("SELECT p FROM Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct p");
    let products = query.get_object_result().await.unwrap();
    assert_loading_of_owning_side(&products).await;
}
