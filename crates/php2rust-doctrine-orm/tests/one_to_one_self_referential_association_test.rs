//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OneToOneSelfReferentialAssociationTest.php
//!
//! On `SQLite` with the `ecommerce` model set (`tests/common/orm.rs`) and
//! the recorded `MultiSelfReference` model of the test file. Reading a
//! property of a lazy proxy does not load it: the tests call
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
use php2rust_doctrine_orm::query::context::HYDRATE_OBJECT;

use orm::{
    ECOMMERCE_CUSTOMER, ECOMMERCE_MODELS, assert_loose_eq, fetch_one_with, get, list, new_object,
    obj, s, set, set_obj,
};

const MULTI_SELF_REFERENCE: &str = "Doctrine\\Tests\\ORM\\Functional\\MultiSelfReference";

struct Test {
    em: Arc<EntityManager>,
    customer: Object,
    mentor: Object,
}

fn ecommerce_customer(name: &str) -> Object {
    let customer = new_object(ECOMMERCE_CUSTOMER);
    set(&customer, "name", s(name));
    customer
}

async fn set_up() -> Test {
    let em = orm::set_up(&ECOMMERCE_MODELS).await;
    Test {
        em,
        customer: ecommerce_customer("Anakin Skywalker"),
        mentor: ecommerce_customer("Obi-wan Kenobi"),
    }
}

impl Test {
    async fn assert_loading_of_association(&self, customer: &Object) {
        let mentor = obj(&get(customer, "mentor"));
        assert!(mentor.instance_of(ECOMMERCE_CUSTOMER));
        // $customer->getMentor()->getName()
        self.em.initialize_object(&mentor).await.unwrap();
        assert_eq!(s("Obi-wan Kenobi"), get(&mentor, "name"));
    }

    async fn assert_foreign_key_is(&self, value: &Value) {
        let foreign_key = fetch_one_with(
            &self.em,
            "SELECT mentor_id FROM ecommerce_customers WHERE id=?",
            vec![get(&self.customer, "id")],
        )
        .await;
        assert_loose_eq(value, &foreign_key);
    }

    async fn create_fixture(&self) -> Value {
        let customer = ecommerce_customer("Luke Skywalker");
        let mentor = ecommerce_customer("Obi-wan Kenobi");
        set_obj(&customer, "mentor", &mentor);

        self.em.persist(&customer).unwrap();

        self.em.flush().await.unwrap();
        self.em.clear().unwrap();

        get(&customer, "id")
    }
}

#[tokio::test]
async fn testSavesAOneToOneAssociationWithCascadeSaveSet() {
    let t = set_up().await;
    set_obj(&t.customer, "mentor", &t.mentor);
    t.em.persist(&t.customer).unwrap();
    t.em.flush().await.unwrap();

    t.assert_foreign_key_is(&get(&t.mentor, "id")).await;
}

#[tokio::test]
async fn testRemovesOneToOneAssociation() {
    let t = set_up().await;
    set_obj(&t.customer, "mentor", &t.mentor);
    t.em.persist(&t.customer).unwrap();
    // removeMentor()
    set(&t.customer, "mentor", Value::Null);

    t.em.flush().await.unwrap();

    t.assert_foreign_key_is(&Value::Null).await;
}

#[tokio::test]
async fn testFind() {
    let t = set_up().await;
    let id = t.create_fixture().await;

    let customer = t.em.find(ECOMMERCE_CUSTOMER, id).await.unwrap().unwrap();
    assert!(!t.em.is_uninitialized_object(&get(&customer, "mentor")));
}

#[tokio::test]
async fn testEagerLoadsAssociation() {
    let t = set_up().await;
    let customer_id = t.create_fixture().await;

    let query = t.em.create_query(
        "select c, m from Doctrine\\Tests\\Models\\ECommerce\\ECommerceCustomer c left join c.mentor m where c.id = :id",
    );
    query.set_parameter("id", customer_id, None);

    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());
    let customer = obj(&result[0]);
    t.assert_loading_of_association(&customer).await;
}

/// mine
#[tokio::test]
async fn testLazyLoadsAssociation() {
    let t = set_up().await;
    t.create_fixture().await;

    let metadata = t.em.get_class_metadata(ECOMMERCE_CUSTOMER).unwrap();
    metadata
        .write()
        .association_mappings
        .get_mut("mentor")
        .unwrap()
        .fetch = Some(FETCH_LAZY);

    let query = t.em.create_query(
        "select c from Doctrine\\Tests\\Models\\ECommerce\\ECommerceCustomer c where c.name='Luke Skywalker'",
    );
    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());
    let customer = obj(&result[0]);
    t.assert_loading_of_association(&customer).await;
}

#[tokio::test]
async fn testMultiSelfReference() {
    let t = set_up().await;
    orm::create_schema(&t.em, &[MULTI_SELF_REFERENCE]).await;

    let entity1 = new_object(MULTI_SELF_REFERENCE);
    t.em.persist(&entity1).unwrap();
    let entity2 = new_object(MULTI_SELF_REFERENCE);
    set_obj(&entity1, "other1", &entity2);
    let entity3 = new_object(MULTI_SELF_REFERENCE);
    set_obj(&entity1, "other2", &entity3);
    t.em.flush().await.unwrap();

    t.em.clear().unwrap();

    let entity2 = t
        .em
        .find(entity1.class(), get(&entity1, "id"))
        .await
        .unwrap()
        .unwrap();

    let other1 = obj(&get(&entity2, "other1"));
    let other2 = obj(&get(&entity2, "other2"));
    assert!(other1.instance_of(MULTI_SELF_REFERENCE));
    assert!(other2.instance_of(MULTI_SELF_REFERENCE));
    // getOther1()->getOther1() on a proxy
    t.em.initialize_object(&other1).await.unwrap();
    t.em.initialize_object(&other2).await.unwrap();
    assert_eq!(Value::Null, get(&other1, "other1"));
    assert_eq!(Value::Null, get(&other1, "other2"));
    assert_eq!(Value::Null, get(&other2, "other1"));
    assert_eq!(Value::Null, get(&other2, "other2"));
}
