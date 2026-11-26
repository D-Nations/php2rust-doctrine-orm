//! Helpers for the Ticket tests that declare their own models in the
//! test file (namespace `Doctrine\Tests\ORM\Functional\Ticket`).
//!
//! Upstream: doctrine/orm tests/Tests/OrmFunctionalTestCase.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use crate::orm;

pub const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\";

/// The class name of a model of the Ticket namespace.
pub fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// `parent::setUp()` and `createSchemaForModels(...)` for models of the
/// Ticket namespace.
pub async fn set_up(shorts: &[&str]) -> Arc<EntityManager> {
    let classes: Vec<String> = shorts.iter().map(|short| class(short)).collect();
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    orm::set_up(&classes).await
}

/// `new $short()` of the Ticket namespace, without a constructor body.
pub fn new(short: &str) -> Object {
    orm::new_object(&class(short))
}

/// `$this->_em->getRepository($class)->findAll()`.
pub async fn find_all(em: &Arc<EntityManager>, short: &str) -> Vec<Object> {
    em.get_repository(&class(short))
        .unwrap()
        .find_all()
        .await
        .unwrap()
}

/// `$this->_em->find($class, $id)`.
pub async fn find(em: &EntityManager, short: &str, id: Value) -> Option<Object> {
    em.find(&class(short), id).await.unwrap()
}

/// `isUninitializedObject($value)`.
pub fn is_uninitialized(em: &EntityManager, value: &Value) -> bool {
    em.is_uninitialized_object(value)
}

/// `assertInstanceOf($class, $value)` for a model of the Ticket namespace
/// (a subclass passes too).
pub fn assert_instance_of(short: &str, value: &Value) {
    let class = class(short);
    assert!(
        matches!(value, Value::Object(o) if o.instance_of(&class)),
        "{value:?} is not a {class}"
    );
}

/// An array of `[key => value]` pairs.
pub fn array(pairs: &[(&str, Value)]) -> Value {
    let mut array = PhpArray::new();
    for (key, value) in pairs {
        array.insert(*key, value.clone());
    }
    Value::Array(array)
}

/// `$enum::$name`, a case of a recorded enum.
pub fn case(enum_type: &str, name: &str) -> Value {
    crate::models::register();
    let class = php2rust_doctrine_persistence::php_reflection::class_info(enum_type)
        .unwrap_or_else(|| panic!("the enum {enum_type} is declared"));
    let case = class
        .cases()
        .iter()
        .find(|case| {
            case.property(b"name")
                .is_some_and(|n| n == Value::string(name))
        })
        .cloned()
        .unwrap_or_else(|| panic!("the case {enum_type}::{name} is declared"));
    Value::Object(case)
}

/// `$this->_em->getConnection()->insert($table, $data)`.
pub async fn insert(em: &EntityManager, table: &str, data: &[(&str, Value)]) {
    let Value::Array(data) = array(data) else {
        unreachable!("array() builds an array")
    };
    em.get_connection()
        .insert(table, &data, &php2rust_doctrine_dbal::ParameterTypes::new())
        .await
        .unwrap();
}

/// DBAL's `EnableForeignKeys` middleware, which upstream's `TestUtil`
/// adds on `SQLite`: `PRAGMA foreign_keys = ON`.
pub async fn enable_foreign_keys(em: &EntityManager) {
    em.get_connection()
        .execute_statement(
            "PRAGMA foreign_keys = ON",
            &PhpArray::new(),
            &php2rust_doctrine_dbal::ParameterTypes::new(),
        )
        .await
        .unwrap();
}

/// `parent::setUp()` and `createSchemaForModels(...)` with the foreign
/// keys enforced, as upstream's `SQLite` connection does.
pub async fn set_up_with_foreign_keys(shorts: &[&str]) -> Arc<EntityManager> {
    let em = set_up(shorts).await;
    enable_foreign_keys(&em).await;
    em
}

/// `Type::addType($name, $class)` (or `overrideType()` when another test
/// of the binary added it already): the test's DBAL type in the registry.
pub fn add_type(name: &str, type_: Arc<dyn php2rust_doctrine_dbal::types::Type>) {
    use php2rust_doctrine_dbal::types::{TypeArgument, add_type, has_type, override_type};
    let argument = || TypeArgument::Instance(Arc::clone(&type_));
    if has_type(name) {
        override_type(name, argument()).unwrap();
    } else if add_type(name, argument()).is_err() {
        // Another test thread may have added it meanwhile.
        override_type(name, argument()).unwrap();
    }
}
