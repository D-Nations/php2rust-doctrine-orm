//! Upstream: doctrine/collections src/ArrayCollection.php

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use php_value::{Key, PhpArray, Thrown, Value};

use crate::expr::{ClosureExpressionVisitor, ExpressionVisitor};
use crate::php::{array_search, array_slice, in_array, stable_sort};
use crate::{Collection, Criteria, Partition, ReadableCollection, Selectable, SortDirection};

/// What a class extending `ArrayCollection` keeps, so that
/// `createFrom()` (`new static(...)`) builds the same class: the methods
/// that make a new collection (`map()`, `filter()`, `partition()`,
/// `matching()`) give it a clone of the original's. `()` is
/// `ArrayCollection` itself.
pub trait ArrayCollectionClass: Clone + fmt::Debug + Send + Sync + 'static {}

impl ArrayCollectionClass for () {}

/// Object handles for `spl_object_hash()`.
static HANDLES: AtomicU64 = AtomicU64::new(1);

fn next_handle() -> u64 {
    HANDLES.fetch_add(1, Ordering::Relaxed)
}

/// `ArrayCollection`: a collection over a PHP array, with the array's key
/// rules, order and internal pointer.
///
/// The internal pointer is kept as PHP keeps it: removing the element it
/// points to moves it to the next one, and past the end it points to
/// where an appended element goes. A new collection's pointer is at the
/// first element (a `PhpArray` has no pointer of its own to copy), except
/// that `matching()` without a filter, orderings or slice copies the
/// collection's own, as PHP's array copy does.
#[derive(Debug)]
pub struct ArrayCollection<C: ArrayCollectionClass = ()> {
    elements: PhpArray,
    /// The internal pointer, as the number of elements before it.
    pointer: usize,
    class: C,
    handle: u64,
}

impl Default for ArrayCollection {
    fn default() -> Self {
        Self::new(PhpArray::new())
    }
}

impl<C: ArrayCollectionClass> Clone for ArrayCollection<C> {
    /// `clone $collection`: a new object (a new handle).
    fn clone(&self) -> Self {
        Self {
            elements: self.elements.clone(),
            pointer: self.pointer,
            class: self.class.clone(),
            handle: next_handle(),
        }
    }
}

impl ArrayCollection {
    /// `new ArrayCollection($elements)`.
    pub fn new(elements: PhpArray) -> Self {
        Self::with_class((), elements)
    }
}

fn occupied() -> Thrown {
    Thrown::new(
        "Error",
        "Cannot add element to the array as the next element is already occupied",
        0,
    )
}

impl<C: ArrayCollectionClass> ArrayCollection<C> {
    /// The constructor of a class extending `ArrayCollection`, keeping
    /// `class`.
    pub fn with_class(class: C, elements: PhpArray) -> Self {
        Self {
            elements,
            pointer: 0,
            class,
            handle: next_handle(),
        }
    }

    /// What the extending class keeps.
    pub fn class(&self) -> &C {
        &self.class
    }

    /// `createFrom($elements)`: `new static($elements)`.
    fn create_from(&self, elements: PhpArray) -> Self {
        Self::with_class(self.class.clone(), elements)
    }

    /// `toArray()`.
    pub fn to_array(&self) -> PhpArray {
        self.elements.clone()
    }

    /// `first()`: `reset()`.
    pub fn first(&mut self) -> Value {
        self.pointer = 0;
        Self::current(self)
    }

    /// `last()`: `end()`.
    pub fn last(&mut self) -> Value {
        self.pointer = self.elements.len().saturating_sub(1);
        Self::current(self)
    }

    fn at_pointer(&self) -> Option<(&Key, &Value)> {
        self.elements.iter().nth(self.pointer)
    }

    /// `key()`.
    pub fn key(&self) -> Option<Key> {
        self.at_pointer().map(|(key, _)| key.clone())
    }

    /// `next()`.
    #[allow(
        clippy::should_implement_trait,
        reason = "PHP's next(), not an iterator"
    )]
    pub fn next(&mut self) -> Value {
        if self.pointer < self.elements.len() {
            self.pointer += 1;
        }
        Self::current(self)
    }

    /// `current()`.
    pub fn current(&self) -> Value {
        self.at_pointer()
            .map_or(Value::Bool(false), |(_, value)| value.clone())
    }

    /// `unset($this->elements[$key])`, keeping the internal pointer on its
    /// element (or on the next one, for the removed element).
    fn unset(&mut self, key: &Key) -> Option<Value> {
        let position = self.elements.keys().position(|k| k == key)?;
        let removed = self.elements.remove(key)?;
        if position < self.pointer {
            self.pointer -= 1;
        }
        Some(removed)
    }

    /// `remove($key)`: the removed element, or `null`.
    pub fn remove(&mut self, key: &Key) -> Value {
        self.unset(key).unwrap_or_default()
    }

    /// `removeElement($element)`: removes the first element identical to
    /// it.
    pub fn remove_element(&mut self, element: &Value) -> bool {
        match array_search(element, &self.elements, true) {
            Some(key) => {
                self.unset(&key);
                true
            }
            None => false,
        }
    }

    /// `offsetExists($offset)`.
    pub fn offset_exists(&self, offset: &Key) -> bool {
        self.contains_key(offset)
    }

    /// `offsetGet($offset)`.
    pub fn offset_get(&self, offset: &Key) -> Value {
        self.get(offset)
    }

    /// `offsetSet($offset, $value)`: `add()` for `None`, else `set()`.
    pub fn offset_set(&mut self, offset: Option<Key>, value: Value) -> Result<(), Thrown> {
        match offset {
            None => self.add(value),
            Some(key) => {
                self.set(key, value);
                Ok(())
            }
        }
    }

    /// `offsetUnset($offset)`.
    pub fn offset_unset(&mut self, offset: &Key) {
        self.remove(offset);
    }

    /// `containsKey($key)`.
    pub fn contains_key(&self, key: &Key) -> bool {
        self.elements.contains_key(key)
    }

    /// `contains($element)`: an element identical (`===`) to it.
    pub fn contains(&self, element: &Value) -> bool {
        in_array(element, &self.elements, true)
    }

    /// `exists($p)`: whether `$p($key, $element)` holds for an element.
    pub fn exists(&self, mut p: impl FnMut(&Key, &Value) -> bool) -> bool {
        self.elements.iter().any(|(key, element)| p(key, element))
    }

    /// `indexOf($element)`: `None` for `false`.
    pub fn index_of(&self, element: &Value) -> Option<Key> {
        array_search(element, &self.elements, true)
    }

    /// `get($key)`: the element, or `null`.
    pub fn get(&self, key: &Key) -> Value {
        self.elements.get(key).cloned().unwrap_or_default()
    }

    /// `getKeys()`.
    pub fn get_keys(&self) -> PhpArray {
        PhpArray::list(self.elements.keys().map(Key::to_value))
    }

    /// `getValues()`.
    pub fn get_values(&self) -> PhpArray {
        PhpArray::list(self.elements.values().cloned())
    }

    /// `count()`.
    pub fn count(&self) -> usize {
        self.elements.len()
    }

    /// `set($key, $value)`: an existing key keeps its place.
    pub fn set(&mut self, key: impl Into<Key>, value: Value) {
        self.elements.insert(key.into(), value);
    }

    /// `add($element)`: `$this->elements[] = $element`.
    pub fn add(&mut self, element: Value) -> Result<(), Thrown> {
        self.elements
            .push(element)
            .map(|_| ())
            .map_err(|_| occupied())
    }

    /// `isEmpty()`.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// `getIterator()`: a copy of the elements (PHP's `ArrayIterator` over
    /// one).
    pub fn get_iterator(&self) -> PhpArray {
        self.elements.clone()
    }

    /// `map($func)`: `array_map()`, keys kept.
    #[must_use]
    pub fn map(&self, mut func: impl FnMut(&Value) -> Value) -> Self {
        let mut mapped = PhpArray::new();
        for (key, element) in self.elements.iter() {
            mapped.insert(key.clone(), func(element));
        }
        self.create_from(mapped)
    }

    /// `reduce($func, $initial)`: `array_reduce()`.
    pub fn reduce(&self, func: impl FnMut(Value, &Value) -> Value, initial: Value) -> Value {
        self.elements.values().fold(initial, func)
    }

    /// `filter($p)`: `array_filter(..., ARRAY_FILTER_USE_BOTH)`,
    /// `$p($element, $key)`.
    #[must_use]
    pub fn filter(&self, mut p: impl FnMut(&Value, &Key) -> bool) -> Self {
        let mut filtered = PhpArray::new();
        for (key, element) in self.elements.iter() {
            if p(element, key) {
                filtered.insert(key.clone(), element.clone());
            }
        }
        self.create_from(filtered)
    }

    /// `findFirst($p)`: the first element for which `$p($key, $element)`
    /// holds, or `null`.
    pub fn find_first(&self, mut p: impl FnMut(&Key, &Value) -> bool) -> Value {
        self.elements
            .iter()
            .find(|(key, element)| p(key, element))
            .map_or_default(|(_, element)| element.clone())
    }

    /// `forAll($p)`.
    pub fn for_all(&self, mut p: impl FnMut(&Key, &Value) -> bool) -> bool {
        self.elements.iter().all(|(key, element)| p(key, element))
    }

    /// `partition($p)`: the matches and the others, keys kept.
    #[must_use]
    pub fn partition(&self, mut p: impl FnMut(&Key, &Value) -> bool) -> (Self, Self) {
        let (mut matches, mut no_matches) = (PhpArray::new(), PhpArray::new());
        for (key, element) in self.elements.iter() {
            if p(key, element) {
                matches.insert(key.clone(), element.clone());
            } else {
                no_matches.insert(key.clone(), element.clone());
            }
        }
        (self.create_from(matches), self.create_from(no_matches))
    }

    /// `clear()`.
    pub fn clear(&mut self) {
        self.elements = PhpArray::new();
        self.pointer = 0;
    }

    /// `slice($offset, $length)`: `array_slice()` keeping the keys.
    pub fn slice(&self, offset: i64, length: Option<i64>) -> PhpArray {
        array_slice(&self.elements, offset, length)
    }

    /// `matching($criteria)`: filters with the criteria's expression
    /// (`ClosureExpressionVisitor`), sorts stably by its orderings
    /// (`sortByField()`, keys kept) and slices by its first and max
    /// results (when either is positive). Fails where a filter or
    /// comparator throws.
    pub fn matching(&self, criteria: &Criteria) -> Result<Self, Thrown> {
        let mut filtered = self.elements.clone();
        let mut pointer = self.pointer;

        if let Some(expr) = criteria.get_where_expression() {
            let filter = ClosureExpressionVisitor::default().dispatch(expr)?;
            let mut kept = PhpArray::new();
            for (key, element) in filtered.iter() {
                if filter.accepts(element)? {
                    kept.insert(key.clone(), element.clone());
                }
            }
            filtered = kept;
            pointer = 0;
        }

        let orderings = criteria.get_orderings();
        if !orderings.is_empty() {
            let mut next = None;
            for (field, ordering) in orderings.iter().rev() {
                let orientation = if *ordering == SortDirection::Descending {
                    -1
                } else {
                    1
                };
                next = Some(ClosureExpressionVisitor::sort_by_field(
                    field,
                    orientation,
                    next,
                ));
            }
            if let Some(comparator) = next {
                let mut pairs: Vec<(Key, Value)> = filtered
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect();
                stable_sort(&mut pairs, &mut |a, b| comparator(&a.1, &b.1))?;
                filtered = PhpArray::new();
                for (key, value) in pairs {
                    filtered.insert(key, value);
                }
            }
            pointer = 0;
        }

        let offset = criteria.get_first_result();
        let length = criteria.get_max_results();
        if offset.is_some_and(|o| o > 0) || length.is_some_and(|l| l > 0) {
            filtered = array_slice(&filtered, offset.unwrap_or(0), length);
            pointer = 0;
        }

        let mut collection = self.create_from(filtered);
        collection.pointer = pointer;
        Ok(collection)
    }
}

impl<C: ArrayCollectionClass> fmt::Display for ArrayCollection<C> {
    /// `__toString()`: the class name, `@` and `spl_object_hash()` (32
    /// hex digits: the object handle, then 16 zeros). Handles here are
    /// never reused, where PHP reuses a freed one.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Doctrine\\Common\\Collections\\ArrayCollection@{:016x}0000000000000000",
            self.handle
        )
    }
}

impl<C: ArrayCollectionClass> Selectable for ArrayCollection<C> {
    fn matching(&mut self, criteria: &Criteria) -> Result<Box<dyn Collection>, Thrown> {
        Ok(Box::new(ArrayCollection::matching(self, criteria)?))
    }
}

impl<C: ArrayCollectionClass> ReadableCollection for ArrayCollection<C> {
    fn contains(&mut self, element: &Value) -> Result<bool, Thrown> {
        Ok(ArrayCollection::contains(self, element))
    }

    fn is_empty(&mut self) -> Result<bool, Thrown> {
        Ok(ArrayCollection::is_empty(self))
    }

    fn contains_key(&mut self, key: &Key) -> Result<bool, Thrown> {
        Ok(ArrayCollection::contains_key(self, key))
    }

    fn get(&mut self, key: &Key) -> Result<Value, Thrown> {
        Ok(ArrayCollection::get(self, key))
    }

    fn get_keys(&mut self) -> Result<PhpArray, Thrown> {
        Ok(ArrayCollection::get_keys(self))
    }

    fn get_values(&mut self) -> Result<PhpArray, Thrown> {
        Ok(ArrayCollection::get_values(self))
    }

    fn to_array(&mut self) -> Result<PhpArray, Thrown> {
        Ok(ArrayCollection::to_array(self))
    }

    fn first(&mut self) -> Result<Value, Thrown> {
        Ok(ArrayCollection::first(self))
    }

    fn last(&mut self) -> Result<Value, Thrown> {
        Ok(ArrayCollection::last(self))
    }

    fn key(&mut self) -> Result<Option<Key>, Thrown> {
        Ok(ArrayCollection::key(self))
    }

    fn current(&mut self) -> Result<Value, Thrown> {
        Ok(ArrayCollection::current(self))
    }

    fn next(&mut self) -> Result<Value, Thrown> {
        Ok(ArrayCollection::next(self))
    }

    fn slice(&mut self, offset: i64, length: Option<i64>) -> Result<PhpArray, Thrown> {
        Ok(ArrayCollection::slice(self, offset, length))
    }

    fn exists(&mut self, p: &mut dyn FnMut(&Key, &Value) -> bool) -> Result<bool, Thrown> {
        Ok(ArrayCollection::exists(self, p))
    }

    fn filter(
        &mut self,
        p: &mut dyn FnMut(&Value, &Key) -> bool,
    ) -> Result<Box<dyn Collection>, Thrown> {
        Ok(Box::new(ArrayCollection::filter(self, p)))
    }

    fn map(
        &mut self,
        func: &mut dyn FnMut(&Value) -> Value,
    ) -> Result<Box<dyn Collection>, Thrown> {
        Ok(Box::new(ArrayCollection::map(self, func)))
    }

    fn partition(&mut self, p: &mut dyn FnMut(&Key, &Value) -> bool) -> Result<Partition, Thrown> {
        let (matches, no_matches) = ArrayCollection::partition(self, p);
        Ok((Box::new(matches), Box::new(no_matches)))
    }

    fn for_all(&mut self, p: &mut dyn FnMut(&Key, &Value) -> bool) -> Result<bool, Thrown> {
        Ok(ArrayCollection::for_all(self, p))
    }

    fn index_of(&mut self, element: &Value) -> Result<Option<Key>, Thrown> {
        Ok(ArrayCollection::index_of(self, element))
    }

    fn find_first(&mut self, p: &mut dyn FnMut(&Key, &Value) -> bool) -> Result<Value, Thrown> {
        Ok(ArrayCollection::find_first(self, p))
    }

    fn reduce(
        &mut self,
        func: &mut dyn FnMut(Value, &Value) -> Value,
        initial: Value,
    ) -> Result<Value, Thrown> {
        Ok(ArrayCollection::reduce(self, func, initial))
    }

    fn count(&mut self) -> Result<usize, Thrown> {
        Ok(ArrayCollection::count(self))
    }

    fn get_iterator(&mut self) -> Result<PhpArray, Thrown> {
        Ok(ArrayCollection::get_iterator(self))
    }
}

impl<C: ArrayCollectionClass> Collection for ArrayCollection<C> {
    fn add(&mut self, element: Value) -> Result<(), Thrown> {
        ArrayCollection::add(self, element)
    }

    fn clear(&mut self) -> Result<(), Thrown> {
        ArrayCollection::clear(self);
        Ok(())
    }

    fn remove(&mut self, key: &Key) -> Result<Value, Thrown> {
        Ok(ArrayCollection::remove(self, key))
    }

    fn remove_element(&mut self, element: &Value) -> Result<bool, Thrown> {
        Ok(ArrayCollection::remove_element(self, element))
    }

    fn set(&mut self, key: Key, value: Value) -> Result<(), Thrown> {
        ArrayCollection::set(self, key, value);
        Ok(())
    }

    fn offset_exists(&mut self, offset: &Key) -> Result<bool, Thrown> {
        Ok(ArrayCollection::offset_exists(self, offset))
    }

    fn offset_get(&mut self, offset: &Key) -> Result<Value, Thrown> {
        Ok(ArrayCollection::offset_get(self, offset))
    }

    fn offset_set(&mut self, offset: Option<Key>, value: Value) -> Result<(), Thrown> {
        ArrayCollection::offset_set(self, offset, value)
    }

    fn offset_unset(&mut self, offset: &Key) -> Result<(), Thrown> {
        ArrayCollection::offset_unset(self, offset);
        Ok(())
    }
}
