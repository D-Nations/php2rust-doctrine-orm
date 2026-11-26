//! Upstream: doctrine/collections src/ReadableCollection.php

use std::fmt;

use php_value::{Key, PhpArray, Thrown, Value};

use crate::{Collection, Selectable};

/// What `partition()` returns: the matches, then the others.
pub type Partition = (Box<dyn Collection>, Box<dyn Collection>);

/// `ReadableCollection`: the reading half of a collection (with
/// `Countable`, `IteratorAggregate` and `Selectable`).
///
/// Every method can fail, as a lazy collection initializes itself first
/// (and so may throw); the methods take `&mut self` for the same reason
/// and because `first()`, `last()` and `next()` move the internal
/// pointer. `false` and `null` results are PHP's: `first()`, `last()`,
/// `current()` and `next()` give `false` past the elements, `get()` and
/// `findFirst()` give `null` for nothing.
pub trait ReadableCollection: Selectable + fmt::Debug + Send + Sync {
    /// `contains($element)`: an element identical (`===`) to it.
    fn contains(&mut self, element: &Value) -> Result<bool, Thrown>;

    /// `isEmpty()`.
    fn is_empty(&mut self) -> Result<bool, Thrown>;

    /// `containsKey($key)`.
    fn contains_key(&mut self, key: &Key) -> Result<bool, Thrown>;

    /// `get($key)`: the element, or `null`.
    fn get(&mut self, key: &Key) -> Result<Value, Thrown>;

    /// `getKeys()`: a list of the keys.
    fn get_keys(&mut self) -> Result<PhpArray, Thrown>;

    /// `getValues()`: a list of the elements.
    fn get_values(&mut self) -> Result<PhpArray, Thrown>;

    /// `toArray()`.
    fn to_array(&mut self) -> Result<PhpArray, Thrown>;

    /// `first()`: moves the internal pointer to the first element and
    /// returns it, or `false`.
    fn first(&mut self) -> Result<Value, Thrown>;

    /// `last()`: moves the internal pointer to the last element and
    /// returns it, or `false`.
    fn last(&mut self) -> Result<Value, Thrown>;

    /// `key()`: the key at the internal pointer, or `null`.
    fn key(&mut self) -> Result<Option<Key>, Thrown>;

    /// `current()`: the element at the internal pointer, or `false`.
    fn current(&mut self) -> Result<Value, Thrown>;

    /// `next()`: moves the internal pointer on and returns its element, or
    /// `false`.
    fn next(&mut self) -> Result<Value, Thrown>;

    /// `slice($offset, $length)`: `array_slice()` keeping the keys.
    fn slice(&mut self, offset: i64, length: Option<i64>) -> Result<PhpArray, Thrown>;

    /// `exists($p)`: whether `$p($key, $element)` holds for an element.
    fn exists(&mut self, p: &mut dyn FnMut(&Key, &Value) -> bool) -> Result<bool, Thrown>;

    /// `filter($p)`: the elements for which `$p($element, $key)` holds,
    /// keys kept.
    fn filter(
        &mut self,
        p: &mut dyn FnMut(&Value, &Key) -> bool,
    ) -> Result<Box<dyn Collection>, Thrown>;

    /// `map($func)`: `$func($element)` for each element, keys kept.
    fn map(&mut self, func: &mut dyn FnMut(&Value) -> Value)
    -> Result<Box<dyn Collection>, Thrown>;

    /// `partition($p)`: the elements for which `$p($key, $element)` holds,
    /// and the others, keys kept.
    fn partition(&mut self, p: &mut dyn FnMut(&Key, &Value) -> bool) -> Result<Partition, Thrown>;

    /// `forAll($p)`: whether `$p($key, $element)` holds for every element.
    fn for_all(&mut self, p: &mut dyn FnMut(&Key, &Value) -> bool) -> Result<bool, Thrown>;

    /// `indexOf($element)`: the key of the first element identical to it;
    /// `None` for `false`.
    fn index_of(&mut self, element: &Value) -> Result<Option<Key>, Thrown>;

    /// `findFirst($p)`: the first element for which `$p($key, $element)`
    /// holds, or `null`.
    fn find_first(&mut self, p: &mut dyn FnMut(&Key, &Value) -> bool) -> Result<Value, Thrown>;

    /// `reduce($func, $initial)`: `array_reduce()`, `$func($carry,
    /// $element)`.
    fn reduce(
        &mut self,
        func: &mut dyn FnMut(Value, &Value) -> Value,
        initial: Value,
    ) -> Result<Value, Thrown>;

    /// `count()`.
    fn count(&mut self) -> Result<usize, Thrown>;

    /// `getIterator()`: the elements to iterate (PHP's `ArrayIterator`
    /// over a copy of them).
    fn get_iterator(&mut self) -> Result<PhpArray, Thrown>;
}
