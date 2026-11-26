//! An `ArrayCollection` as a PHP object that an entity's property holds
//! (`$this->phonenumbers = new ArrayCollection()`): collections'
//! `ArrayCollection` behind a lock, shared like PHP's object.
//!
//! The functions below are the `Collection` methods an entity's code and
//! the ORM call on a property value that is either such an object or a
//! [`PersistentCollection`]. The methods that make a persistent
//! collection load its elements are `async` (PHP loads them on the spot).
//!
//! Upstream: none

use std::any::Any;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use php_value::{Key, Object, PhpArray, PhpObject, Thrown, Value};
use php2rust_doctrine_collections::ArrayCollection;

use crate::lazy_criteria_collection::LazyCriteriaCollection;
use crate::persistent_collection::PersistentCollection;

/// `Doctrine\Common\Collections\ArrayCollection`.
pub const ARRAY_COLLECTION: &str = "Doctrine\\Common\\Collections\\ArrayCollection";

const COLLECTION_INTERFACES: [&str; 7] = [
    ARRAY_COLLECTION,
    "Doctrine\\Common\\Collections\\Collection",
    "Doctrine\\Common\\Collections\\ReadableCollection",
    "Doctrine\\Common\\Collections\\Selectable",
    "ArrayAccess",
    "Countable",
    "Traversable",
];

/// A `new ArrayCollection($elements)` object.
#[derive(Debug, Default)]
pub struct ArrayCollectionObject {
    inner: Mutex<ArrayCollection>,
}

impl ArrayCollectionObject {
    pub fn new(elements: PhpArray) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(ArrayCollection::new(elements)),
        })
    }

    /// `new ArrayCollection($elements)` as a property value.
    pub fn value(elements: PhpArray) -> Value {
        Value::Object(Self::new(elements))
    }

    /// The collection, locked.
    pub fn lock(&self) -> MutexGuard<'_, ArrayCollection> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The object behind a value, when it is one.
    pub fn from_value(value: &Value) -> Option<Arc<Self>> {
        match value {
            Value::Object(object) => downcast(object),
            _ => None,
        }
    }
}

/// `$object instanceof T`, as the shared object.
pub fn downcast<T: PhpObject>(object: &Object) -> Option<Arc<T>> {
    let any: Arc<dyn Any + Send + Sync> = object.clone();
    any.downcast::<T>().ok()
}

impl PhpObject for ArrayCollectionObject {
    fn class(&self) -> &str {
        ARRAY_COLLECTION
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }

    fn instance_of(&self, class: &str) -> bool {
        COLLECTION_INTERFACES
            .iter()
            .any(|known| known.eq_ignore_ascii_case(class))
    }

    fn php_count(&self) -> Option<Result<i64, Thrown>> {
        Some(Ok(i64::try_from(self.lock().count()).unwrap_or(i64::MAX)))
    }

    fn traverse(&self) -> Option<Result<Vec<(Value, Value)>, Thrown>> {
        Some(Ok(self
            .lock()
            .to_array()
            .iter()
            .map(|(k, v)| (k.to_value(), v.clone()))
            .collect()))
    }
}

/// A collection-valued property: an `ArrayCollection`, a
/// `PersistentCollection`, or a plain array.
#[derive(Debug, Clone)]
pub enum CollectionRef {
    Array(Arc<ArrayCollectionObject>),
    Persistent(Arc<PersistentCollection>),
    Lazy(Arc<LazyCriteriaCollection>),
}

impl CollectionRef {
    /// The collection behind a property value (`None` for `null` and other
    /// values).
    pub fn of(value: &Value) -> Option<Self> {
        let Value::Object(object) = value else {
            return None;
        };
        if let Some(c) = downcast::<ArrayCollectionObject>(object) {
            return Some(Self::Array(c));
        }
        if let Some(c) = downcast::<PersistentCollection>(object) {
            return Some(Self::Persistent(c));
        }
        downcast::<LazyCriteriaCollection>(object).map(Self::Lazy)
    }
}

fn not_a_collection(value: &Value) -> Thrown {
    Thrown::new(
        "Error",
        format!(
            "Call to a member function on {}",
            match value {
                Value::Object(o) => o.class().to_owned(),
                Value::Null => "null".to_owned(),
                Value::Array(_) => "array".to_owned(),
                _ => "a scalar".to_owned(),
            }
        ),
        0,
    )
}

/// A synchronous change of a lazy criteria collection: it would have to
/// load its elements first, which is `async`.
fn lazy_sync(method: &str) -> Thrown {
    Thrown::new(
        "LogicException",
        format!("LazyCriteriaCollection::{method}() loads the collection; initialize() it first"),
        0,
    )
}

fn of(value: &Value) -> Result<CollectionRef, Thrown> {
    CollectionRef::of(value).ok_or_else(|| not_a_collection(value))
}

/// `$collection->add($element)` / `$collection[] = $element`.
pub fn add(collection: &Value, element: Value) -> Result<(), Thrown> {
    match of(collection)? {
        CollectionRef::Array(c) => c.lock().add(element),
        CollectionRef::Persistent(c) => c.add(element),
        CollectionRef::Lazy(_) => Err(lazy_sync("add")),
    }
}

/// `$collection->toArray()`.
pub async fn to_array(collection: &Value) -> Result<PhpArray, Thrown> {
    match of(collection)? {
        CollectionRef::Array(c) => Ok(c.lock().to_array()),
        CollectionRef::Persistent(c) => c.to_array().await,
        CollectionRef::Lazy(c) => c.to_array().await,
    }
}

/// `count($collection)`.
pub async fn count(collection: &Value) -> Result<usize, Thrown> {
    match of(collection)? {
        CollectionRef::Array(c) => Ok(c.lock().count()),
        CollectionRef::Persistent(c) => c.count().await,
        CollectionRef::Lazy(c) => Ok(usize::try_from(c.count().await?).unwrap_or(0)),
    }
}

/// `$collection->get($key)` / `$collection[$key]`.
pub async fn get(collection: &Value, key: impl Into<Key>) -> Result<Value, Thrown> {
    let key = key.into();
    match of(collection)? {
        CollectionRef::Array(c) => Ok(c.lock().get(&key)),
        CollectionRef::Persistent(c) => c.get(&key).await,
        CollectionRef::Lazy(c) => c.get(&key).await,
    }
}

/// `$collection->set($key, $value)`.
pub async fn set(collection: &Value, key: impl Into<Key>, value: Value) -> Result<(), Thrown> {
    let key = key.into();
    match of(collection)? {
        CollectionRef::Array(c) => {
            c.lock().set(key, value);
            Ok(())
        }
        CollectionRef::Persistent(c) => c.set(key, value).await,
        CollectionRef::Lazy(c) => c.with_collection(|c| c.set(key, value)).await,
    }
}

/// `$collection->remove($key)` / `unset($collection[$key])`.
pub async fn remove(collection: &Value, key: impl Into<Key>) -> Result<Value, Thrown> {
    let key = key.into();
    match of(collection)? {
        CollectionRef::Array(c) => Ok(c.lock().remove(&key)),
        CollectionRef::Persistent(c) => c.remove(&key).await,
        CollectionRef::Lazy(c) => c.with_collection(|c| c.remove(&key)).await,
    }
}

/// `$collection->removeElement($element)`.
pub async fn remove_element(collection: &Value, element: &Value) -> Result<bool, Thrown> {
    match of(collection)? {
        CollectionRef::Array(c) => Ok(c.lock().remove_element(element)),
        CollectionRef::Persistent(c) => c.remove_element(element).await,
        CollectionRef::Lazy(c) => c.with_collection(|c| c.remove_element(element)).await,
    }
}

/// `$collection->contains($element)`.
pub async fn contains(collection: &Value, element: &Value) -> Result<bool, Thrown> {
    match of(collection)? {
        CollectionRef::Array(c) => Ok(c.lock().contains(element)),
        CollectionRef::Persistent(c) => c.contains(element).await,
        CollectionRef::Lazy(c) => c.contains(element).await,
    }
}

/// `isset($collection[$key])`.
pub async fn contains_key(collection: &Value, key: impl Into<Key>) -> Result<bool, Thrown> {
    let key = key.into();
    match of(collection)? {
        CollectionRef::Array(c) => Ok(c.lock().contains_key(&key)),
        CollectionRef::Persistent(c) => c.contains_key(&key).await,
        CollectionRef::Lazy(c) => c.contains_key(&key).await,
    }
}

/// `$collection->clear()`.
pub async fn clear(collection: &Value) -> Result<(), Thrown> {
    match of(collection)? {
        CollectionRef::Array(c) => {
            c.lock().clear();
            Ok(())
        }
        CollectionRef::Persistent(c) => c.clear().await,
        CollectionRef::Lazy(c) => c.with_collection(ArrayCollection::clear).await,
    }
}

/// `$collection->isEmpty()`.
pub async fn is_empty(collection: &Value) -> Result<bool, Thrown> {
    match of(collection)? {
        CollectionRef::Array(c) => Ok(c.lock().is_empty()),
        CollectionRef::Persistent(c) => c.is_empty().await,
        CollectionRef::Lazy(c) => c.is_empty().await,
    }
}

/// `$collection->first()` (`false` when empty).
pub async fn first(collection: &Value) -> Result<Value, Thrown> {
    match of(collection)? {
        CollectionRef::Array(c) => Ok(c.lock().first()),
        CollectionRef::Persistent(c) => c.first().await,
        CollectionRef::Lazy(c) => c.first().await,
    }
}

/// `$collection->getValues()`.
pub async fn get_values(collection: &Value) -> Result<Vec<Value>, Thrown> {
    Ok(to_array(collection).await?.values().cloned().collect())
}
