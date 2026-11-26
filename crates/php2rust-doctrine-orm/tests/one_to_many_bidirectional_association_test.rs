//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OneToManyBidirectionalAssociationTest.php

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

struct Fixture {
    em: Arc<EntityManager>,
    product: Object,
    first_feature: Object,
    second_feature: Object,
}

async fn set_up_test() -> Fixture {
    let em = set_up(&ECOMMERCE_MODELS).await;
    Fixture {
        em,
        product: ecommerce_product("Doctrine Cookbook"),
        first_feature: ecommerce_feature("Model writing tutorial"),
        second_feature: ecommerce_feature("Attributes examples"),
    }
}

async fn assert_feature_foreign_key_is(em: &EntityManager, value: Value, feature: &Object) {
    let foreign_key = fetch_one_with(
        em,
        "SELECT product_id FROM ecommerce_features WHERE id=?",
        vec![get(feature, "id")],
    )
    .await;
    assert_loose_eq(&value, &foreign_key);
}

#[tokio::test]
async fn test_saves_a_one_to_many_association_with_cascade_save_set() {
    let f = set_up_test().await;
    add_feature(&f.product, &f.first_feature);
    add_feature(&f.product, &f.second_feature);
    f.em.persist(&f.product).unwrap();
    f.em.flush().await.unwrap();

    assert_feature_foreign_key_is(&f.em, get(&f.product, "id"), &f.first_feature).await;
    assert_feature_foreign_key_is(&f.em, get(&f.product, "id"), &f.second_feature).await;
}

#[tokio::test]
async fn test_saves_an_empty_collection() {
    let f = set_up_test().await;
    f.em.persist(&f.product).unwrap();
    f.em.flush().await.unwrap();

    assert_eq!(count_of(&f.product, "features").await, 0);
}

#[tokio::test]
async fn test_does_not_save_an_inverse_side_set() {
    let f = set_up_test().await;
    // $product->brokenAddFeature($feature)
    append(&f.product, "features", Value::Object(f.first_feature.clone()));
    f.em.persist(&f.product).unwrap();
    f.em.flush().await.unwrap();

    assert_feature_foreign_key_is(&f.em, Value::Null, &f.first_feature).await;
}

#[tokio::test]
async fn test_removes_one_to_one_association() {
    let f = set_up_test().await;
    add_feature(&f.product, &f.first_feature);
    add_feature(&f.product, &f.second_feature);
    f.em.persist(&f.product).unwrap();

    remove_feature(&f.product, &f.first_feature).await;
    f.em.flush().await.unwrap();

    assert_feature_foreign_key_is(&f.em, Value::Null, &f.first_feature).await;
    assert_feature_foreign_key_is(&f.em, get(&f.product, "id"), &f.second_feature).await;
}
