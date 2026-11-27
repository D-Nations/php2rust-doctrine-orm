//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OneToOneUnidirectionalAssociationTest.php
//!
//! On `SQLite` with the `ecommerce` model set (`tests/common/orm.rs`).
//! Reading a property of a lazy proxy does not load it: the tests call
//! `initialize_object()` where PHP's getter on the proxy would.

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::class_metadata::FETCH_LAZY;
use php2rust_doctrine_orm::query::context::{HYDRATE_OBJECT, Hint};
use php2rust_doctrine_orm::query::dql_query::HINT_FORCE_PARTIAL_LOAD;

use orm::{
    ECOMMERCE_MODELS, ECOMMERCE_PRODUCT, ECOMMERCE_SHIPPING, assert_loose_eq, ecommerce_product,
    fetch_one_with, get, list, new_object, obj, s, set, set_obj,
};

struct Test {
    em: Arc<EntityManager>,
    product: Object,
    shipping: Object,
}

async fn set_up() -> Test {
    let em = orm::set_up(&ECOMMERCE_MODELS).await;

    let product = ecommerce_product("Doctrine 2 Manual");
    let shipping = new_object(ECOMMERCE_SHIPPING);
    set(&shipping, "days", s("5"));
    Test {
        em,
        product,
        shipping,
    }
}

impl Test {
    async fn create_fixture(&self) {
        let product = ecommerce_product("Php manual");
        let shipping = new_object(ECOMMERCE_SHIPPING);
        set(&shipping, "days", s("1"));
        set_obj(&product, "shipping", &shipping);

        self.em.persist(&product).unwrap();

        self.em.flush().await.unwrap();
        self.em.clear().unwrap();
    }

    async fn assert_foreign_key_is(&self, value: &Value) {
        let foreign_key = fetch_one_with(
            &self.em,
            "SELECT shipping_id FROM ecommerce_products WHERE id=?",
            vec![get(&self.product, "id")],
        )
        .await;
        assert_loose_eq(value, &foreign_key);
    }

    async fn first_product(&self, dql: &str, partial: bool) -> Object {
        let query = self.em.create_query(dql);
        if partial {
            query.set_hint(HINT_FORCE_PARTIAL_LOAD, Hint::Value(Value::Bool(true)));
        }
        let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());
        obj(&result[0])
    }
}

#[tokio::test]
async fn testSavesAOneToOneAssociationWithCascadeSaveSet() {
    let t = set_up().await;
    set_obj(&t.product, "shipping", &t.shipping);
    t.em.persist(&t.product).unwrap();
    t.em.flush().await.unwrap();

    t.assert_foreign_key_is(&get(&t.shipping, "id")).await;
}

#[tokio::test]
async fn testRemovesOneToOneAssociation() {
    let t = set_up().await;
    set_obj(&t.product, "shipping", &t.shipping);
    t.em.persist(&t.product).unwrap();
    // removeShipping()
    set(&t.product, "shipping", Value::Null);

    t.em.flush().await.unwrap();

    t.assert_foreign_key_is(&Value::Null).await;
}

#[tokio::test]
async fn testEagerLoad() {
    let t = set_up().await;
    t.create_fixture().await;

    let product = t
        .first_product(
            "select p, s from Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct p left join p.shipping s",
            false,
        )
        .await;

    let shipping = obj(&get(&product, "shipping"));
    assert!(shipping.instance_of(ECOMMERCE_SHIPPING));
    assert_loose_eq(&Value::Int(1), &get(&shipping, "days"));
}

#[tokio::test]
async fn testLazyLoadsObjects() {
    let t = set_up().await;
    t.create_fixture().await;
    let metadata = t.em.get_class_metadata(ECOMMERCE_PRODUCT).unwrap();
    metadata
        .write()
        .association_mappings
        .get_mut("shipping")
        .unwrap()
        .fetch = Some(FETCH_LAZY);

    let product = t
        .first_product(
            "select p from Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct p",
            false,
        )
        .await;

    let shipping = obj(&get(&product, "shipping"));
    assert!(shipping.instance_of(ECOMMERCE_SHIPPING));
    // $product->getShipping()->getDays()
    t.em.initialize_object(&shipping).await.unwrap();
    assert_loose_eq(&Value::Int(1), &get(&shipping, "days"));
}

#[tokio::test]
async fn testDoesNotLazyLoadObjectsIfConfigurationDoesNotAllowIt() {
    let t = set_up().await;
    t.create_fixture().await;

    let product = t
        .first_product(
            "select p from Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct p",
            true,
        )
        .await;

    assert_eq!(Value::Null, get(&product, "shipping"));
}

/// DDC-762
#[tokio::test]
async fn testNullForeignKey() {
    let t = set_up().await;
    let product = ecommerce_product("Doctrine 2 Manual");

    t.em.persist(&product).unwrap();
    t.em.flush().await.unwrap();

    let product = t
        .em
        .find(product.class(), get(&product, "id"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(Value::Null, get(&product, "shipping"));
}
