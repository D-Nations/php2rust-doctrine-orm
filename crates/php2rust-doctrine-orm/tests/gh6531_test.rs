//! Ticket/GH6531Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `GH6531Article::$attributes` has no constructor: `addAttribute()`'s
//! `$this->attributes[$name] = ...` makes it an array.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6531Test.php

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
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

async fn set_up() -> Arc<EntityManager> {
    ticket::set_up(&[
        "GH6531User",
        "GH6531Address",
        "GH6531Article",
        "GH6531ArticleAttribute",
        "GH6531Order",
        "GH6531OrderItem",
        "GH6531Product",
    ])
    .await
}

#[tokio::test]
async fn testSimpleDerivedIdentity() {
    let em = set_up().await;
    let user = new("GH6531User");
    let address = new("GH6531Address");
    set_obj(&address, "user", &user);

    em.persist(&user).unwrap();
    em.persist(&address).unwrap();
    em.flush().await.unwrap();

    let found = find(&em, "GH6531User", get(&user, "id")).await.unwrap();
    assert!(same(&user, &found));
    let found = find(&em, "GH6531Address", Value::Object(user.clone()))
        .await
        .unwrap();
    assert!(same(&address, &found));
}

/// `GH6531Article::addAttribute($name, $value)`.
fn add_attribute(article: &Object, name: &str, value: &str) {
    // new GH6531ArticleAttribute($name, $value, $this)
    let attribute = new("GH6531ArticleAttribute");
    set(&attribute, "attribute", s(name));
    set(&attribute, "value", s(value));
    set_obj(&attribute, "article", article);

    let mut attributes = match get(article, "attributes") {
        Value::Array(attributes) => attributes,
        _ => PhpArray::new(),
    };
    attributes.insert(name, Value::Object(attribute));
    set(article, "attributes", Value::Array(attributes));
}

#[tokio::test]
async fn testDynamicAttributes() {
    let em = set_up().await;
    let article = new("GH6531Article");
    add_attribute(&article, "name", "value");

    em.persist(&article).unwrap();
    em.flush().await.unwrap();

    let found = find(
        &em,
        "GH6531ArticleAttribute",
        array(&[
            ("article", Value::Object(article.clone())),
            ("attribute", s("name")),
        ]),
    )
    .await
    .unwrap();
    // $article->attributes['name']: a PersistentCollection after the flush
    let attribute = coll::get(&get(&article, "attributes"), "name")
        .await
        .unwrap();
    assert!(same_value(&attribute, &found));
}

#[tokio::test]
async fn testJoinTableWithMetadata() {
    let em = set_up().await;
    let product = new("GH6531Product");
    em.persist(&product).unwrap();
    em.flush().await.unwrap();

    // new GH6531Order()
    let order = new("GH6531Order");
    set(
        &order,
        "items",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    // $order->addItem($product, 2): new GH6531OrderItem($this, $product, $amount)
    let item = new("GH6531OrderItem");
    set_obj(&item, "order", &order);
    set_obj(&item, "product", &product);
    set(&item, "amount", Value::Int(2));
    coll::add(&get(&order, "items"), Value::Object(item)).unwrap();

    em.persist(&order).unwrap();
    em.flush().await.unwrap();

    let found = find(
        &em,
        "GH6531OrderItem",
        array(&[
            ("product", Value::Object(product.clone())),
            ("order", Value::Object(order.clone())),
        ]),
    )
    .await
    .unwrap();
    assert!(same_value(
        &coll::first(&get(&order, "items")).await.unwrap(),
        &found
    ));
}
