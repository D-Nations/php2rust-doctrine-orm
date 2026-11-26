//! Upstream: doctrine/collections src/AbstractLazyCollection.php

use std::fmt;

use php_value::{Key, PhpArray, Thrown, Value};

use crate::{Collection, Criteria, Partition, ReadableCollection, Selectable};

/// The `doInitialize()` of a class extending `AbstractLazyCollection`.
pub trait DoInitialize: fmt::Debug + Send + Sync {
    /// `doInitialize()`: sets the collection (`$this->collection`).
    fn do_initialize(&mut self, collection: &mut Option<Box<dyn Collection>>)
    -> Result<(), Thrown>;
}

/// `AbstractLazyCollection`: a collection that loads its elements (into a
/// wrapped collection) the first time it is used, and then delegates
/// every method to it. `T` is the extending class, with its
/// `doInitialize()`.
#[derive(Debug)]
pub struct AbstractLazyCollection<T: DoInitialize> {
    collection: Option<Box<dyn Collection>>,
    initialized: bool,
    class: T,
}

impl<T: DoInitialize> AbstractLazyCollection<T> {
    /// A collection of the extending class `class`, not initialized.
    pub fn new(class: T) -> Self {
        Self {
            collection: None,
            initialized: false,
            class,
        }
    }

    /// The extending class.
    pub fn class(&self) -> &T {
        &self.class
    }

    /// The extending class, to change.
    pub fn class_mut(&mut self) -> &mut T {
        &mut self.class
    }

    /// The wrapped collection (`$this->collection`).
    pub fn collection(&self) -> Option<&dyn Collection> {
        self.collection.as_deref()
    }

    /// The wrapped collection (`$this->collection`), to change.
    pub fn collection_mut(&mut self) -> &mut Option<Box<dyn Collection>> {
        &mut self.collection
    }

    /// `isInitialized()`.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// `initialize()`: runs `doInitialize()` once; a `LogicException` when
    /// it left no collection (the collection then counts as initialized,
    /// as in PHP).
    pub fn initialize(&mut self) -> Result<(), Thrown> {
        if self.initialized {
            return Ok(());
        }
        self.class.do_initialize(&mut self.collection)?;
        self.initialized = true;
        if self.collection.is_none() {
            return Err(Thrown::new(
                "LogicException",
                "You must initialize the collection property in the doInitialize() method.",
                0,
            ));
        }
        Ok(())
    }

    /// The wrapped collection after `initialize()`, for a call of
    /// `method` on it (PHP's `Error` on a `null` collection).
    fn inner(&mut self, method: &str) -> Result<&mut dyn Collection, Thrown> {
        self.initialize()?;
        match self.collection.as_deref_mut() {
            Some(collection) => Ok(collection),
            None => Err(Thrown::new(
                "Error",
                format!("Call to a member function {method}() on null"),
                0,
            )),
        }
    }
}

impl<T: DoInitialize> Selectable for AbstractLazyCollection<T> {
    fn matching(&mut self, criteria: &Criteria) -> Result<Box<dyn Collection>, Thrown> {
        self.inner("matching")?.matching(criteria)
    }
}

impl<T: DoInitialize> ReadableCollection for AbstractLazyCollection<T> {
    fn contains(&mut self, element: &Value) -> Result<bool, Thrown> {
        self.inner("contains")?.contains(element)
    }

    fn is_empty(&mut self) -> Result<bool, Thrown> {
        self.inner("isEmpty")?.is_empty()
    }

    fn contains_key(&mut self, key: &Key) -> Result<bool, Thrown> {
        self.inner("containsKey")?.contains_key(key)
    }

    fn get(&mut self, key: &Key) -> Result<Value, Thrown> {
        self.inner("get")?.get(key)
    }

    fn get_keys(&mut self) -> Result<PhpArray, Thrown> {
        self.inner("getKeys")?.get_keys()
    }

    fn get_values(&mut self) -> Result<PhpArray, Thrown> {
        self.inner("getValues")?.get_values()
    }

    fn to_array(&mut self) -> Result<PhpArray, Thrown> {
        self.inner("toArray")?.to_array()
    }

    fn first(&mut self) -> Result<Value, Thrown> {
        self.inner("first")?.first()
    }

    fn last(&mut self) -> Result<Value, Thrown> {
        self.inner("last")?.last()
    }

    fn key(&mut self) -> Result<Option<Key>, Thrown> {
        self.inner("key")?.key()
    }

    fn current(&mut self) -> Result<Value, Thrown> {
        self.inner("current")?.current()
    }

    fn next(&mut self) -> Result<Value, Thrown> {
        self.inner("next")?.next()
    }

    fn slice(&mut self, offset: i64, length: Option<i64>) -> Result<PhpArray, Thrown> {
        self.inner("slice")?.slice(offset, length)
    }

    fn exists(&mut self, p: &mut dyn FnMut(&Key, &Value) -> bool) -> Result<bool, Thrown> {
        self.inner("exists")?.exists(p)
    }

    fn filter(
        &mut self,
        p: &mut dyn FnMut(&Value, &Key) -> bool,
    ) -> Result<Box<dyn Collection>, Thrown> {
        self.inner("filter")?.filter(p)
    }

    fn map(
        &mut self,
        func: &mut dyn FnMut(&Value) -> Value,
    ) -> Result<Box<dyn Collection>, Thrown> {
        self.inner("map")?.map(func)
    }

    fn partition(&mut self, p: &mut dyn FnMut(&Key, &Value) -> bool) -> Result<Partition, Thrown> {
        self.inner("partition")?.partition(p)
    }

    fn for_all(&mut self, p: &mut dyn FnMut(&Key, &Value) -> bool) -> Result<bool, Thrown> {
        self.inner("forAll")?.for_all(p)
    }

    fn index_of(&mut self, element: &Value) -> Result<Option<Key>, Thrown> {
        self.inner("indexOf")?.index_of(element)
    }

    fn find_first(&mut self, p: &mut dyn FnMut(&Key, &Value) -> bool) -> Result<Value, Thrown> {
        self.inner("findFirst")?.find_first(p)
    }

    fn reduce(
        &mut self,
        func: &mut dyn FnMut(Value, &Value) -> Value,
        initial: Value,
    ) -> Result<Value, Thrown> {
        self.inner("reduce")?.reduce(func, initial)
    }

    fn count(&mut self) -> Result<usize, Thrown> {
        self.inner("count")?.count()
    }

    fn get_iterator(&mut self) -> Result<PhpArray, Thrown> {
        self.inner("getIterator")?.get_iterator()
    }
}

impl<T: DoInitialize> Collection for AbstractLazyCollection<T> {
    fn add(&mut self, element: Value) -> Result<(), Thrown> {
        self.inner("add")?.add(element)
    }

    fn clear(&mut self) -> Result<(), Thrown> {
        self.inner("clear")?.clear()
    }

    fn remove(&mut self, key: &Key) -> Result<Value, Thrown> {
        self.inner("remove")?.remove(key)
    }

    fn remove_element(&mut self, element: &Value) -> Result<bool, Thrown> {
        self.inner("removeElement")?.remove_element(element)
    }

    fn set(&mut self, key: Key, value: Value) -> Result<(), Thrown> {
        self.inner("set")?.set(key, value)
    }

    fn offset_exists(&mut self, offset: &Key) -> Result<bool, Thrown> {
        self.inner("offsetExists")?.offset_exists(offset)
    }

    fn offset_get(&mut self, offset: &Key) -> Result<Value, Thrown> {
        self.inner("offsetGet")?.offset_get(offset)
    }

    fn offset_set(&mut self, offset: Option<Key>, value: Value) -> Result<(), Thrown> {
        self.inner("offsetSet")?.offset_set(offset, value)
    }

    fn offset_unset(&mut self, offset: &Key) -> Result<(), Thrown> {
        self.inner("offsetUnset")?.offset_unset(offset)
    }
}
