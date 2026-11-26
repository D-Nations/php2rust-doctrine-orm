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
