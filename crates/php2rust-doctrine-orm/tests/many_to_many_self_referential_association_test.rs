//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ManyToManySelfReferentialAssociationTest.php

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

async fn count(em: &EntityManager, first: Value, second: Value) -> usize {
    count_foreign_keys(em, "ecommerce_products_related", "product_id", "related_id", first, second).await
}

fn unnamed_product() -> php_value::Object {
    let product = new_object(ECOMMERCE_PRODUCT);
    for collection in ["features", "categories", "related"] {
        set(&product, collection, ArrayCollectionObject::value(PhpArray::new()));
    }
    product
}

#[tokio::test]
async fn test_saves_a_many_to_many_association_with_cascade_save_set() {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = unnamed_product();
    let first_related = ecommerce_product("Business");
    let second_related = ecommerce_product("Home");

    add_related(&first_product, &first_related).await;
    add_related(&first_product, &second_related).await;
    em.persist(&first_product).unwrap();
    em.flush().await.unwrap();

    assert_eq!(count(&em, get(&first_product, "id"), get(&first_related, "id")).await, 1);
    assert_eq!(count(&em, get(&first_product, "id"), get(&second_related, "id")).await, 1);
}

#[tokio::test]
async fn test_removes_a_many_to_many_association() {
    let em = set_up(&ECOMMERCE_MODELS).await;
    let first_product = unnamed_product();
    let first_related = ecommerce_product("Business");
    let second_related = ecommerce_product("Home");

    add_related(&first_product, &first_related).await;
    add_related(&first_product, &second_related).await;
    em.persist(&first_product).unwrap();
    remove_related(&first_product, &first_related).await;
    em.flush().await.unwrap();

    assert_eq!(count(&em, get(&first_product, "id"), get(&first_related, "id")).await, 0);
    assert_eq!(count(&em, get(&first_product, "id"), get(&second_related, "id")).await, 1);
}
