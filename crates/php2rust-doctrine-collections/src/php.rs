//! The PHP array functions the package calls, on `php_value` arrays:
//! `in_array()`, `array_search()`, `array_slice()` with preserved keys,
//! the stable sort under `uasort()`/`usort()`, `iterator_to_array()`, and
//! calling a value as `array_filter()`'s callback.
//!
//! Upstream: none

use php_value::compare::loose_equals;
use php_value::object::Closure;
use php_value::{Key, PhpArray, Thrown, Value};

/// `$a === $b` (`$strict`) or `$a == $b`.
fn matches(a: &Value, b: &Value, strict: bool) -> bool {
    if strict {
        a.identical(b)
    } else {
        loose_equals(a, b)
    }
}

/// `in_array($needle, $haystack, $strict)`.
pub(crate) fn in_array(needle: &Value, haystack: &PhpArray, strict: bool) -> bool {
    haystack
        .values()
        .any(|value| matches(value, needle, strict))
}

/// `array_search($needle, $haystack, $strict)`; `None` for `false`.
pub(crate) fn array_search(needle: &Value, haystack: &PhpArray, strict: bool) -> Option<Key> {
    haystack
        .iter()
        .find(|(_, value)| matches(value, needle, strict))
        .map(|(key, _)| key.clone())
}

/// `array_slice($array, $offset, $length, true)`.
pub(crate) fn array_slice(array: &PhpArray, offset: i64, length: Option<i64>) -> PhpArray {
    let count = i64::try_from(array.len()).unwrap_or(i64::MAX);
    if offset > count {
        return PhpArray::new();
    }
    let start = if offset < 0 {
        count.saturating_add(offset).max(0)
    } else {
        offset
    };
    let length = match length {
        None => count - start,
        Some(length) if length < 0 => (count - start).saturating_add(length),
        Some(length) => length,
    };
    if length <= 0 {
        return PhpArray::new();
    }
    let start = usize::try_from(start).unwrap_or(usize::MAX);
    let length = usize::try_from(length).unwrap_or(usize::MAX);
    let mut slice = PhpArray::new();
    for (key, value) in array.iter().skip(start).take(length) {
        slice.insert(key.clone(), value.clone());
    }
    slice
}

/// A stable sort by a PHP comparison callback (negative: `a` first,
/// positive: `b` first, zero: keep their order), as PHP 8's sorts are
/// stable; the first error the callback returns stops the sort. A
/// merge sort: for a callback that is a consistent order, the result is
/// PHP's; for one that is not, the comparisons made (and so the order)
/// can differ from PHP's.
pub(crate) fn stable_sort<T: Clone>(
    items: &mut Vec<T>,
    compare: &mut dyn FnMut(&T, &T) -> Result<i64, Thrown>,
) -> Result<(), Thrown> {
    if items.len() < 2 {
        return Ok(());
    }
    let right = items.split_off(items.len() / 2);
    let mut left = std::mem::take(items);
    let mut right = right;
    stable_sort(&mut left, compare)?;
    stable_sort(&mut right, compare)?;
    let mut merged = Vec::with_capacity(left.len() + right.len());
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        if compare(&left[i], &right[j])? > 0 {
            merged.push(right[j].clone());
            j += 1;
        } else {
            merged.push(left[i].clone());
            i += 1;
        }
    }
    merged.extend_from_slice(&left[i..]);
    merged.extend_from_slice(&right[j..]);
    *items = merged;
    Ok(())
}

/// `iterator_to_array($value)` (keys preserved, a later key replacing an
/// earlier one): an array as it is, a `\Traversable` object's pairs.
pub(crate) fn iterator_to_array(value: &Value) -> Result<PhpArray, Thrown> {
    match value {
        Value::Array(array) => Ok(array.clone()),
        Value::Object(object) => match object.traverse() {
            Some(pairs) => {
                let mut array = PhpArray::new();
                for (key, value) in pairs? {
                    let Some(key) = Key::from_value(&key) else {
                        return Err(Thrown::new(
                            "TypeError",
                            format!("Cannot access offset of type {} on array", key.debug_type()),
                            0,
                        ));
                    };
                    array.insert(key, value);
                }
                Ok(array)
            }
            None => Err(iterator_to_array_type_error(value)),
        },
        _ => Err(iterator_to_array_type_error(value)),
    }
}

fn iterator_to_array_type_error(value: &Value) -> Thrown {
    Thrown::new(
        "TypeError",
        format!(
            "iterator_to_array(): Argument #1 ($iterator) must be of type Traversable|array, {} given",
            value.value_name()
        ),
        0,
    )
}

/// Whether `array_filter($array, $callback)` keeps `element`: with a
/// `null` callback, its truthiness; with a `\Closure` or an object with
/// `__invoke()` or an `[$object, 'method']` array, the truthiness of the
/// call. Function and class names are not resolved here: a string is
/// reported as PHP reports an unknown function, a `['Class', 'method']`
/// array as PHP reports an unknown class.
pub(crate) fn array_filter_callback(callback: &Value, element: &Value) -> Result<bool, Thrown> {
    const PREFIX: &str =
        "array_filter(): Argument #2 ($callback) must be a valid callback or null, ";
    let invalid = |reason: String| Thrown::new("TypeError", format!("{PREFIX}{reason}"), 0);
    match callback {
        Value::Null => Ok(element.is_truthy()),
        Value::Object(object) => {
            if let Some(closure) = object.downcast_ref::<Closure>() {
                return Ok(closure.call(std::slice::from_ref(element)).is_truthy());
            }
            match object.call_method("__invoke", std::slice::from_ref(element)) {
                Some(result) => Ok(result?.is_truthy()),
                None => Err(invalid("no array or string given".to_owned())),
            }
        }
        Value::String(name) => Err(invalid(format!(
            "function \"{}\" not found or invalid function name",
            name.to_string_lossy()
        ))),
        Value::Array(array) if array.len() != 2 => Err(invalid(
            "array callback must have exactly two members".to_owned(),
        )),
        Value::Array(array) => {
            let (Some(target), Some(method)) = (array.get(&Key::Int(0)), array.get(&Key::Int(1)))
            else {
                return Err(invalid(
                    "array callback has to contain indices 0 and 1".to_owned(),
                ));
            };
            if !matches!(target, Value::Object(_) | Value::String(_)) {
                return Err(invalid(
                    "first array member is not a valid class name or object".to_owned(),
                ));
            }
            let Value::String(method) = method else {
                return Err(invalid(
                    "second array member is not a valid method".to_owned(),
                ));
            };
            let name = method.to_string_lossy();
            match target {
                Value::Object(object) => {
                    match object.call_method(&name, std::slice::from_ref(element)) {
                        Some(result) => Ok(result?.is_truthy()),
                        None => Err(invalid(format!(
                            "class {} does not have a method \"{name}\"",
                            object.class()
                        ))),
                    }
                }
                _ => Err(invalid(format!(
                    "class \"{}\" not found",
                    target.to_php_string().unwrap_or_default().to_string_lossy()
                ))),
            }
        }
        _ => Err(invalid("no array or string given".to_owned())),
    }
}
