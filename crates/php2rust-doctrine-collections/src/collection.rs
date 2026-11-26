//! Upstream: doctrine/collections src/Collection.php

use php_value::{Key, Thrown, Value};

use crate::ReadableCollection;

/// `Collection`: a `ReadableCollection` that can be changed (with
/// `ArrayAccess`).
pub trait Collection: ReadableCollection {
    /// `add($element)`: appends it (PHP's `Error` when the next int key is
    /// taken, after `PHP_INT_MAX`).
    fn add(&mut self, element: Value) -> Result<(), Thrown>;

    /// `clear()`.
    fn clear(&mut self) -> Result<(), Thrown>;

    /// `remove($key)`: the removed element, or `null`.
    fn remove(&mut self, key: &Key) -> Result<Value, Thrown>;

    /// `removeElement($element)`: removes the first element identical to
    /// it.
    fn remove_element(&mut self, element: &Value) -> Result<bool, Thrown>;

    /// `set($key, $value)`.
    fn set(&mut self, key: Key, value: Value) -> Result<(), Thrown>;

    /// `isset($collection[$offset])` (`offsetExists()`).
    fn offset_exists(&mut self, offset: &Key) -> Result<bool, Thrown>;

    /// `$collection[$offset]` (`offsetGet()`).
    fn offset_get(&mut self, offset: &Key) -> Result<Value, Thrown>;

    /// `$collection[$offset] = $value`, or `$collection[] = $value` for
    /// `None` (`offsetSet()`).
    fn offset_set(&mut self, offset: Option<Key>, value: Value) -> Result<(), Thrown>;

    /// `unset($collection[$offset])` (`offsetUnset()`).
    fn offset_unset(&mut self, offset: &Key) -> Result<(), Thrown>;
}
