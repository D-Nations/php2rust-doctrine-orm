//! Port of `CompositeKeyRelationsTest`, on `SQLite` with the
//! `compositekeyrelations` model set. The test calls
//! `initialize_object()` where PHP's property read of the lazy customer
//! initializes it.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/CompositeKeyRelationsTest.php

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

use php_value::{Object, PhpArray, Value, php_array};
use php2rust_doctrine_dbal::ParameterTypes;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{get, get_object};

const CUSTOMER_CLASS: &str = "Doctrine\\Tests\\Models\\CompositeKeyRelations\\CustomerClass";
const INVOICE_CLASS: &str = "Doctrine\\Tests\\Models\\CompositeKeyRelations\\InvoiceClass";

/// `setUp()`: `useModelSet('compositekeyrelations')`.
async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[CUSTOMER_CLASS, INVOICE_CLASS]).await
}

async fn insert(em: &EntityManager, table: &str, data: &PhpArray) {
    em.get_connection()
        .insert(table, data, &ParameterTypes::new())
        .await
        .unwrap();
}

/// `findEntity()`.
async fn find_entity(em: &EntityManager, company_code: &str, invoice_number: &str) -> Object {
    em.find(
        INVOICE_CLASS,
        Value::Array(php_array! {
            "companyCode" => company_code,
            "invoiceNumber" => invoice_number,
        }),
    )
    .await
    .unwrap()
    .unwrap()
}

#[tokio::test]
async fn testFindEntityWithNotNullRelation() {
    let em = set_up().await;
    insert(
        &em,
        "CustomerClass",
        &php_array! { "companyCode" => "AA", "code" => "CUST1", "name" => "Customer 1" },
    )
    .await;

    insert(
        &em,
        "InvoiceClass",
        &php_array! { "companyCode" => "AA", "invoiceNumber" => "INV1", "customerCode" => "CUST1" },
    )
    .await;

    let entity = find_entity(&em, "AA", "INV1").await;
    assert_eq!(Value::from("AA"), get(&entity, "companyCode"));
    assert_eq!(Value::from("INV1"), get(&entity, "invoiceNumber"));
    let customer = get_object(&entity, "customer").unwrap();
    assert!(customer.instance_of(CUSTOMER_CLASS));
    // `$entity->customer->name` initializes the lazy object
    em.initialize_object(&customer).await.unwrap();
    assert_eq!(Value::from("Customer 1"), get(&customer, "name"));
}

#[tokio::test]
async fn testFindEntityWithNullRelation() {
    let em = set_up().await;
    insert(
        &em,
        "InvoiceClass",
        &php_array! { "companyCode" => "BB", "invoiceNumber" => "INV1" },
    )
    .await;

    let entity = find_entity(&em, "BB", "INV1").await;
    assert_eq!(Value::from("BB"), get(&entity, "companyCode"));
    assert_eq!(Value::from("INV1"), get(&entity, "invoiceNumber"));
    assert!(get(&entity, "customer").is_null());
}
