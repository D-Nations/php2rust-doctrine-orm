//! PHPUnit's assertions and PHP's operators over [`Value`]s, for the tests
//! translated line by line from the hydration tests: `$a[$k]` is
//! [`idx`], `$a->p` is [`prop`], `$a->m(...)` is [`call`], and the
//! `ResultSetMapping` calls go through [`rsm_call`] with PHP's defaults.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Hydration/HydrationTestCase.php tests/Tests/Mocks/ArrayResultFactory.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::needless_pass_by_value,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;

use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_orm::collection_object::{ArrayCollectionObject, downcast};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::internal::hydration::{self, HydrationHints};
use php2rust_doctrine_orm::object::EntityObject;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::query::ResultSetMapping;
use php2rust_doctrine_orm::unit_of_work::Hints;
use php2rust_doctrine_orm::utility::to_text;
use php2rust_doctrine_persistence::php_reflection;

/// A PHP string.
pub fn s(text: &str) -> Value {
    Value::string(text)
}

fn key(value: &Value) -> Key {
    match value {
        Value::Int(i) => Key::Int(*i),
        Value::Bool(b) => Key::Int(i64::from(*b)),
        other => Key::from_bytes(to_text(other).as_bytes()),
    }
}

/// `(bool) $value`.
pub fn truthy(value: &Value) -> bool {
    value.is_truthy()
}

/// `$a ?: $b`.
pub fn elvis(a: Value, b: Value) -> Value {
    if a.is_truthy() { a } else { b }
}

/// `$a === $b`.
pub fn same(a: &Value, b: &Value) -> Value {
    Value::Bool(a.identical(b))
}

/// `$a !== $b`.
pub fn not_same(a: &Value, b: &Value) -> Value {
    Value::Bool(!a.identical(b))
}

/// `$a == $b`.
pub fn loose(a: &Value, b: &Value) -> Value {
    Value::Bool(php_value::compare::loose_equals(a, b))
}

/// `$a != $b`.
pub fn not_loose(a: &Value, b: &Value) -> Value {
    Value::Bool(!php_value::compare::loose_equals(a, b))
}

/// `!$a`.
pub fn not(a: &Value) -> Value {
    Value::Bool(!a.is_truthy())
}

/// `-$a`.
pub fn neg(a: &Value) -> Value {
    match a {
        Value::Int(i) => Value::Int(-i),
        Value::Float(f) => Value::Float(-f),
        other => panic!("cannot negate {other:?}"),
    }
}

/// `$a + 1`.
pub fn inc(a: &Value) -> Value {
    match a {
        Value::Int(i) => Value::Int(i + 1),
        other => panic!("cannot increment {other:?}"),
    }
}

/// `[$a, $b, ...]`.
pub fn list_of(values: Vec<Value>) -> Value {
    Value::Array(PhpArray::list(values))
}

/// `[$k => $v, ...]` (a `null` key appends).
pub fn array_of(entries: Vec<(Value, Value)>) -> Value {
    let mut array = PhpArray::new();
    for (k, v) in entries {
        if k.is_null() {
            array.push(v).unwrap();
        } else {
            array.insert(key(&k), v);
        }
    }
    Value::Array(array)
}

/// The elements of an array or a collection.
fn elements(value: &Value) -> Option<PhpArray> {
    match value {
        Value::Array(array) => Some(array.clone()),
        other => {
            if let Some(pc) = PersistentCollection::from_value(other) {
                return Some(pc.unwrap());
            }
            ArrayCollectionObject::from_value(other).map(|c| c.lock().to_array())
        }
    }
}

/// `$value[$key]` (`null` when not set, as `isset()` reads it).
pub fn idx(value: &Value, k: &Value) -> Value {
    match elements(value) {
        Some(array) => array.get(&key(k)).cloned().unwrap_or(Value::Null),
        None => panic!("cannot index {value:?}"),
    }
}

/// `foreach ($value as $v)`.
pub fn iter(value: &Value) -> Vec<Value> {
    elements(value)
        .unwrap_or_else(|| panic!("cannot iterate {value:?}"))
        .values()
        .cloned()
        .collect()
}

/// `$value->$name`.
pub fn prop(value: &Value, name: &str) -> Value {
    let Value::Object(object) = value else {
        panic!("property {name} of {value:?}");
    };
    let entity = downcast::<EntityObject>(object).expect("an entity object");
    entity.get(name).unwrap_or(Value::Null)
}

/// `$value->$method(...$args)` for the collection and entity methods the
/// tests call.
pub fn call(value: &Value, method: &str, args: &[Value]) -> Value {
    if let Some(pc) = PersistentCollection::from_value(value) {
        return match method {
            "count" => Value::Int(i64::try_from(pc.unwrap().len()).unwrap()),
            "isInitialized" => Value::Bool(pc.is_initialized()),
            "isDirty" => Value::Bool(pc.is_dirty()),
            "first" => pc
                .unwrap()
                .values()
                .next()
                .cloned()
                .unwrap_or(Value::Bool(false)),
            "getValues" => list_of(pc.unwrap().values().cloned().collect()),
            "toArray" => Value::Array(pc.unwrap()),
            "getKeys" => list_of(pc.unwrap().keys().map(Key::to_value).collect()),
            "get" => idx(value, &args[0]),
            "containsKey" => Value::Bool(pc.unwrap().contains_key(&key(&args[0]))),
            other => panic!("PersistentCollection::{other}()"),
        };
    }
    if let Some(c) = ArrayCollectionObject::from_value(value) {
        let array = c.lock().to_array();
        return match method {
            "count" => Value::Int(i64::try_from(array.len()).unwrap()),
            "first" => array.values().next().cloned().unwrap_or(Value::Bool(false)),
            "get" => idx(value, &args[0]),
            other => panic!("ArrayCollection::{other}()"),
        };
    }
    if let Some(field) = method.strip_prefix("get") {
        let mut chars = field.chars();
        let first = chars.next().map_or_default(|c| c.to_ascii_lowercase());
        return prop(value, &format!("{first}{}", chars.as_str()));
    }
    panic!("method {method}() of {value:?}")
}

/// `count($value)`.
pub fn php_count(args: &[Value]) -> Value {
    let n = elements(&args[0]).unwrap_or_else(|| panic!("count() of {:?}", args[0]));
    Value::Int(i64::try_from(n.len()).unwrap())
}

/// `isset($value)`.
pub fn php_isset(args: &[Value]) -> Value {
    Value::Bool(args.iter().all(|v| !v.is_null()))
}

/// `property_exists($object, $name)`.
pub fn php_property_exists(args: &[Value]) -> Value {
    let Value::Object(object) = &args[0] else {
        return Value::Bool(false);
    };
    let entity = downcast::<EntityObject>(object).expect("an entity object");
    Value::Bool(entity.get(&to_text(&args[1])).is_some())
}

fn text(value: &Value) -> String {
    to_text(value)
}

fn opt(value: Option<&Value>) -> Option<String> {
    value.filter(|v| !v.is_null()).map(to_text)
}

/// `$rsm->$method(...$args)` with PHP's default arguments.
pub fn rsm_call(rsm: &mut ResultSetMapping, method: &str, args: &[Value]) {
    let a = |i: usize| text(&args[i]);
    match method {
        "addEntityResult" => {
            let result_alias = opt(args.get(2));
            rsm.add_entity_result(&a(0), &a(1), result_alias.as_deref());
        }
        "addFieldResult" => {
            let declaring = opt(args.get(3));
            rsm.add_field_result(&a(0), &a(1), &a(2), declaring.as_deref());
        }
        "addJoinedEntityResult" => {
            rsm.add_joined_entity_result(&a(0), &a(1), &a(2), &a(3), false);
        }
        "addScalarResult" => {
            let ty = opt(args.get(2)).unwrap_or_else(|| "string".to_owned());
            rsm.add_scalar_result(&a(0), key(&args[1]), &ty);
        }
        "addMetaResult" => {
            let is_identifier = args.get(3).is_some_and(Value::is_truthy);
            let ty = opt(args.get(4));
            rsm.add_meta_result(&a(0), &a(1), &a(2), is_identifier, ty.as_deref());
        }
        "addIndexBy" => {
            rsm.add_index_by(&a(0), &a(1));
        }
        "addIndexByScalar" => {
            rsm.add_index_by_scalar(&a(0));
        }
        "addIndexByColumn" => {
            rsm.add_index_by_column(&a(0), &a(1));
        }
        "setDiscriminatorColumn" => {
            rsm.set_discriminator_column(&a(0), &a(1));
        }
        "addEnumResult" => {
            rsm.add_enum_result(&a(0), &a(1));
        }
        other => panic!("ResultSetMapping::{other}()"),
    }
}

fn rows_of(result_set: &Value) -> Vec<PhpArray> {
    iter(result_set)
        .into_iter()
        .map(|row| match row {
            Value::Array(row) => row,
            other => panic!("not a row: {other:?}"),
        })
        .collect()
}

fn hints_of(hints: &Value) -> (Hints, HydrationHints) {
    let mut uow_hints = Hints::default();
    let mut extra = HydrationHints::default();
    if let Value::Array(hints) = hints {
        for (name, value) in hints.iter() {
            match php2rust_doctrine_orm::utility::key_text(name).as_str() {
                "deferEagerLoad" => {
                    uow_hints.defer_eager_load = !value.is_null();
                    extra.defer_eager_load = Some(value.identical(&Value::Bool(true)));
                }
                "doctrine.refresh" => uow_hints.refresh = !value.is_null(),
                "doctrine.refresh.entity" => {
                    if let Value::Object(entity) = value {
                        uow_hints.refresh_entity = Some(entity.clone());
                    }
                }
                other => panic!("hint {other}"),
            }
        }
    }
    (uow_hints, extra)
}

/// `$hydrator->hydrateAll($stmt, $rsm, $hints)`.
pub async fn php_hydrate_all(
    em: &Arc<EntityManager>,
    mode: i64,
    result_set: &Value,
    rsm: &ResultSetMapping,
    hints: &Value,
) -> Result<Value, Thrown> {
    let (mut uow_hints, extra) = hints_of(hints);
    hydration::hydrate(em, mode, rows_of(result_set), rsm, &mut uow_hints, extra).await
}

/// `$hydrator->toIterable($stmt, $rsm, $hints)`.
pub async fn php_to_iterable(
    em: &Arc<EntityManager>,
    mode: i64,
    result_set: &Value,
    rsm: &ResultSetMapping,
    hints: &Value,
) -> Result<Value, Thrown> {
    let (uow_hints, extra) = hints_of(hints);
    hydration::to_iterable(em, mode, rows_of(result_set), rsm, &uow_hints, extra)
        .await
        .map(list_of)
}

/// `assertEquals($expected, $actual)`: PHP's `==`, by value for arrays
/// and objects.
pub fn assert_equals(expected: &Value, actual: &Value) {
    assert!(
        php_value::compare::loose_equals(expected, actual),
        "{expected:?} != {actual:?}"
    );
}

/// `assertSame($expected, $actual)`.
pub fn assert_same(expected: &Value, actual: &Value) {
    assert!(expected.identical(actual), "{expected:?} !== {actual:?}");
}

/// `assertNotSame($expected, $actual)`.
pub fn assert_not_same(expected: &Value, actual: &Value) {
    assert!(!expected.identical(actual), "{expected:?} === {actual:?}");
}

/// `assertInstanceOf($class, $actual)`.
pub fn assert_instance_of(class: &Value, actual: &Value) {
    let class = to_text(class);
    let Value::Object(object) = actual else {
        panic!("{actual:?} is not an instance of {class}");
    };
    let actual_class = object.class();
    assert!(
        actual_class == class || php_reflection::is_a(actual_class, &class),
        "{actual_class} is not an instance of {class}"
    );
}

/// `assertContainsOnly($class, $values)`.
pub fn assert_contains_only(class: &Value, values: &Value) {
    for value in iter(values) {
        assert_instance_of(class, &value);
    }
}

/// `assertArrayHasKey($key, $array)`.
pub fn assert_array_has_key(k: &Value, array: &Value) {
    let array = elements(array).unwrap_or_else(|| panic!("not an array: {array:?}"));
    assert!(array.contains_key(&key(k)), "no key {k:?} in {array:?}");
}

/// `assertArrayNotHasKey($key, $array)`.
pub fn assert_array_not_has_key(k: &Value, array: &Value) {
    let array = elements(array).unwrap_or_else(|| panic!("not an array: {array:?}"));
    assert!(!array.contains_key(&key(k)), "key {k:?} in {array:?}");
}

/// `assertIsArray($value)`.
pub fn assert_is_array(value: &Value) {
    assert!(matches!(value, Value::Array(_)), "not an array: {value:?}");
}

/// `assertCount($count, $value)`.
pub fn assert_count(count: &Value, value: &Value) {
    assert_equals(count, &php_count(std::slice::from_ref(value)));
}

/// `assertEmpty($value)`.
pub fn assert_empty(value: &Value) {
    assert!(!value.is_truthy(), "not empty: {value:?}");
}

/// `assertNotEmpty($value)`.
pub fn assert_not_empty(value: &Value) {
    assert!(value.is_truthy(), "empty: {value:?}");
}

/// `assertTrue($value)`.
pub fn assert_true(value: &Value) {
    assert!(value.identical(&Value::Bool(true)), "not true: {value:?}");
}

/// `assertFalse($value)`.
pub fn assert_false(value: &Value) {
    assert!(value.identical(&Value::Bool(false)), "not false: {value:?}");
}

/// `assertNull($value)`.
pub fn assert_null(value: &Value) {
    assert!(value.is_null(), "not null: {value:?}");
}

/// `assertNotNull($value)`.
pub fn assert_not_null(value: &Value) {
    assert!(!value.is_null(), "null");
}
