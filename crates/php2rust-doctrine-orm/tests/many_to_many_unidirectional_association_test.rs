//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ManyToManyUnidirectionalAssociationTest.php

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
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

async fn count(em: &EntityManager, first: Value, second: Value) -> usize {
    count_foreign_keys(
        em,
        "ecommerce_carts_products",
        "cart_id",
        "product_id",
        first,
        second,
    )
    .await
}

#[tokio::test]
async fn testSavesAManyToManyAssociationWithCascadeSaveSet() {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = ecommerce_product("Doctrine 1.x Manual");
    let second_product = ecommerce_product("Doctrine 2.x Manual");
    let first_cart = new_object(ECOMMERCE_CART);
    set(
        &first_cart,
        "products",
        php2rust_doctrine_orm::collection_object::ArrayCollectionObject::value(
            php_value::PhpArray::new(),
        ),
    );

    append(
        &first_cart,
        "products",
        Value::Object(first_product.clone()),
    );
    append(
        &first_cart,
        "products",
        Value::Object(second_product.clone()),
    );
    em.persist(&first_cart).unwrap();
    em.flush().await.unwrap();

    assert_eq!(
        count(&em, get(&first_cart, "id"), get(&first_product, "id")).await,
        1
    );
    assert_eq!(
        count(&em, get(&first_cart, "id"), get(&second_product, "id")).await,
        1
    );
}

#[tokio::test]
async fn testRemovesAManyToManyAssociation() {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = ecommerce_product("Doctrine 1.x Manual");
    let second_product = ecommerce_product("Doctrine 2.x Manual");
    let first_cart = new_object(ECOMMERCE_CART);
    set(
        &first_cart,
        "products",
        php2rust_doctrine_orm::collection_object::ArrayCollectionObject::value(
            php_value::PhpArray::new(),
        ),
    );

    append(
        &first_cart,
        "products",
        Value::Object(first_product.clone()),
    );
    append(
        &first_cart,
        "products",
        Value::Object(second_product.clone()),
    );
    em.persist(&first_cart).unwrap();
    remove_element(&first_cart, "products", &first_product).await;
    em.flush().await.unwrap();

    assert_eq!(
        count(&em, get(&first_cart, "id"), get(&first_product, "id")).await,
        0
    );
    assert_eq!(
        count(&em, get(&first_cart, "id"), get(&second_product, "id")).await,
        1
    );
}

/// `setUp()` and `createFixture()`: two carts with the same two products.
async fn create_fixture() -> std::sync::Arc<EntityManager> {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = ecommerce_product("Doctrine 1.x Manual");
    let second_product = ecommerce_product("Doctrine 2.x Manual");
    let first_cart = new_object(ECOMMERCE_CART);
    let second_cart = new_object(ECOMMERCE_CART);
    for cart in [&first_cart, &second_cart] {
        set(
            cart,
            "products",
            php2rust_doctrine_orm::collection_object::ArrayCollectionObject::value(
                php_value::PhpArray::new(),
            ),
        );
        append(cart, "products", Value::Object(first_product.clone()));
        append(cart, "products", Value::Object(second_product.clone()));
    }
    em.persist(&first_cart).unwrap();
    em.persist(&second_cart).unwrap();

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

async fn assert_carts(result: &Value) {
    let first_cart = obj(&at(result, 0));
    let products = elements(&first_cart, "products").await;
    let second_cart = obj(&at(result, 1));

    assert_eq!(products[0].class(), ECOMMERCE_PRODUCT);
    assert_eq!(products[1].class(), ECOMMERCE_PRODUCT);
    assert_collection_equals(&first_cart, &second_cart, "products").await;
}

#[tokio::test]
async fn testEagerLoad() {
    let em = create_fixture().await;

    let query = em.create_query(
        "SELECT c, p FROM Doctrine\\Tests\\Models\\ECommerce\\ECommerceCart c LEFT JOIN c.products p ORDER BY c.id, p.id",
    );
    let result = query.get_object_result().await.unwrap();
    assert_carts(&result).await;
}

#[tokio::test]
async fn testLazyLoadsCollection() {
    let em = create_fixture().await;
    em.get_class_metadata(ECOMMERCE_CART)
        .unwrap()
        .write()
        .association_mappings
        .get_mut("products")
        .unwrap()
        .fetch = Some(php2rust_doctrine_orm::mapping::class_metadata::FETCH_LAZY);

    let query =
        em.create_query("SELECT c FROM Doctrine\\Tests\\Models\\ECommerce\\ECommerceCart c");
    let result = query.get_object_result().await.unwrap();
    assert_carts(&result).await;
}
