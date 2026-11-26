//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ManyToManyBidirectionalAssociationTest.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/AbstractManyToManyAssociationTestCase.php

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use php_value::Value;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

async fn count(em: &EntityManager, first: Value, second: Value) -> usize {
    count_foreign_keys(em, "ecommerce_products_categories", "product_id", "category_id", first, second).await
}

#[tokio::test]
async fn test_saves_a_many_to_many_association_with_cascade_save_set() {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = ecommerce_product("First Product");
    let first_category = ecommerce_category("Business");
    let second_category = ecommerce_category("Home");

    add_category(&first_product, &first_category).await;
    add_category(&first_product, &second_category).await;
    em.persist(&first_product).unwrap();
    em.flush().await.unwrap();

    assert_eq!(count(&em, get(&first_product, "id"), get(&first_category, "id")).await, 1);
    assert_eq!(count(&em, get(&first_product, "id"), get(&second_category, "id")).await, 1);
}

#[tokio::test]
async fn test_removes_a_many_to_many_association() {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = ecommerce_product("First Product");
    let first_category = ecommerce_category("Business");
    let second_category = ecommerce_category("Home");

    add_category(&first_product, &first_category).await;
    add_category(&first_product, &second_category).await;
    em.persist(&first_product).unwrap();
    remove_category(&first_product, &first_category).await;
    em.flush().await.unwrap();

    assert_eq!(count(&em, get(&first_product, "id"), get(&first_category, "id")).await, 0);
    assert_eq!(count(&em, get(&first_product, "id"), get(&second_category, "id")).await, 1);

    coll::remove(&get(&first_product, "categories"), 1).await.unwrap();
    em.flush().await.unwrap();

    assert_eq!(count(&em, get(&first_product, "id"), get(&second_category, "id")).await, 0);
}
