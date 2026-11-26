//! Functional/TypeValueSqlTest.php on `SQLite` (`tests/common/orm.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/TypeValueSqlTest.php
//! Upstream: doctrine/orm tests/Tests/DbalTypes/UpperCaseStringType.php tests/Tests/DbalTypes/NegativeToPositiveType.php
//! Upstream: doctrine/orm tests/Tests/Models/CustomType/CustomTypeParent.php tests/Tests/Models/CustomType/CustomTypeChild.php
//! Upstream: doctrine/orm tests/Tests/Models/CustomType/CustomTypeUpperCase.php

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

use php_value::compare::loose_equals;
use php_value::{Object, PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type};
use php2rust_doctrine_dbal::types::{TypeArgument, add_type, has_type, override_type};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::context::HYDRATE_OBJECT;

use orm::{fetch_one, get, new_object, obj, s, set};

const CUSTOM_TYPE_CHILD: &str = "Doctrine\\Tests\\Models\\CustomType\\CustomTypeChild";
const CUSTOM_TYPE_PARENT: &str = "Doctrine\\Tests\\Models\\CustomType\\CustomTypeParent";
const CUSTOM_TYPE_UPPER_CASE: &str = "Doctrine\\Tests\\Models\\CustomType\\CustomTypeUpperCase";

/// `Doctrine\Tests\DbalTypes\UpperCaseStringType extends StringType`.
#[derive(Debug)]
struct UpperCaseStringType;

impl Type for UpperCaseStringType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\UpperCaseStringType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(value.clone())
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(value.clone())
    }

    fn convert_to_database_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        let mut out = b"UPPER(".to_vec();
        out.extend_from_slice(sql_expr);
        out.push(b')');
        PhpString::from(out)
    }

    fn convert_to_php_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        let mut out = b"LOWER(".to_vec();
        out.extend_from_slice(sql_expr);
        out.push(b')');
        PhpString::from(out)
    }
}

/// `Doctrine\Tests\DbalTypes\NegativeToPositiveType extends Type`.
#[derive(Debug)]
struct NegativeToPositiveType;

impl Type for NegativeToPositiveType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\NegativeToPositiveType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_integer_type_declaration_sql(column)
    }

    fn convert_to_database_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        let mut sql = b"ABS(".to_vec();
        sql.extend_from_slice(sql_expr);
        sql.push(b')');
        PhpString::from(sql)
    }

    fn convert_to_php_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        let mut sql = b"-(".to_vec();
        sql.extend_from_slice(sql_expr);
        sql.push(b')');
        PhpString::from(sql)
    }
}

/// `DBALType::addType($name, $class)`, or `overrideType()` when the
/// type is there.
fn add_or_override(name: &str, type_: impl Fn() -> TypeArgument) {
    if has_type(name) {
        override_type(name, type_()).unwrap();
    } else if add_type(name, type_()).is_err() {
        // Another test thread may have added it meanwhile.
        override_type(name, type_()).unwrap();
    }
}

async fn set_up() -> Arc<EntityManager> {
    add_or_override("upper_case_string", || {
        TypeArgument::Instance(Arc::new(UpperCaseStringType))
    });
    add_or_override("negative_to_positive", || {
        TypeArgument::Instance(Arc::new(NegativeToPositiveType))
    });

    // useModelSet('customtype')
    orm::set_up(&[
        CUSTOM_TYPE_CHILD,
        CUSTOM_TYPE_PARENT,
        CUSTOM_TYPE_UPPER_CASE,
    ])
    .await
}

/// `new CustomTypeParent()`.
fn custom_type_parent() -> Object {
    let parent = new_object(CUSTOM_TYPE_PARENT);
    set(
        &parent,
        "friendsWithMe",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(
        &parent,
        "myFriends",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    parent
}

/// `$parent->addMyFriend($friend)`.
fn add_my_friend(parent: &Object, friend: &Object) {
    coll::add(&get(parent, "myFriends"), Value::Object(friend.clone())).unwrap();
    coll::add(&get(friend, "friendsWithMe"), Value::Object(parent.clone())).unwrap();
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
