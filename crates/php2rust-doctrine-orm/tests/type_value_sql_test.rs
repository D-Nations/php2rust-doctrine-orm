//! Functional/TypeValueSqlTest.php on `SQLite` (`tests/common/orm.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/TypeValueSqlTest.php
//! Upstream: doctrine/orm tests/Tests/Models/CustomType/CustomTypeChild.php
//! Upstream: doctrine/orm tests/Tests/Models/CustomType/CustomTypeUpperCase.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/custom_types.rs"]
mod custom_types;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php_value::compare::loose_equals;
use php_value::Value;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::context::HYDRATE_OBJECT;

use custom_types::{CUSTOM_TYPE_CHILD, CUSTOM_TYPE_PARENT, add_my_friend, custom_type_parent};
use orm::{fetch_one, get, new_object, obj, s, set};

const CUSTOM_TYPE_UPPER_CASE: &str = "Doctrine\\Tests\\Models\\CustomType\\CustomTypeUpperCase";

async fn set_up() -> Arc<EntityManager> {
    custom_types::register_types();

    // useModelSet('customtype')
    orm::set_up(&[
        CUSTOM_TYPE_CHILD,
        CUSTOM_TYPE_PARENT,
        CUSTOM_TYPE_UPPER_CASE,
    ])
    .await
}

fn assert_equals(expected: &Value, actual: &Value, message: &str) {
    assert!(
        loose_equals(expected, actual),
        "{message}: {expected:?} != {actual:?}"
    );
}

#[tokio::test]
async fn testUpperCaseStringType() {
    let em = set_up().await;
    let entity = new_object(CUSTOM_TYPE_UPPER_CASE);
    set(&entity, "lowerCaseString", s("foo"));

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    let id = get(&entity, "id");

    em.clear().unwrap();

    let entity = em.find(CUSTOM_TYPE_UPPER_CASE, id).await.unwrap().unwrap();

    assert_equals(
        &s("foo"),
        &get(&entity, "lowerCaseString"),
        "Entity holds lowercase string",
    );
    let sql = format!(
        "select lowerCaseString from customtype_uppercases where id={}",
        get(&entity, "id")
            .to_php_string()
            .unwrap()
            .to_string_lossy()
    );
    assert_equals(
        &s("FOO"),
        &fetch_one(&em, &sql).await,
        "Database holds uppercase string",
    );
}

#[tokio::test]
async fn testUpperCaseStringTypeWhenColumnNameIsDefined() {
    let em = set_up().await;
    let entity = new_object(CUSTOM_TYPE_UPPER_CASE);
    set(&entity, "lowerCaseString", s("Some Value"));
    set(&entity, "namedLowerCaseString", s("foo"));

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    let id = get(&entity, "id");

    em.clear().unwrap();

    let entity = em.find(CUSTOM_TYPE_UPPER_CASE, id).await.unwrap().unwrap();
    assert_equals(
        &s("foo"),
        &get(&entity, "namedLowerCaseString"),
        "Entity holds lowercase string",
    );
    let sql = format!(
        "select named_lower_case_string from customtype_uppercases where id={}",
        get(&entity, "id")
            .to_php_string()
            .unwrap()
            .to_string_lossy()
    );
    assert_equals(
        &s("FOO"),
        &fetch_one(&em, &sql).await,
        "Database holds uppercase string",
    );

    set(&entity, "namedLowerCaseString", s("bar"));

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    let id = get(&entity, "id");

    em.clear().unwrap();

    let entity = em.find(CUSTOM_TYPE_UPPER_CASE, id).await.unwrap().unwrap();
    assert_equals(
        &s("bar"),
        &get(&entity, "namedLowerCaseString"),
        "Entity holds lowercase string",
    );
    assert_equals(
        &s("BAR"),
        &fetch_one(&em, &sql).await,
        "Database holds uppercase string",
    );
}

#[tokio::test]
async fn testTypeValueSqlWithAssociations() {
    let em = set_up().await;
    let parent = custom_type_parent();
    set(&parent, "customInteger", Value::Int(-1));
    set(
        &parent,
        "child",
        Value::Object(new_object(CUSTOM_TYPE_CHILD)),
    );

    let friend1 = custom_type_parent();
    let friend2 = custom_type_parent();

    add_my_friend(&parent, &friend1);
    add_my_friend(&parent, &friend2);

    em.persist(&parent).unwrap();
    em.persist(&friend1).unwrap();
    em.persist(&friend2).unwrap();
    em.flush().await.unwrap();

    let parent_id = get(&parent, "id");

    em.clear().unwrap();

    let entity = em
        .find(CUSTOM_TYPE_PARENT, parent_id)
        .await
        .unwrap()
        .unwrap();

    assert!(
        matches!(get(&entity, "customInteger"), Value::Int(i) if i < 0),
        "Fetched customInteger negative"
    );
    let sql = format!(
        "select customInteger from customtype_parents where id={}",
        get(&entity, "id")
            .to_php_string()
            .unwrap()
            .to_string_lossy()
    );
    assert_equals(
        &Value::Int(1),
        &fetch_one(&em, &sql).await,
        "Database has stored customInteger positive",
    );

    assert!(!get(&parent, "child").is_null(), "Child attached");
    assert_eq!(
        coll::count(&get(&entity, "myFriends")).await.unwrap(),
        2,
        "2 friends attached"
    );
}

#[tokio::test]
async fn testSelectDQL() {
    let em = set_up().await;
    let parent = custom_type_parent();
    set(&parent, "customInteger", Value::Int(-1));
    set(
        &parent,
        "child",
        Value::Object(new_object(CUSTOM_TYPE_CHILD)),
    );

    em.persist(&parent).unwrap();
    em.flush().await.unwrap();

    let parent_id = get(&parent, "id");

    em.clear().unwrap();

    let query = em.create_query(&format!(
        "SELECT p, p.customInteger, c from Doctrine\\Tests\\Models\\CustomType\\CustomTypeParent p JOIN p.child c where p.id = {}",
        parent_id.to_php_string().unwrap().to_string_lossy()
    ));

    let Value::Array(result) = query.get_result(HYDRATE_OBJECT).await.unwrap() else {
        panic!("not a list");
    };

    assert_eq!(result.len(), 1);
    let Some(Value::Array(row)) = result.values().next().cloned() else {
        panic!("not a row: {result:?}");
    };
    let entity = obj(&row.get(&0.into()).cloned().unwrap());
    assert_eq!(entity.class(), CUSTOM_TYPE_PARENT);
    assert_equals(
        &Value::Int(-1),
        &get(&entity, "customInteger"),
        "customInteger",
    );

    assert_equals(
        &Value::Int(-1),
        &row.get(&"customInteger".into()).cloned().unwrap(),
        "customInteger scalar",
    );

    let child = obj(&get(&entity, "child"));
    assert_equals(&s("foo"), &get(&child, "lowerCaseString"), "child");
}
