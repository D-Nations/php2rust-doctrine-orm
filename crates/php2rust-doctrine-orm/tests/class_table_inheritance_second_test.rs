//! ClassTableInheritanceSecondTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! Functional tests for the Class Table Inheritance mapping strategy, on
//! the models the test file declares.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ClassTableInheritanceSecondTest.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_persistence::php_reflection;

use orm::*;

const CTI_PARENT: &str = "Doctrine\\Tests\\ORM\\Functional\\CTIParent";
const CTI_CHILD: &str = "Doctrine\\Tests\\ORM\\Functional\\CTIChild";
const CTI_RELATED: &str = "Doctrine\\Tests\\ORM\\Functional\\CTIRelated";
const CTI_RELATED2: &str = "Doctrine\\Tests\\ORM\\Functional\\CTIRelated2";

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[CTI_PARENT, CTI_CHILD, CTI_RELATED, CTI_RELATED2]).await
}

/// `new CTIChild()` with its data.
fn cti_child(data: &str) -> Object {
    let child = new_object(CTI_CHILD);
    set(&child, "data", s(data));
    child
}

/// `new CTIRelated2()`.
fn cti_related2() -> Object {
    let related = new_object(CTI_RELATED2);
    set(
        &related,
        "ctiChildren",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    related
}

fn is_a(object: &Object, class: &str) -> bool {
    php_reflection::is_a(object.class(), class)
}

#[tokio::test]
async fn testOneToOneAssocToBaseTypeBidirectional() {
    let em = set_up().await;
    let child = cti_child("hello");

    let related = new_object(CTI_RELATED);
    set_obj(&related, "ctiParent", &child);

    em.persist(&related).unwrap();
    em.persist(&child).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let related_id = get(&related, "id");

    let related2 = em.find(CTI_RELATED, related_id).await.unwrap().unwrap();

    assert!(is_a(&related2, CTI_RELATED));
    let parent = get_object(&related2, "ctiParent").unwrap();
    assert!(is_a(&parent, CTI_CHILD));
    assert_loose_eq(&s("hello"), &get(&parent, "data"));

    assert!(same(&related2, &get_object(&parent, "related").unwrap()));
}

#[tokio::test]
async fn testManyToManyToCTIHierarchy() {
    let em = set_up().await;
    let mmrel = cti_related2();
    let child = cti_child("child");
    coll::add(&get(&mmrel, "ctiChildren"), Value::Object(child.clone())).unwrap();

    em.persist(&mmrel).unwrap();
    em.persist(&child).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let mmrel2 = em
        .find(mmrel.class(), get(&mmrel, "id"))
        .await
        .unwrap()
        .unwrap();
    let children = get(&mmrel2, "ctiChildren");
    let collection = PersistentCollection::from_value(&children).expect("a persistent collection");
    assert!(!collection.is_initialized());
    assert_eq!(coll::count(&children).await.unwrap(), 1);
    assert!(collection.is_initialized());
    assert!(is_a(
        &obj(&coll::get(&children, 0_i64).await.unwrap()),
        CTI_CHILD
    ));
}
