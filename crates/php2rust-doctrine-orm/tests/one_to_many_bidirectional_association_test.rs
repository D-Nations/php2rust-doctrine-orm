//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OneToManyBidirectionalAssociationTest.php

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

use std::sync::Arc;

use php_value::{Object, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_orm::collection_object as coll;
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
async fn testSavesAOneToManyAssociationWithCascadeSaveSet() {
    let f = set_up_test().await;
    add_feature(&f.product, &f.first_feature);
    add_feature(&f.product, &f.second_feature);
    f.em.persist(&f.product).unwrap();
    f.em.flush().await.unwrap();

    assert_feature_foreign_key_is(&f.em, get(&f.product, "id"), &f.first_feature).await;
    assert_feature_foreign_key_is(&f.em, get(&f.product, "id"), &f.second_feature).await;
}

#[tokio::test]
async fn testSavesAnEmptyCollection() {
    let f = set_up_test().await;
    f.em.persist(&f.product).unwrap();
    f.em.flush().await.unwrap();

    assert_eq!(count_of(&f.product, "features").await, 0);
}

#[tokio::test]
async fn testDoesNotSaveAnInverseSideSet() {
    let f = set_up_test().await;
    // $product->brokenAddFeature($feature)
    append(
        &f.product,
        "features",
        Value::Object(f.first_feature.clone()),
    );
    f.em.persist(&f.product).unwrap();
    f.em.flush().await.unwrap();

    assert_feature_foreign_key_is(&f.em, Value::Null, &f.first_feature).await;
}

#[tokio::test]
async fn testRemovesOneToOneAssociation() {
    let f = set_up_test().await;
    add_feature(&f.product, &f.first_feature);
    add_feature(&f.product, &f.second_feature);
    f.em.persist(&f.product).unwrap();

    remove_feature(&f.product, &f.first_feature).await;
    f.em.flush().await.unwrap();

    assert_feature_foreign_key_is(&f.em, Value::Null, &f.first_feature).await;
    assert_feature_foreign_key_is(&f.em, get(&f.product, "id"), &f.second_feature).await;
}

async fn create_fixture(fixture: &Fixture) {
    add_feature(&fixture.product, &fixture.first_feature);
    add_feature(&fixture.product, &fixture.second_feature);
    fixture.em.persist(&fixture.product).unwrap();

    fixture.em.flush().await.unwrap();
    fixture.em.clear().unwrap();
}

fn assert_instance_of(class: &str, value: &Value) {
    let Value::Object(object) = value else {
        panic!("{value:?} is not an instance of {class}");
    };
    assert_eq!(object.class(), class);
}

/// `$object->getX()` of a lazy object: PHP initializes the object when a
/// property is read; the port's reads do not query, so the
/// initialization is explicit.
async fn get_initialized(em: &EntityManager, object: &Object, name: &str) -> Value {
    if em.is_uninitialized_object(&Value::Object(object.clone())) {
        em.initialize_object(object).await.unwrap();
    }
    get(object, name)
}

fn persistent_collection(
    value: &Value,
) -> Arc<php2rust_doctrine_orm::persistent_collection::PersistentCollection> {
    php2rust_doctrine_orm::persistent_collection::PersistentCollection::from_value(value)
        .expect("a persistent collection")
}

#[tokio::test]
async fn testEagerLoadsOneToManyAssociation() {
    let fixture = set_up_test().await;
    let em = &fixture.em;
    create_fixture(&fixture).await;
    let query = em.create_query(
        "select p, f from Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct p join p.features f",
    );
    let result = query.get_object_result().await.unwrap();
    let product = obj(&at(&result, 0));

    let features = elements(&product, "features").await;

    assert_instance_of(ECOMMERCE_FEATURE, &Value::Object(features[0].clone()));
    assert!(!em.is_uninitialized_object(&get(&features[0], "product")));
    assert!(same_value(&get(&features[0], "product"), &product));
    assert_loose_eq(
        &s("Model writing tutorial"),
        &get(&features[0], "description"),
    );
    assert_instance_of(ECOMMERCE_FEATURE, &Value::Object(features[1].clone()));
    assert!(same_value(&get(&features[1], "product"), &product));
    assert!(!em.is_uninitialized_object(&get(&features[1], "product")));
    assert_loose_eq(&s("Attributes examples"), &get(&features[1], "description"));
}

#[tokio::test]
async fn testLazyLoadsObjectsOnTheOwningSide() {
    let fixture = set_up_test().await;
    let em = &fixture.em;
    create_fixture(&fixture).await;

    let query =
        em.create_query("select p from Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct p");
    let result = query.get_object_result().await.unwrap();
    let product = obj(&at(&result, 0));
    let features = persistent_collection(&get(&product, "features"));

    assert!(!features.is_initialized());
    let elements = elements(&product, "features").await;
    assert_instance_of(ECOMMERCE_FEATURE, &Value::Object(elements[0].clone()));
    assert!(features.is_initialized());
    assert!(same_value(&get(&elements[0], "product"), &product));
    assert_loose_eq(
        &s("Model writing tutorial"),
        &get(&elements[0], "description"),
    );
    assert_instance_of(ECOMMERCE_FEATURE, &Value::Object(elements[1].clone()));
    assert!(same_value(&get(&elements[1], "product"), &product));
    assert_loose_eq(&s("Attributes examples"), &get(&elements[1], "description"));
}

#[tokio::test]
async fn testLazyLoadsObjectsOnTheInverseSide() {
    let fixture = set_up_test().await;
    let em = &fixture.em;
    create_fixture(&fixture).await;

    let query =
        em.create_query("select f from Doctrine\\Tests\\Models\\ECommerce\\ECommerceFeature f");
    let features = query.get_object_result().await.unwrap();

    let product = get(&obj(&at(&features, 0)), "product");
    assert_instance_of(ECOMMERCE_PRODUCT, &product);
    assert!(em.is_uninitialized_object(&product));
    assert!(s("Doctrine Cookbook").identical(&get_initialized(em, &obj(&product), "name").await));
    assert!(!em.is_uninitialized_object(&product));
}

#[tokio::test]
async fn testLazyLoadsObjectsOnTheInverseSide2() {
    let fixture = set_up_test().await;
    let em = &fixture.em;
    create_fixture(&fixture).await;

    let query = em.create_query(
        "select f,p from Doctrine\\Tests\\Models\\ECommerce\\ECommerceFeature f join f.product p",
    );
    let features = query.get_object_result().await.unwrap();

    let product = get(&obj(&at(&features, 0)), "product");
    assert!(!em.is_uninitialized_object(&product));
    assert_instance_of(ECOMMERCE_PRODUCT, &product);
    assert!(s("Doctrine Cookbook").identical(&get(&obj(&product), "name")));

    assert!(!persistent_collection(&get(&obj(&product), "features")).is_initialized());
}

#[tokio::test]
async fn testJoinFromOwningSide() {
    let fixture = set_up_test().await;
    let em = &fixture.em;
    let query = em.create_query(
        "select f,p from Doctrine\\Tests\\Models\\ECommerce\\ECommerceFeature f join f.product p",
    );
    let features = query.get_object_result().await.unwrap();
    assert_eq!(list(&features).len(), 0);
}

/// `Doctrine\Common\Collections\Collection`.
const COLLECTION: &str = "Doctrine\\Common\\Collections\\Collection";

/// `$this->_em->find(ECommerceProduct::class, $this->product->getId())`.
async fn find_product(f: &Fixture) -> Object {
    f.em.find(ECOMMERCE_PRODUCT, get(&f.product, "id"))
        .await
        .unwrap()
        .unwrap()
}

/// `Criteria::create()->where(Criteria::expr()->eq('description', $description))`.
fn description_is(description: &str) -> Criteria {
    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("description", s(description)));
    criteria
}

/// `assertInstanceOf(Collection::class, $results)`.
fn assert_collection(results: &Value) {
    let Value::Object(object) = results else {
        panic!("{results:?} is not a collection");
    };
    assert!(object.instance_of(COLLECTION), "{}", object.class());
}

#[tokio::test]
async fn testMatching() {
    let f = set_up_test().await;
    create_fixture(&f).await;

    let product = find_product(&f).await;
    let features = get(&product, "features");

    let results = coll::matching(&features, &description_is("Model writing tutorial"))
        .await
        .unwrap();

    assert_collection(&results);
    assert_eq!(coll::count(&results).await.unwrap(), 1);

    let results = coll::matching(&features, &Criteria::create())
        .await
        .unwrap();

    assert_collection(&results);
    assert_eq!(coll::count(&results).await.unwrap(), 2);
}

#[tokio::test]
async fn testMatchingOnDirtyCollection() {
    let f = set_up_test().await;
    create_fixture(&f).await;

    let product = find_product(&f).await;

    let third_feature = ecommerce_feature("Model writing tutorial");

    let features = get(&product, "features");
    coll::add(&features, Value::Object(third_feature)).unwrap();

    let results = coll::matching(&features, &description_is("Model writing tutorial"))
        .await
        .unwrap();

    assert_eq!(coll::count(&results).await.unwrap(), 2);
}

#[tokio::test]
async fn testMatchingBis() {
    let f = set_up_test().await;
    create_fixture(&f).await;

    let product = find_product(&f).await;
    let features = get(&product, "features");

    let third_feature = ecommerce_feature("Third feature");
    add_feature(&product, &third_feature);

    let results = coll::matching(&features, &description_is("Third feature"))
        .await
        .unwrap();

    assert_collection(&results);
    assert_eq!(coll::count(&results).await.unwrap(), 1);

    let results = coll::matching(&features, &Criteria::create())
        .await
        .unwrap();

    assert_collection(&results);
    assert_eq!(coll::count(&results).await.unwrap(), 3);
}
