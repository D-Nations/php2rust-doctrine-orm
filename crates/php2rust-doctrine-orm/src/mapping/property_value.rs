//! The typed properties of the mapping value objects, as PHP assigns them
//! under `declare(strict_types=1)`: a value of another type is a
//! `TypeError` (`Cannot assign ... to property ... of type ...`), and the
//! `__sleep()` serialization of an object of one of these classes.
//!
//! Upstream: none

use std::sync::Arc;

use php_value::object::GenericObject;
use php_value::serialize::{Classes, serialize, unserialize};
use php_value::{Key, PhpArray, PhpString, Thrown, Value};

/// The name PHP gives a value in a `TypeError` (`zend_zval_value_name()`).
pub(crate) fn value_name(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(true) => "true".to_owned(),
        Value::Bool(false) => "false".to_owned(),
        Value::Int(_) => "int".to_owned(),
        Value::Float(_) => "float".to_owned(),
        Value::String(_) => "string".to_owned(),
        Value::Array(_) => "array".to_owned(),
        Value::Object(object) => object.class().to_owned(),
    }
}

/// `Cannot assign <value> to property <class>::$<property> of type <type>`.
pub(crate) fn cannot_assign(value: &Value, class: &str, property: &str, ty: &str) -> Thrown {
    Thrown::new(
        "TypeError",
        format!(
            "Cannot assign {} to property {class}::${property} of type {ty}",
            value_name(value)
        ),
        0,
    )
}

/// A string as Rust text (mapping names are UTF-8 in practice).
pub(crate) fn text(string: &PhpString) -> String {
    String::from_utf8_lossy(string.as_bytes()).into_owned()
}

pub(crate) fn string(value: &Value, class: &str, property: &str) -> Result<String, Thrown> {
    match value {
        Value::String(s) => Ok(text(s)),
        _ => Err(cannot_assign(value, class, property, "string")),
    }
}

pub(crate) fn opt_string(
    value: &Value,
    class: &str,
    property: &str,
) -> Result<Option<String>, Thrown> {
    match value {
        Value::Null => Ok(None),
        Value::String(s) => Ok(Some(text(s))),
        _ => Err(cannot_assign(value, class, property, "?string")),
    }
}

pub(crate) fn opt_bool(value: &Value, class: &str, property: &str) -> Result<Option<bool>, Thrown> {
    match value {
        Value::Null => Ok(None),
        Value::Bool(b) => Ok(Some(*b)),
        _ => Err(cannot_assign(value, class, property, "?bool")),
    }
}

pub(crate) fn bool(value: &Value, class: &str, property: &str) -> Result<bool, Thrown> {
    match value {
        Value::Bool(b) => Ok(*b),
        _ => Err(cannot_assign(value, class, property, "bool")),
    }
}

pub(crate) fn opt_int(value: &Value, class: &str, property: &str) -> Result<Option<i64>, Thrown> {
    match value {
        Value::Null => Ok(None),
        Value::Int(i) => Ok(Some(*i)),
        _ => Err(cannot_assign(value, class, property, "?int")),
    }
}

pub(crate) fn array(value: &Value, class: &str, property: &str) -> Result<PhpArray, Thrown> {
    match value {
        Value::Array(a) => Ok(a.clone()),
        _ => Err(cannot_assign(value, class, property, "array")),
    }
}

pub(crate) fn opt_array(
    value: &Value,
    class: &str,
    property: &str,
) -> Result<Option<PhpArray>, Thrown> {
    match value {
        Value::Null => Ok(None),
        Value::Array(a) => Ok(Some(a.clone())),
        _ => Err(cannot_assign(value, class, property, "?array")),
    }
}

/// A list of strings (`array` properties such as `cascade`).
pub(crate) fn strings(value: &Value, class: &str, property: &str) -> Result<Vec<String>, Thrown> {
    let array = array(value, class, property)?;
    Ok(array
        .values()
        .map(|value| match value {
            Value::String(s) => text(s),
            other => other.to_php_string().map_or_default(|s| text(&s)),
        })
        .collect())
}

pub(crate) fn from_opt_string(value: Option<&String>) -> Value {
    value.map_or(Value::Null, |s| Value::from(s.as_str()))
}

pub(crate) fn from_opt_bool(value: Option<bool>) -> Value {
    value.map_or(Value::Null, Value::Bool)
}

pub(crate) fn from_opt_int(value: Option<i64>) -> Value {
    value.map_or(Value::Null, Value::Int)
}

pub(crate) fn from_opt_array(value: Option<&PhpArray>) -> Value {
    value.map_or(Value::Null, |a| Value::Array(a.clone()))
}

pub(crate) fn from_strings(values: &[String]) -> Value {
    Value::Array(PhpArray::list(
        values.iter().map(|s| Value::from(s.as_str())),
    ))
}

/// The value at `key` of a mapping array.
pub(crate) fn get<'a>(array: &'a PhpArray, key: &str) -> Option<&'a Value> {
    array.get(&Key::from(key))
}

/// `isset($array[$key])`.
pub(crate) fn isset(array: &PhpArray, key: &str) -> bool {
    get(array, key).is_some_and(|value| !matches!(value, Value::Null))
}

/// `$array[$key]` that must be there: PHP reads null and warns when it
/// is not, which the typed constructor parameter then rejects.
pub(crate) fn required<'a>(array: &'a PhpArray, key: &str) -> &'a Value {
    get(array, key).unwrap_or(&Value::Null)
}

/// The key as PHP's property name.
pub(crate) fn key_name(key: &Key) -> String {
    String::from_utf8_lossy(&key.bytes()).into_owned()
}

/// PHP's `empty()`.
pub(crate) fn empty(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null | Value::Bool(false)) => true,
        Some(Value::Bool(true) | Value::Object(_)) => false,
        Some(Value::Int(i)) => *i == 0,
        Some(Value::Float(f)) => *f == 0.0,
        Some(Value::String(s)) => s.as_bytes().is_empty() || s.as_bytes() == b"0",
        Some(Value::Array(a)) => a.is_empty(),
    }
}

/// A value object PHP serializes through `__sleep()`.
pub trait Sleep {
    /// The class name.
    fn class_name(&self) -> String;
    /// The properties `__sleep()` names, in order, with their values.
    fn sleep(&self) -> PhpArray;
}

/// `serialize($object)`.
pub fn serialize_object(object: &dyn Sleep) -> PhpString {
    serialize(&Value::Object(Arc::new(GenericObject::new(
        object.class_name(),
        object.sleep(),
    ))))
}

/// `unserialize($data)` limited to the given classes: the class and the
/// properties of the object it gives.
pub(crate) fn unserialize_object(data: &[u8], classes: &[&str]) -> Option<(String, PhpArray)> {
    let mut allowed = Classes::new();
    for class in classes {
        allowed.allow(class);
    }
    match unserialize(data, &allowed, &php_value::constants::Registry::default())? {
        Value::Object(object) => Some((object.class().to_owned(), object.properties())),
        _ => None,
    }
}
