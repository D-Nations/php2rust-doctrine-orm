//! A `PersistentCollection` represents a collection of elements that have
//! persistent state.
//!
//! Collections of entities represent only the associations (links) to
//! those entities. That means, if the collection is part of a many-many
//! mapping and you remove entities from the collection, only the links in
//! the relation table are removed (on flush). Similarly, if you remove
//! entities from a collection that is part of a one-many mapping this
//! will only result in the nulling out of the foreign keys on flush.
//!
//! The collection is a PHP object (an entity's property holds it); the
//! methods that make PHP initialize it (`count()`, `get()`, `toArray()`,
//! ...) are `async` here, as loading reads the database. `add()` does not
//! initialize the collection and stays synchronous. `EXTRA_LAZY`
//! associations load like `LAZY` ones (the extra-lazy paths of
//! `count()`, `contains()`, `get()` and `slice()` are not ported), and
//! `matching()` is not ported. The owner is held weakly (PHP holds it).
//!
//! Upstream: doctrine/orm src/PersistentCollection.php src/PersistentCollectionImplementation.php

use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use php_value::{Key, Object, PhpArray, PhpObject, Thrown, Value};
use php2rust_doctrine_collections::ArrayCollection;

use crate::entity_manager::EntityManager;
use crate::internal::object_id;
use crate::mapping::{AssociationMapping, ClassMetadata};

/// `Doctrine\ORM\PersistentCollection`.
pub const CLASS: &str = "Doctrine\\ORM\\PersistentCollection";

#[derive(Debug)]
struct State {
    collection: ArrayCollection,
    initialized: bool,
    /// A snapshot of the collection at the moment it was fetched from the
    /// database. This is used to create a diff of the collection at
    /// commit time.
    snapshot: PhpArray,
    /// The entity that owns this collection.
    owner: Option<Weak<dyn PhpObject>>,
    /// The association mapping the collection belongs to.
    association: Option<AssociationMapping>,
    /// The name of the field on the target entities that points to the
    /// owner of the collection. This is only set if the association is
    /// bi-directional.
    back_ref_field_name: Option<String>,
    /// Whether the collection is dirty and needs to be synchronized with
    /// the database when the `UnitOfWork` that manages its persistent state
    /// commits.
    is_dirty: bool,
}

/// `Doctrine\ORM\PersistentCollection`.
pub struct PersistentCollection {
    em: Mutex<Option<Weak<EntityManager>>>,
    type_class: Option<Arc<ClassMetadata>>,
    state: Mutex<State>,
}

impl std::fmt::Debug for PersistentCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistentCollection")
            .finish_non_exhaustive()
    }
}

fn no_em() -> Thrown {
    Thrown::new(
        "Error",
        "Call to a member function getUnitOfWork() on null",
        0,
    )
}

impl PersistentCollection {
    /// Creates a new persistent collection.
    pub fn new(
        em: Option<&Arc<EntityManager>>,
        type_class: Option<Arc<ClassMetadata>>,
        collection: ArrayCollection,
    ) -> Arc<Self> {
        Arc::new(Self {
            em: Mutex::new(em.map(Arc::downgrade)),
            type_class,
            state: Mutex::new(State {
                collection,
                initialized: true,
                snapshot: PhpArray::new(),
                owner: None,
                association: None,
                back_ref_field_name: None,
                is_dirty: false,
            }),
        })
    }

    fn st(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn em(&self) -> Option<Arc<EntityManager>> {
        self.em
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .and_then(Weak::upgrade)
    }

    fn entity_manager(&self) -> Result<Arc<EntityManager>, Thrown> {
        self.em().ok_or_else(no_em)
    }

    /// The collection as a property value.
    pub fn value(self: &Arc<Self>) -> Value {
        Value::Object(self.clone())
    }

    /// The collection behind a property value, when it is one.
    pub fn from_value(value: &Value) -> Option<Arc<Self>> {
        match value {
            Value::Object(object) => crate::collection_object::downcast(object),
            _ => None,
        }
    }

    /// `spl_object_id()` of the collection.
    pub fn oid(self: &Arc<Self>) -> usize {
        let object: Object = self.clone();
        object_id(&object)
    }

    /// Sets the collection's owning entity together with the
    /// `AssociationMapping` that describes the association between the owner
    /// and the elements of the collection.
    pub fn set_owner(&self, entity: &Object, assoc: &AssociationMapping) {
        let mut st = self.st();
        st.owner = Some(Arc::downgrade(entity));
        st.association = Some(assoc.clone());
        st.back_ref_field_name = if assoc.is_owning_side() {
            assoc.inversed_by.clone()
        } else {
            assoc.mapped_by.clone()
        };
    }

    /// Gets the collection owner.
    pub fn get_owner(&self) -> Option<Object> {
        self.st().owner.as_ref().and_then(Weak::upgrade)
    }

    pub fn get_type_class(&self) -> Option<&Arc<ClassMetadata>> {
        self.type_class.as_ref()
    }

    /// INTERNAL: Adds an element to a collection during hydration. This
    /// will automatically complete bidirectional associations in the case
    /// of a one-to-many association.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "the collection takes the element, as PHP's method does"
    )]
    pub fn hydrate_add(&self, element: Value) -> Result<(), Thrown> {
        let (back_ref, is_one_to_many, owner) = {
            let mut st = self.st();
            st.collection.add(element.clone())?;
            (
                st.back_ref_field_name.clone(),
                st.association
                    .as_ref()
                    .is_some_and(AssociationMapping::is_one_to_many),
                st.owner.as_ref().and_then(Weak::upgrade),
            )
        };
        if let (Some(back_ref), true, Value::Object(element)) = (back_ref, is_one_to_many, &element)
            && !back_ref.is_empty()
        {
            let owner_value = owner.map_or(Value::Null, Value::Object);
            if let Some(type_class) = &self.type_class {
                type_class
                    .read()
                    .set_field_value(element, &back_ref, owner_value.clone())?;
            }
            self.entity_manager()?
                .get_unit_of_work()
                .set_original_entity_property(object_id(element), &back_ref, owner_value);
        }
        Ok(())
    }

    /// INTERNAL: Sets a keyed element in the collection during hydration.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "the collection takes the element, as PHP's method does"
    )]
    pub fn hydrate_set(&self, key: Key, element: Value) -> Result<(), Thrown> {
        let (back_ref, is_one_to_many, owner) = {
            let mut st = self.st();
            st.collection.set(key, element.clone());
            (
                st.back_ref_field_name.clone(),
                st.association
                    .as_ref()
                    .is_some_and(AssociationMapping::is_one_to_many),
                st.owner.as_ref().and_then(Weak::upgrade),
            )
        };
        if let (Some(back_ref), true, Value::Object(element)) = (back_ref, is_one_to_many, &element)
            && !back_ref.is_empty()
            && let Some(type_class) = &self.type_class
        {
            type_class.read().set_field_value(
                element,
                &back_ref,
                owner.map_or(Value::Null, Value::Object),
            )?;
        }
        Ok(())
    }

    /// Initializes the collection by loading its contents from the
    /// database if the collection is not yet initialized.
    pub async fn initialize(self: &Arc<Self>) -> Result<(), Thrown> {
        {
            let st = self.st();
            if st.initialized || st.association.is_none() {
                return Ok(());
            }
        }
        self.do_initialize().await?;
        self.st().initialized = true;
        Ok(())
    }

    /// INTERNAL: Tells this collection to take a snapshot of its current
    /// state.
    pub fn take_snapshot(&self) {
        let mut st = self.st();
        st.snapshot = st.collection.to_array();
        st.is_dirty = false;
    }

    /// INTERNAL: Returns the last snapshot of the elements in the
    /// collection.
    pub fn get_snapshot(&self) -> PhpArray {
        self.st().snapshot.clone()
    }

    fn by_oid(array: &PhpArray) -> Vec<(Option<usize>, Value)> {
        array
            .values()
            .map(|v| match v {
                Value::Object(o) => (Some(object_id(o)), v.clone()),
                _ => (None, v.clone()),
            })
            .collect()
    }

    fn diff(left: &PhpArray, right: &PhpArray) -> Vec<Value> {
        // array_diff_key() over array_combine(array_map('spl_object_id', ...)):
        // later duplicates replace earlier ones under the same key.
        let right: Vec<Option<usize>> = Self::by_oid(right).into_iter().map(|(k, _)| k).collect();
        let mut seen: Vec<usize> = Vec::new();
        let mut out: Vec<(usize, Value)> = Vec::new();
        for (oid, value) in Self::by_oid(left) {
            let Some(oid) = oid else { continue };
            if right.contains(&Some(oid)) {
                continue;
            }
            if let Some(at) = seen.iter().position(|k| *k == oid) {
                out[at].1 = value;
                continue;
            }
            seen.push(oid);
            out.push((oid, value));
        }
        out.into_iter().map(|(_, v)| v).collect()
    }

    /// INTERNAL: getDeleteDiff: the elements of the snapshot that are no
    /// longer in the collection.
    pub fn get_delete_diff(&self) -> Vec<Value> {
        let st = self.st();
        Self::diff(&st.snapshot, &st.collection.to_array())
    }

    /// INTERNAL: getInsertDiff: the elements of the collection that are
    /// not in the snapshot.
    pub fn get_insert_diff(&self) -> Vec<Value> {
        let st = self.st();
        Self::diff(&st.collection.to_array(), &st.snapshot)
    }

    /// INTERNAL: Gets the association mapping of the collection.
    pub fn get_mapping(&self) -> Result<AssociationMapping, Thrown> {
        self.st().association.clone().ok_or_else(|| {
            Thrown::new(
                "UnexpectedValueException",
                "The underlying association mapping is null although it should not be",
                0,
            )
        })
    }

    /// Marks this collection as changed/dirty.
    fn changed(&self) {
        let mut st = self.st();
        if st.is_dirty {
            return;
        }
        st.is_dirty = true;
    }

    /// Gets a boolean flag indicating whether this collection is dirty
    /// which means its state needs to be synchronized with the database.
    pub fn is_dirty(&self) -> bool {
        self.st().is_dirty
    }

    /// Sets a boolean flag, indicating whether this collection is dirty.
    pub fn set_dirty(&self, dirty: bool) {
        self.st().is_dirty = dirty;
    }

    /// Sets the initialized flag of the collection, forcing it into that
    /// state.
    pub fn set_initialized(&self, initialized: bool) {
        self.st().initialized = initialized;
    }

    /// Is the lazy collection already initialized?
    pub fn is_initialized(&self) -> bool {
        self.st().initialized
    }

    /// Retrieves the wrapped Collection instance (a copy of its elements).
    pub fn unwrap(&self) -> PhpArray {
        self.st().collection.to_array()
    }

    /// The wrapped collection, for a change that does not go through the
    /// persistent collection (PHP's `$coll->unwrap()->...`).
    pub fn with_unwrapped<R>(&self, f: impl FnOnce(&mut ArrayCollection) -> R) -> R {
        f(&mut self.st().collection)
    }

    fn orphan_removal_applies(&self) -> bool {
        let st = self.st();
        st.association
            .as_ref()
            .is_some_and(|a| a.is_to_many() && a.orphan_removal)
            && st.owner.as_ref().and_then(Weak::upgrade).is_some()
    }

    /// `remove($key)`.
    pub async fn remove(self: &Arc<Self>, key: &Key) -> Result<Value, Thrown> {
        self.initialize().await?;
        let removed = self.st().collection.remove(key);
        if !removed.is_truthy() {
            return Ok(removed);
        }
        self.changed();
        if self.orphan_removal_applies()
            && let Value::Object(entity) = &removed
        {
            self.entity_manager()?
                .get_unit_of_work()
                .schedule_orphan_removal(entity);
        }
        Ok(removed)
    }

    /// `removeElement($element)`.
    pub async fn remove_element(self: &Arc<Self>, element: &Value) -> Result<bool, Thrown> {
        self.initialize().await?;
        let removed = self.st().collection.remove_element(element);
        if !removed {
            return Ok(removed);
        }
        self.changed();
        if self.orphan_removal_applies()
            && let Value::Object(entity) = element
        {
            self.entity_manager()?
                .get_unit_of_work()
                .schedule_orphan_removal(entity);
        }
        Ok(removed)
    }

    /// `containsKey($key)`.
    pub async fn contains_key(self: &Arc<Self>, key: &Key) -> Result<bool, Thrown> {
        self.initialize().await?;
        Ok(self.st().collection.contains_key(key))
    }

    /// `contains($element)`.
    pub async fn contains(self: &Arc<Self>, element: &Value) -> Result<bool, Thrown> {
        self.initialize().await?;
        Ok(self.st().collection.contains(element))
    }

    /// `get($key)`.
    pub async fn get(self: &Arc<Self>, key: &Key) -> Result<Value, Thrown> {
        self.initialize().await?;
        Ok(self.st().collection.get(key))
    }

    /// `count()`.
    pub async fn count(self: &Arc<Self>) -> Result<usize, Thrown> {
        self.initialize().await?;
        Ok(self.st().collection.count())
    }

    /// `set($key, $value)`.
    pub async fn set(self: &Arc<Self>, key: Key, value: Value) -> Result<(), Thrown> {
        self.initialize().await?;
        self.st().collection.set(key, value.clone());
        self.changed();
        if let (Value::Object(entity), Some(em)) = (&value, self.em()) {
            em.get_unit_of_work().cancel_orphan_removal(entity);
        }
        Ok(())
    }

    /// `add($value)`: does not initialize the collection.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "the collection takes the element, as PHP's method does"
    )]
    pub fn add(&self, value: Value) -> Result<(), Thrown> {
        self.st().collection.add(value.clone())?;
        self.changed();
        if let (Value::Object(entity), Some(em)) = (&value, self.em()) {
            em.get_unit_of_work().cancel_orphan_removal(entity);
        }
        Ok(())
    }

    /// `isEmpty()`.
    pub async fn is_empty(self: &Arc<Self>) -> Result<bool, Thrown> {
        let empty = self.st().collection.is_empty();
        Ok(empty && self.count().await? == 0)
    }

    /// `clear()`.
    pub async fn clear(self: &Arc<Self>) -> Result<(), Thrown> {
        let initialized = self.is_initialized();
        if initialized && self.is_empty().await? {
            self.st().collection.clear();
            return Ok(());
        }
        let em = self.entity_manager()?;
        let uow = em.get_unit_of_work();
        let association = self.get_mapping()?;
        let has_owner = self.get_owner().is_some();
        if association.is_to_many() && association.orphan_removal && has_owner {
            self.initialize().await?;
            for element in self.unwrap().values() {
                if let Value::Object(entity) = element {
                    uow.schedule_orphan_removal(entity);
                }
            }
        }
        {
            let mut st = self.st();
            st.collection.clear();
            st.initialized = true; // direct call, {@link initialize()} is too expensive
        }
        if association.is_owning_side() && has_owner {
            self.changed();
            uow.schedule_collection_deletion(self);
            self.take_snapshot();
        }
        Ok(())
    }

    /// `first()`.
    pub async fn first(self: &Arc<Self>) -> Result<Value, Thrown> {
        self.initialize().await?;
        Ok(self.st().collection.first())
    }

    /// `last()`.
    pub async fn last(self: &Arc<Self>) -> Result<Value, Thrown> {
        self.initialize().await?;
        Ok(self.st().collection.last())
    }

    /// `key()`: the key at the collection's internal pointer.
    pub fn key(&self) -> Option<Key> {
        self.st().collection.key()
    }

    /// `slice($offset, $length)`.
    pub async fn slice(
        self: &Arc<Self>,
        offset: i64,
        length: Option<i64>,
    ) -> Result<PhpArray, Thrown> {
        self.initialize().await?;
        Ok(self.st().collection.slice(offset, length))
    }

    /// `toArray()`.
    pub async fn to_array(self: &Arc<Self>) -> Result<PhpArray, Thrown> {
        self.initialize().await?;
        Ok(self.st().collection.to_array())
    }

    /// `getValues()`.
    pub async fn get_values(self: &Arc<Self>) -> Result<PhpArray, Thrown> {
        self.initialize().await?;
        Ok(self.st().collection.get_values())
    }

    /// `getKeys()`.
    pub async fn get_keys(self: &Arc<Self>) -> Result<PhpArray, Thrown> {
        self.initialize().await?;
        Ok(self.st().collection.get_keys())
    }

    /// `clone $collection` (`__clone()`): a new initialized collection with
    /// the elements, without an owner or snapshot, marked dirty.
    pub async fn clone_collection(self: &Arc<Self>) -> Result<Arc<Self>, Thrown> {
        let em = self.em();
        let elements = self.unwrap();
        let initialized = self.is_initialized();
        let association = self.st().association.clone();
        let copy = Self::new(
            em.as_ref(),
            self.type_class.clone(),
            ArrayCollection::new(elements),
        );
        {
            let mut st = copy.st();
            st.initialized = initialized;
            st.association = association;
        }
        copy.initialize().await?;
        {
            let mut st = copy.st();
            st.owner = None;
            st.snapshot = PhpArray::new();
        }
        copy.changed();
        Ok(copy)
    }

    /// `__wakeup()`: an unserialized collection has no entity manager.
    pub fn wakeup(&self) {
        *self.em.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    async fn do_initialize(self: &Arc<Self>) -> Result<(), Thrown> {
        // Has NEW objects added through add(). Remember them.
        let newly_added_dirty_objects = if self.is_dirty() {
            self.unwrap()
        } else {
            PhpArray::new()
        };
        self.st().collection.clear();
        self.entity_manager()?
            .get_unit_of_work()
            .load_collection(self)
            .await?;
        self.take_snapshot();
        if !newly_added_dirty_objects.is_empty() {
            self.restore_new_objects_in_dirty_collection(&newly_added_dirty_objects)?;
        }
        Ok(())
    }

    /// The collection was dirty: the objects added before the
    /// initialization that the database did not give back are added again.
    fn restore_new_objects_in_dirty_collection(
        &self,
        new_objects: &PhpArray,
    ) -> Result<(), Thrown> {
        let loaded = self.unwrap();
        let loaded_oids: Vec<usize> = loaded
            .values()
            .filter_map(|v| match v {
                Value::Object(o) => Some(object_id(o)),
                _ => None,
            })
            .collect();
        let not_loaded = Self::diff(new_objects, &loaded);
        let _ = loaded_oids;
        if !not_loaded.is_empty() {
            let mut st = self.st();
            for object in not_loaded {
                st.collection.add(object)?;
            }
            st.is_dirty = true;
        }
        Ok(())
    }
}

impl PhpObject for PersistentCollection {
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
}
