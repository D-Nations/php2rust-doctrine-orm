//! Ticket/GH10334Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the Models/GH10334 models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10334Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

const FOO: &str = "Doctrine\\Tests\\Models\\GH10334\\GH10334Foo";
const FOO_COLLECTION: &str = "Doctrine\\Tests\\Models\\GH10334\\GH10334FooCollection";
const PRODUCT: &str = "Doctrine\\Tests\\Models\\GH10334\\GH10334Product";
const PRODUCT_TYPE: &str = "Doctrine\\Tests\\Models\\GH10334\\GH10334ProductType";
const PRODUCT_TYPE_ID: &str = "Doctrine\\Tests\\Models\\GH10334\\GH10334ProductTypeId";

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[FOO_COLLECTION, FOO, PRODUCT_TYPE, PRODUCT]).await
}

/// `new GH10334Foo($collection, $productTypeId)`.
#[allow(clippy::disallowed_names, reason = "upstream's names")]
fn foo(collection: &Object, product_type_id: &str) -> Object {
    let foo = new_object(FOO);
    set_obj(&foo, "collection", collection);
    set(
        &foo,
        "productTypeId",
        ticket::case(PRODUCT_TYPE_ID, product_type_id),
    );
    foo
}

/// `new GH10334ProductType($id, $value)`.
fn product_type(id: &str, value: f64) -> Object {
    let product_type = new_object(PRODUCT_TYPE);
    set(&product_type, "id", ticket::case(PRODUCT_TYPE_ID, id));
    set(&product_type, "value", Value::Float(value));
    set(
        &product_type,
        "products",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    product_type
}

async fn foo_collections(em: &Arc<EntityManager>) -> Vec<Value> {
    let mut qb = em
        .get_repository(FOO_COLLECTION)
        .unwrap()
        .create_query_builder("collection", None)
        .unwrap();
    qb.left_join("collection.foos", "foo", None, None, None)
        .unwrap()
        .add_select(vec![s("foo")])
        .unwrap();
    list(
        &qb.get_query()
            .unwrap()
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    )
}

#[tokio::test]
async fn testTicket() {
    let em = set_up().await;

    let collection = new_object(FOO_COLLECTION);
    set(
        &collection,
        "foos",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    let foo = foo(&collection, "Jean");
    let foo2 = self::foo(&collection, "Short");

    em.persist(&collection).unwrap();
    em.persist(&foo).unwrap();
    em.persist(&foo2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let result = foo_collections(&em).await;

    foo_collections(&em).await;

    assert_eq!(1, result.len());
    assert_eq!(2, count_of(&obj(&result[0]), "foos").await);
}

#[tokio::test]
async fn testGetChildWithBackedEnumId() {
    let em = set_up().await;

    let jean = product_type("Jean", 23.5);
    let short = product_type("Short", 45.2);
    // new GH10334Product('Extra Large Blue', $jean)
    let product = new_object(PRODUCT);
    set(&product, "name", s("Extra Large Blue"));
    set_obj(&product, "productType", &jean);
    // $jean->addProduct($product)
    set_obj(&product, "productType", &jean);
    coll::add(&get(&jean, "products"), Value::Object(product.clone())).unwrap();

    em.persist(&jean).unwrap();
    em.persist(&short).unwrap();
    em.persist(&product).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let entity = em.find(PRODUCT, Value::Int(1)).await.unwrap();
    let entity = entity.expect("the product");

    let product_type = obj(&get(&entity, "productType"));
    let id = get(&product_type, "id");
    let expected = ticket::case(PRODUCT_TYPE_ID, "Jean");
    assert!(expected.identical(&id), "{id:?}");
}
