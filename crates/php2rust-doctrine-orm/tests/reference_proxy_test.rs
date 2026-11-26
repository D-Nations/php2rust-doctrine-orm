//! ReferenceProxyTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` with the `ecommerce` and `company` model sets and native
//! lazy objects. Reading a property of a lazy proxy does not load it, so
//! the tests call `initialize_object()` where PHP's method call on the
//! proxy would. `clone $entity` is `clone_entity()`: as PHP 8.4 does for a
//! lazy object, it initializes the original, copies its properties into a
//! new object and runs the class's `__clone()` (the bodies below).
//! `testCommonPersistenceProxy` is n/a: upstream skips it with native
//! lazy objects (no proxy classes are generated).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ReferenceProxyTest.php
//! Upstream: doctrine/orm tests/Tests/Models/ECommerce/ECommerceProduct2.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/company.rs"]
mod company;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use company::*;
use orm::*;

const ECOMMERCE_PRODUCT2: &str = "Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct2";

async fn set_up() -> Arc<EntityManager> {
    register_listeners();
    let mut classes: Vec<&str> = ECOMMERCE_MODELS.to_vec();
    classes.extend(COMPANY_MODELS);
    orm::set_up(&classes).await
}

async fn create_product(em: &Arc<EntityManager>) -> Value {
    let product = ecommerce_product("Doctrine Cookbook");
    em.persist(&product).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    get(&product, "id")
}

async fn create_auction(em: &Arc<EntityManager>) -> Value {
    let event = company_event(COMPANY_AUCTION, "Doctrine Cookbook");
    em.persist(&event).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    get(&event, "id")
}

async fn reference(em: &Arc<EntityManager>, class: &str, id: Value) -> Object {
    em.get_reference(class, id).await.unwrap().unwrap()
}

/// `$entity->getName()`: the proxy loads on the method call.
async fn name(em: &Arc<EntityManager>, entity: &Object) -> Value {
    em.initialize_object(entity).await.unwrap();
    get(entity, "name")
}

fn is_uninitialized(em: &Arc<EntityManager>, entity: &Object) -> bool {
    em.is_uninitialized_object(&Value::Object(entity.clone()))
}

/// `clone $entity`.
async fn clone_entity(em: &Arc<EntityManager>, entity: &Object) -> Object {
    em.initialize_object(entity).await.unwrap();
    let clone = new_object(entity.class());
    for (key, value) in entity.properties().iter() {
        set(
            &clone,
            &String::from_utf8_lossy(&key.bytes()),
            value.clone(),
        );
    }
    match entity.class() {
        // ECommerceProduct::__clone()
        ECOMMERCE_PRODUCT => {
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
        }
        // ECommerceProduct2::__clone()
        ECOMMERCE_PRODUCT2 => {
            set(&clone, "id", Value::Null);
            let name = get(&clone, "name");
            set(
                &clone,
                "name",
                s(&format!("Clone of {}", name.to_php_string().unwrap())),
            );
        }
        _ => {}
    }
    clone
}

#[tokio::test]
async fn testLazyLoadsFieldValuesFromDatabase() {
    let em = set_up().await;
    let id = create_product(&em).await;

    let mut criteria = PhpArray::new();
    criteria.insert("id", id);
    let product_proxy = reference(&em, ECOMMERCE_PRODUCT, Value::Array(criteria)).await;
    assert_eq!(s("Doctrine Cookbook"), name(&em, &product_proxy).await);
}

#[tokio::test]
async fn testAccessMetatadaForProxy() {
    let em = set_up().await;
    let id = create_product(&em).await;

    let entity = reference(&em, ECOMMERCE_PRODUCT, id).await;
    let class = em.get_class_metadata(entity.class()).unwrap();

    assert_eq!(ECOMMERCE_PRODUCT, class.read().name);
}

#[tokio::test]
async fn testReferenceFind() {
    let em = set_up().await;
    let id = create_product(&em).await;

    let entity = reference(&em, ECOMMERCE_PRODUCT, id.clone()).await;
    let entity2 = em.find(ECOMMERCE_PRODUCT, id).await.unwrap().unwrap();

    assert!(same(&entity, &entity2));
    assert_eq!(s("Doctrine Cookbook"), name(&em, &entity2).await);
}

#[tokio::test]
async fn testCloneProxy() {
    let em = set_up().await;
    let id = create_product(&em).await;

    let entity = reference(&em, ECOMMERCE_PRODUCT, id.clone()).await;

    let clone = clone_entity(&em, &entity).await;

    assert_eq!(id, get(&entity, "id"));
    assert_eq!(s("Doctrine Cookbook"), name(&em, &entity).await);

    assert!(
        !em.contains(&clone).unwrap(),
        "Cloning a reference proxy should return an unmanaged/detached entity."
    );
    assert_eq!(
        id,
        get(&clone, "id"),
        "Cloning a reference proxy should return same id."
    );
    assert_eq!(
        s("Doctrine Cookbook"),
        get(&clone, "name"),
        "Cloning a reference proxy should return same product name."
    );

    // domain logic, Product::__clone sets isCloned public property
    assert_eq!(Value::Bool(true), get(&clone, "isCloned"));
    assert_eq!(Value::Bool(false), get(&entity, "isCloned"));
}

#[tokio::test]
async fn testCloneProxyWithResetId() {
    let em = set_up().await;
    let id = create_product(&em).await;

    let entity = reference(&em, ECOMMERCE_PRODUCT2, id.clone()).await;

    let clone = clone_entity(&em, &entity).await;

    assert_eq!(id, get(&entity, "id"));
    assert_eq!(s("Doctrine Cookbook"), name(&em, &entity).await);

    assert!(!em.contains(&clone).unwrap());
    assert!(get(&clone, "id").is_null());
    assert_eq!(s("Clone of Doctrine Cookbook"), get(&clone, "name"));
}

#[tokio::test]
async fn testInitializeProxy() {
    let em = set_up().await;
    let id = create_product(&em).await;

    let entity = reference(&em, ECOMMERCE_PRODUCT, id).await;

    assert!(
        is_uninitialized(&em, &entity),
        "Pre-Condition: Object is unitialized proxy."
    );
    em.get_unit_of_work()
        .initialize_object(&entity)
        .await
        .unwrap();
    assert!(
        !is_uninitialized(&em, &entity),
        "Should be initialized after called UnitOfWork::initializeObject()"
    );
}

#[tokio::test]
async fn testInitializeChangeAndFlushProxy() {
    let em = set_up().await;
    let id = create_product(&em).await;

    let entity = reference(&em, ECOMMERCE_PRODUCT, id.clone()).await;
    // $entity->setName(): the proxy loads on the method call.
    em.initialize_object(&entity).await.unwrap();
    set(&entity, "name", s("Doctrine 2 Cookbook"));

    em.flush().await.unwrap();
    em.clear().unwrap();

    let entity = reference(&em, ECOMMERCE_PRODUCT, id).await;
    assert_eq!(s("Doctrine 2 Cookbook"), name(&em, &entity).await);
}

#[tokio::test]
async fn testWakeupOnProxy() {
    let em = set_up().await;
    let id = create_product(&em).await;

    let entity = reference(&em, ECOMMERCE_PRODUCT, id).await;

    assert_eq!(Value::Bool(false), get(&entity, "wakeUp"));

    em.initialize_object(&entity).await.unwrap();
    set(&entity, "name", s("Doctrine 2 Cookbook"));

    // A native lazy object is not a CommonProxy.
    assert_eq!(
        Value::Bool(false),
        get(&entity, "wakeUp"),
        "Loading the proxy should call __wakeup()."
    );
}

#[tokio::test]
async fn testDoNotInitializeProxyOnGettingTheIdentifier() {
    let em = set_up().await;
    let id = create_product(&em).await;

    let entity = reference(&em, ECOMMERCE_PRODUCT, id.clone()).await;

    assert!(
        is_uninitialized(&em, &entity),
        "Pre-Condition: Object is unitialized proxy."
    );
    assert_eq!(id, get(&entity, "id"));
    assert!(
        is_uninitialized(&em, &entity),
        "Getting the identifier doesn't initialize the proxy."
    );
}

#[tokio::test]
async fn testDoNotInitializeProxyOnGettingTheIdentifierDDC1625() {
    let em = set_up().await;
    let id = create_auction(&em).await;

    let entity = reference(&em, COMPANY_AUCTION, id.clone()).await;

    assert!(
        is_uninitialized(&em, &entity),
        "Pre-Condition: Object is unitialized proxy."
    );
    assert_eq!(id, get(&entity, "id"));
    assert!(
        is_uninitialized(&em, &entity),
        "Getting the identifier doesn't initialize the proxy when extending."
    );
}

#[tokio::test]
async fn testDoNotInitializeProxyOnGettingTheIdentifierAndReturnTheRightType() {
    let em = set_up().await;
    let product = ecommerce_product("Doctrine Cookbook");

    let shipping = new_object(ECOMMERCE_SHIPPING);
    set(&shipping, "days", Value::Int(1));
    set(&product, "shipping", Value::Object(shipping.clone()));
    em.persist(&product).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let id = get(&shipping, "id");

    let product = em
        .get_repository(ECOMMERCE_PRODUCT)
        .unwrap()
        .find(get(&product, "id"))
        .await
        .unwrap()
        .unwrap();

    let entity = get_object(&product, "shipping").unwrap();
    assert!(
        is_uninitialized(&em, &entity),
        "Pre-Condition: Object is unitialized proxy."
    );
    assert!(php_value::compare::loose_equals(&id, &get(&entity, "id")));
    assert!(
        id.identical(&get(&entity, "id")),
        "Check that the id's are the same value, and type."
    );
    assert!(
        is_uninitialized(&em, &entity),
        "Getting the identifier doesn't initialize the proxy."
    );
}

#[tokio::test]
async fn testInitializeProxyOnGettingSomethingOtherThanTheIdentifier() {
    let em = set_up().await;
    let id = create_product(&em).await;

    let entity = reference(&em, ECOMMERCE_PRODUCT, id).await;

    assert!(
        is_uninitialized(&em, &entity),
        "Pre-Condition: Object is unitialized proxy."
    );
    assert_eq!(s("Doctrine Cookbook"), name(&em, &entity).await);
    assert!(
        !is_uninitialized(&em, &entity),
        "Getting something other than the identifier initializes the proxy."
    );
}
