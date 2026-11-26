//! Helpers shared by the `ValueConversionType` functional tests, with the
//! models' constructors (`associatedEntities = new ArrayCollection()`).
//!
//! Upstream: doctrine/orm tests/Tests/Models/ValueConversionType/InversedManyToManyCompositeIdEntity.php tests/Tests/Models/ValueConversionType/InversedManyToManyCompositeIdForeignKeyEntity.php
//! Upstream: doctrine/orm tests/Tests/Models/ValueConversionType/InversedManyToManyEntity.php tests/Tests/Models/ValueConversionType/InversedOneToManyCompositeIdEntity.php
//! Upstream: doctrine/orm tests/Tests/Models/ValueConversionType/InversedOneToManyCompositeIdForeignKeyEntity.php tests/Tests/Models/ValueConversionType/InversedOneToManyEntity.php
//! Upstream: doctrine/orm tests/Tests/Models/ValueConversionType/OwningManyToManyCompositeIdEntity.php tests/Tests/Models/ValueConversionType/OwningManyToManyCompositeIdForeignKeyEntity.php
//! Upstream: doctrine/orm tests/Tests/Models/ValueConversionType/OwningManyToManyEntity.php

#![allow(dead_code, reason = "each test file uses a subset")]

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_persistence::php_reflection;

use crate::orm::{self, get, new_object, s, set};

pub const NS: &str = "Doctrine\\Tests\\Models\\ValueConversionType\\";

/// The class name of a `ValueConversionType` model.
pub fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// `new $class()` with string fields set.
pub fn entity(class: &str, fields: &[(&str, &str)]) -> Object {
    let entity = new_object(class);
    for (name, value) in fields {
        set(&entity, name, s(value));
    }
    entity
}

/// `new $class()` whose constructor makes `associatedEntities` an
/// `ArrayCollection`, with string fields set.
pub fn entity_with_collection(class: &str, fields: &[(&str, &str)]) -> Object {
    let entity = entity(class, fields);
    set(
        &entity,
        "associatedEntities",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    entity
}

/// `$a->associatedEntities->add($b)`.
pub fn add(a: &Object, b: &Object) {
    coll::add(&get(a, "associatedEntities"), Value::Object(b.clone())).unwrap();
}

/// Registers the rot13 type and creates the schema of `classes`.
pub async fn set_up(classes: &[String]) -> Arc<EntityManager> {
    crate::rot13::register_rot13_type();
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    orm::set_up(&classes).await
}

/// Persists the objects in order, flushes and clears.
pub async fn persist_flush_clear(em: &EntityManager, objects: &[&Object]) {
    for object in objects {
        em.persist(object).unwrap();
    }
    em.flush().await.unwrap();
    em.clear().unwrap();
}

/// `$this->_em->find($class, $id)`, asserting a result.
pub async fn find(em: &EntityManager, class: &str, id: Value) -> Object {
    em.find(class, id)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("{class} was not found"))
}

/// `['id1' => $a, $field => $b]` as a `find()` criteria.
pub fn ids(pairs: &[(&str, Value)]) -> Value {
    let mut array = PhpArray::new();
    for (key, value) in pairs {
        array.insert(*key, value.clone());
    }
    Value::Array(array)
}

/// `self::assertEquals($expected, $conn->fetchOne($sql))`.
pub async fn assert_fetch_one(em: &EntityManager, expected: Value, sql: &str) {
    orm::assert_loose_eq(&expected, &orm::fetch_one(em, sql).await);
}

/// `self::assertInstanceOf($class, $object)`.
pub fn assert_instance_of(class: &str, object: &Object) {
    assert!(
        php_reflection::is_a(object.class(), class),
        "{} is not a {class}",
        object.class()
    );
}

/// `self::assertEquals($expected, $object->$name)` for a string.
pub fn assert_field(expected: &str, object: &Object, name: &str) {
    orm::assert_loose_eq(&s(expected), &get(object, name));
}

/// `self::assertCount($n, $object->associatedEntities)`.
pub async fn assert_collection_count(expected: usize, object: &Object) {
    assert_eq!(expected, orm::count_of(object, "associatedEntities").await);
}
