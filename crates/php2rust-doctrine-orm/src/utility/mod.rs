//! `Doctrine\ORM\Utility`: the persister helpers and the identifier
//! flattener, and small conversions between PHP values and Rust text.
//!
//! Upstream: none

mod identifier_flattener;
mod persister_helper;

pub(crate) use identifier_flattener::backed_value as identifier_flattener_backed;
pub use identifier_flattener::flatten_identifier;
pub use persister_helper::{
    ParamType, convert_to_parameter_value, get_type_of_column, get_type_of_field,
    infer_parameter_types,
};

use php_value::{Key, PhpArray, Value};

/// `(string) $value` for a scalar (`''` for `null`, `false` and arrays
/// and objects without `__toString()`).
pub fn to_text(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(false) => String::new(),
        Value::Bool(true) => "1".to_owned(),
        Value::Int(i) => i.to_string(),
        other => other
            .to_php_string_or_stringable()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
    }
}

/// A key as text.
pub fn key_text(key: &Key) -> String {
    to_text(&key.to_value())
}

/// `implode(' ', $values)`.
pub fn implode_values(values: &PhpArray) -> String {
    values.values().map(to_text).collect::<Vec<_>>().join(" ")
}

/// The string values of a PHP array of strings (a column map), as pairs.
pub fn string_pairs(array: &PhpArray) -> Vec<(String, String)> {
    array
        .iter()
        .map(|(k, v)| (key_text(k), to_text(v)))
        .collect()
}
