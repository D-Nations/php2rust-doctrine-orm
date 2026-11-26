//! A lazy collection that allows a fast count when using criteria object.
//! Once count gets executed once without collection being initialized,
//! result is cached and returned on subsequent calls until collection
//! gets initialized.
//!
//! The methods that load the elements are `async` here (PHP loads them on
//! the spot); `matching()` gives an `ArrayCollection` object.
//!
//! Upstream: doctrine/orm src/LazyCriteriaCollection.php

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use php_value::{Key, PhpArray, PhpObject, Thrown, Value};
use php2rust_doctrine_collections::{ArrayCollection, Criteria};

use crate::collection_object::ArrayCollectionObject;
use crate::persisters::EntityPersister;

/// `Doctrine\ORM\LazyCriteriaCollection`.
pub const CLASS: &str = "Doctrine\\ORM\\LazyCriteriaCollection";

#[derive(Debug, Default)]
struct State {
    collection: Option<ArrayCollection>,
    count: Option<i64>,
}

/// `Doctrine\ORM\LazyCriteriaCollection`.
#[derive(Debug)]
pub struct LazyCriteriaCollection {
    entity_persister: Arc<dyn EntityPersister>,
    criteria: Criteria,
    state: Mutex<State>,
}

impl LazyCriteriaCollection {
    pub fn new(entity_persister: Arc<dyn EntityPersister>, criteria: Criteria) -> Arc<Self> {
        Arc::new(Self {
            entity_persister,
            criteria,
            state: Mutex::new(State::default()),
        })
    }

    fn st(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The collection as a value.
    pub fn value(self: &Arc<Self>) -> Value {
        Value::Object(self.clone())
    }

    /// `isInitialized()`.
    pub fn is_initialized(&self) -> bool {
        self.st().collection.is_some()
    }

    /// `initialize()`.
    pub async fn initialize(&self) -> Result<(), Thrown> {
        if self.is_initialized() {
            return Ok(());
        }
        self.do_initialize().await
    }

    async fn do_initialize(&self) -> Result<(), Thrown> {
        let elements = self.entity_persister.load_criteria(&self.criteria).await?;
        self.st().collection = Some(ArrayCollection::new(PhpArray::list(elements)));
        Ok(())
    }

    /// Runs `f` on the initialized collection (the `AbstractLazyCollection`
    /// methods: `initialize()`, then the wrapped collection's method).
    pub async fn with_collection<R>(
        &self,
        f: impl FnOnce(&mut ArrayCollection) -> R,
    ) -> Result<R, Thrown> {
        self.initialize().await?;
        let mut st = self.st();
        let collection = st.collection.get_or_insert_with(ArrayCollection::default);
        Ok(f(collection))
    }

    /// Do an efficient count on the collection
    pub async fn count(&self) -> Result<i64, Thrown> {
        {
            let st = self.st();
            if let Some(collection) = &st.collection {
                return Ok(i64::try_from(collection.count()).unwrap_or(i64::MAX));
            }
            // Return cached result in case count query was already executed
            if let Some(count) = st.count {
                return Ok(count);
            }
        }
        let count = self.entity_persister.count_criteria(&self.criteria).await?;
        self.st().count = Some(count);
        Ok(count)
    }

    /// check if collection is empty without loading it
    pub async fn is_empty(&self) -> Result<bool, Thrown> {
        if let Some(collection) = &self.st().collection {
            return Ok(collection.is_empty());
        }
        Ok(self.count().await? == 0)
    }

    /// Do an optimized search of an element
    pub async fn contains(&self, element: &Value) -> Result<bool, Thrown> {
        if let Some(collection) = &self.st().collection {
            return Ok(collection.contains(element));
        }
        let Value::Object(entity) = element else {
            return Err(Thrown::new(
                "TypeError",
                "Doctrine\\ORM\\Persisters\\Entity\\BasicEntityPersister::exists(): Argument #1 ($entity) must be of type object",
                0,
            ));
        };
        self.entity_persister
            .exists(entity, Some(&self.criteria))
            .await
    }

    /// `matching($criteria)`: initializes the collection and matches its
    /// elements in memory.
    pub async fn matching(&self, criteria: &Criteria) -> Result<Value, Thrown> {
        let matched = self
            .with_collection(|collection| collection.matching(criteria))
            .await??;
        Ok(ArrayCollectionObject::value(matched.to_array()))
    }

    /// `toArray()`.
    pub async fn to_array(&self) -> Result<PhpArray, Thrown> {
        self.with_collection(|c| c.to_array()).await
    }

    /// `getValues()`.
    pub async fn get_values(&self) -> Result<PhpArray, Thrown> {
        self.with_collection(|c| c.get_values()).await
    }

    /// `get($key)`.
    pub async fn get(&self, key: &Key) -> Result<Value, Thrown> {
        self.with_collection(|c| c.get(key)).await
    }

    /// `containsKey($key)`.
    pub async fn contains_key(&self, key: &Key) -> Result<bool, Thrown> {
        self.with_collection(|c| c.contains_key(key)).await
    }

    /// `first()`.
    pub async fn first(&self) -> Result<Value, Thrown> {
        self.with_collection(ArrayCollection::first).await
    }

    /// `slice($offset, $length)`.
    pub async fn slice(&self, offset: i64, length: Option<i64>) -> Result<PhpArray, Thrown> {
        self.with_collection(|c| c.slice(offset, length)).await
    }
}

impl PhpObject for LazyCriteriaCollection {
    fn class(&self) -> &str {
        CLASS
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }

    fn instance_of(&self, class: &str) -> bool {
        [
            CLASS,
            "Doctrine\\Common\\Collections\\AbstractLazyCollection",
            "Doctrine\\Common\\Collections\\Collection",
            "Doctrine\\Common\\Collections\\ReadableCollection",
            "Doctrine\\Common\\Collections\\Selectable",
            "ArrayAccess",
            "Countable",
            "Traversable",
        ]
        .iter()
        .any(|known| known.eq_ignore_ascii_case(class))
    }

    fn php_count(&self) -> Option<Result<i64, Thrown>> {
        // `count()` may run a query; a synchronous caller sees the
        // initialized or the cached count only.
        let st = self.st();
        if let Some(collection) = &st.collection {
            return Some(Ok(i64::try_from(collection.count()).unwrap_or(i64::MAX)));
        }
        st.count.map(Ok)
    }

    fn traverse(&self) -> Option<Result<Vec<(Value, Value)>, Thrown>> {
        let st = self.st();
        st.collection.as_ref().map(|c| {
            Ok(c.to_array()
                .iter()
                .map(|(k, v)| (k.to_value(), v.clone()))
                .collect())
        })
    }
}
