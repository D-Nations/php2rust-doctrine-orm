//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OneToManySelfReferentialAssociationTest.php

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
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

struct Fixture {
    em: Arc<EntityManager>,
    parent: Object,
    first_child: Object,
    second_child: Object,
}

async fn set_up_test() -> Fixture {
    let em = set_up(&ECOMMERCE_MODELS).await;
    Fixture {
        em,
        parent: ecommerce_category("Programming languages books"),
        first_child: ecommerce_category("Java books"),
        second_child: ecommerce_category("Php books"),
    }
}

async fn assert_foreign_key_is(em: &EntityManager, value: Value, child: &Object) {
    let foreign_key = fetch_one_with(
        em,
        "SELECT parent_id FROM ecommerce_categories WHERE id=?",
        vec![get(child, "id")],
    )
    .await;
    assert_loose_eq(&value, &foreign_key);
}

#[tokio::test]
async fn testSavesAOneToManyAssociationWithCascadeSaveSet() {
    let f = set_up_test().await;
    add_child(&f.parent, &f.first_child);
    add_child(&f.parent, &f.second_child);
    f.em.persist(&f.parent).unwrap();
    f.em.flush().await.unwrap();

    assert_foreign_key_is(&f.em, get(&f.parent, "id"), &f.first_child).await;
    assert_foreign_key_is(&f.em, get(&f.parent, "id"), &f.second_child).await;
}

#[tokio::test]
async fn testSavesAnEmptyCollection() {
    let f = set_up_test().await;
    f.em.persist(&f.parent).unwrap();
    f.em.flush().await.unwrap();

    assert_eq!(count_of(&f.parent, "children").await, 0);
}

#[tokio::test]
async fn testDoesNotSaveAnInverseSideSet() {
    let f = set_up_test().await;
    // $parent->brokenAddChild($child)
    append(&f.parent, "children", Value::Object(f.first_child.clone()));
    f.em.persist(&f.parent).unwrap();
    f.em.flush().await.unwrap();

    assert_foreign_key_is(&f.em, Value::Null, &f.first_child).await;
}

#[tokio::test]
async fn testRemovesOneToManyAssociation() {
    let f = set_up_test().await;
    add_child(&f.parent, &f.first_child);
    add_child(&f.parent, &f.second_child);
    f.em.persist(&f.parent).unwrap();

    remove_child(&f.parent, &f.first_child).await;
    f.em.flush().await.unwrap();

    assert_foreign_key_is(&f.em, Value::Null, &f.first_child).await;
    assert_foreign_key_is(&f.em, get(&f.parent, "id"), &f.second_child).await;
}
