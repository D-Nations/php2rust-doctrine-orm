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

async fn create_fixture(f: &Fixture) {
    add_child(&f.parent, &f.first_child);
    add_child(&f.parent, &f.second_child);
    f.em.persist(&f.parent).unwrap();

    f.em.flush().await.unwrap();
    f.em.clear().unwrap();
}

/// `strstr($name, ' books')`.
fn strstr_books(name: &Value) -> Value {
    let name = php2rust_doctrine_orm::utility::to_text(name);
    name.find(" books")
        .map_or(Value::Bool(false), |i| s(&name[i..]))
}

async fn assert_children_of(parent: &Object) {
    let children = elements(parent, "children").await;

    assert_eq!(children[0].class(), ECOMMERCE_CATEGORY);
    assert!(same_value(&get(&children[0], "parent"), parent));
    assert_loose_eq(&s(" books"), &strstr_books(&get(&children[0], "name")));
    assert_eq!(children[1].class(), ECOMMERCE_CATEGORY);
    assert!(same_value(&get(&children[1], "parent"), parent));
    assert_loose_eq(&s(" books"), &strstr_books(&get(&children[1], "name")));
}

#[tokio::test]
async fn testEagerLoadsOneToManyAssociation() {
    let f = set_up_test().await;
    create_fixture(&f).await;

    let query = f.em.create_query(
        "select c1, c2 from Doctrine\\Tests\\Models\\ECommerce\\ECommerceCategory c1 join c1.children c2",
    );
    let result = query.get_object_result().await.unwrap();
    assert_eq!(list(&result).len(), 1);
    let parent = obj(&at(&result, 0));
    assert_children_of(&parent).await;
}

#[tokio::test]
async fn testLazyLoadsOneToManyAssociation() {
    let f = set_up_test().await;
    create_fixture(&f).await;
    f.em.get_class_metadata(ECOMMERCE_CATEGORY)
        .unwrap()
        .write()
        .association_mappings
        .get_mut("children")
        .unwrap()
        .fetch = Some(php2rust_doctrine_orm::mapping::class_metadata::FETCH_LAZY);

    let query = f.em.create_query(
        "select c from Doctrine\\Tests\\Models\\ECommerce\\ECommerceCategory c order by c.id asc",
    );
    let result = query.get_object_result().await.unwrap();
    let parent = obj(&at(&result, 0));
    assert_children_of(&parent).await;
}
