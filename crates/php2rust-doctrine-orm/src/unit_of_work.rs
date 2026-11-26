//! The `UnitOfWork` is responsible for tracking changes to objects during
//! an "object-level" transaction and for writing out changes to the
//! database in the correct order.
//!
//! Internal note: this class contains highly performance-sensitive code.
//!
//! The methods that read or write the database (`commit()`, `remove()`,
//! `refresh()`, `createEntity()`, `loadCollection()`, ...) are `async`;
//! `persist()`, `detach()` and the bookkeeping methods are not. Objects
//! are told apart by their address (`spl_object_id()`), and the unit of
//! work holds the objects it tracks. The second level cache, filters,
//! the `NOTIFY` change tracking policy's listener and partial objects are
//! not ported.
//!
//! Upstream: doctrine/orm src/UnitOfWork.php src/Internal/HydrationCompleteHandler.php src/Internal/UnitOfWork/InsertBatch.php
//! Upstream: doctrine/orm src/Id/AssignedGenerator.php src/Id/IdentityGenerator.php src/Id/SequenceGenerator.php

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use php_value::{Handle, Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_collections::ArrayCollection;
use php2rust_doctrine_dbal::LockMode;
use php2rust_doctrine_event_manager::EventDispatcher;

use crate::collection_object::{ArrayCollectionObject, downcast};
use crate::entity_manager::EntityManager;
use crate::event::{
    self, LifecycleEventArgs, ListenersInvoker, ManagerEventArgs, PreUpdateEventArgs,
};
use crate::events;
use crate::exception;
use crate::id::IdGenerator;
use crate::internal::{StronglyConnectedComponents, TopologicalSort, object_id};
use crate::mapping::{AssociationMapping, ClassMetadata};
use crate::object::EntityObject;
use crate::persistent_collection::PersistentCollection;
use crate::persisters::BasicEntityPersister;
use crate::utility::{flatten_identifier, to_text};

/// A future of the unit of work.
/// A boxed future of a `Result`.
pub type Fut<'a, T> = Pin<Box<dyn Future<Output = Result<T, Thrown>> + Send + 'a>>;

/// An entity is in MANAGED state when its persistence is managed by an
/// `EntityManager`.
pub const STATE_MANAGED: i32 = 1;
/// An entity is new if it has just been instantiated (i.e. using the
/// "new" operator) and is not (yet) managed by an `EntityManager`.
pub const STATE_NEW: i32 = 2;
/// A detached entity is an instance with persistent state and identity
/// that is not (or no longer) associated with an `EntityManager` (and a
/// `UnitOfWork`).
pub const STATE_DETACHED: i32 = 3;
/// A removed entity instance is an instance with a persistent identity,
/// associated with an `EntityManager`, whose persistent state will be
/// deleted on commit.
pub const STATE_REMOVED: i32 = 4;

/// Hint used to collect all primary keys of associated entities during
/// hydration and execute it in a dedicated query afterwards.
pub const HINT_DEFEREAGERLOAD: &str = "deferEagerLoad";

/// `ClassMetadata::FETCH_EAGER`.
const FETCH_EAGER: i64 = 3;

/// The hints of a hydration (`Query::HINT_REFRESH`, ...).
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools, reason = "the query hints")]
pub struct Hints {
    /// `Query::HINT_REFRESH`.
    pub refresh: bool,
    /// `Query::HINT_REFRESH_ENTITY`.
    pub refresh_entity: Option<Object>,
    /// `Query::HINT_READ_ONLY`.
    pub read_only: bool,
    /// `UnitOfWork::HINT_DEFEREAGERLOAD`.
    pub defer_eager_load: bool,
    /// `fetchAlias`.
    pub fetch_alias: Option<String>,
    /// `fetched[alias][field]`.
    pub fetched: Vec<(String, String)>,
    /// `fetchMode[class][field]`.
    pub fetch_mode: Vec<(String, String, Option<i64>)>,
    /// `LimitSubqueryWalker::FORCE_DBAL_TYPE_CONVERSION`.
    pub force_dbal_type_conversion: bool,
    /// `Query::HINT_INTERNAL_ITERATION`.
    pub internal_iteration: bool,
}

impl Hints {
    fn is_fetched(&self, field: &str) -> bool {
        self.fetch_alias
            .as_ref()
            .is_some_and(|alias| self.fetched.iter().any(|(a, f)| a == alias && f == field))
    }

    fn fetch_mode(&mut self, class: &str, field: &str, default: Option<i64>) -> Option<i64> {
        if let Some((_, _, mode)) = self
            .fetch_mode
            .iter()
            .find(|(c, f, _)| c == class && f == field)
        {
            return *mode;
        }
        self.fetch_mode
            .push((class.to_owned(), field.to_owned(), default));
        default
    }
}

/// An insertion-ordered map (PHP's array keyed by object id or name).
#[derive(Debug, Clone)]
pub(crate) struct Ordered<K, V> {
    slots: Vec<Option<(K, V)>>,
    index: HashMap<K, usize>,
}

impl<K, V> Default for Ordered<K, V> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            index: HashMap::new(),
        }
    }
}

impl<K: Clone + Eq + Hash, V> Ordered<K, V> {
    pub(crate) fn get(&self, key: &K) -> Option<&V> {
        let at = *self.index.get(key)?;
        self.slots[at].as_ref().map(|(_, v)| v)
    }

    pub(crate) fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let at = *self.index.get(key)?;
        self.slots[at].as_mut().map(|(_, v)| v)
    }

    pub(crate) fn contains(&self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    /// `$map[$key] = $value`: a known key keeps its position.
    pub(crate) fn insert(&mut self, key: K, value: V) {
        if let Some(&at) = self.index.get(&key) {
            self.slots[at] = Some((key, value));
            return;
        }
        self.index.insert(key.clone(), self.slots.len());
        self.slots.push(Some((key, value)));
    }

    pub(crate) fn remove(&mut self, key: &K) -> Option<V> {
        let at = self.index.remove(key)?;
        let removed = self.slots[at].take().map(|(_, v)| v);
        if self.index.is_empty() {
            self.slots.clear();
        }
        removed
    }

    pub(crate) fn len(&self) -> usize {
        self.index.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.slots
            .iter()
            .filter_map(|slot| slot.as_ref().map(|(k, v)| (k, v)))
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, v)| v)
    }

    #[allow(
        clippy::needless_pass_by_value,
        reason = "the key is stored when it is missing"
    )]
    fn get_or_default(&mut self, key: K) -> &mut V
    where
        V: Default,
    {
        if !self.contains(&key) {
            self.insert(key.clone(), V::default());
        }
        let at = self.index[&key];
        match &mut self.slots[at] {
            Some((_, v)) => v,
            None => unreachable!("an indexed slot is filled"),
        }
    }
}

type ObjectMap = Ordered<usize, Object>;
type CollectionMap = Ordered<usize, Arc<PersistentCollection>>;

#[derive(Debug, Default)]
struct State {
    /// The identity map that holds references to all managed entities that
    /// have an identity. The entities are grouped by their class name.
    identity_map: Ordered<String, Ordered<String, Object>>,
    /// Map of all identifiers of managed entities.
    entity_identifiers: HashMap<usize, PhpArray>,
    /// Map of the original entity data of managed entities.
    original_entity_data: HashMap<usize, PhpArray>,
    /// Map of entity changes.
    entity_change_sets: HashMap<usize, PhpArray>,
    /// The (cached) states of any known entities.
    entity_states: HashMap<usize, i32>,
    /// Map of entities that are scheduled for dirty checking at
    /// commit time (deferred explicit change tracking).
    scheduled_for_synchronization: Ordered<String, ObjectMap>,
    /// A list of all pending entity insertions.
    entity_insertions: ObjectMap,
    /// A list of all pending entity updates.
    entity_updates: ObjectMap,
    /// Any pending extra updates that have been scheduled by persisters.
    extra_updates: Ordered<usize, (Object, PhpArray)>,
    /// A list of all pending entity deletions.
    entity_deletions: ObjectMap,
    /// New entities that were discovered through relationships that were
    /// not marked as cascade-persist.
    non_cascaded_new_detected_entities: Ordered<usize, (AssociationMapping, Object)>,
    /// All pending collection deletions.
    collection_deletions: CollectionMap,
    /// All pending collection updates.
    collection_updates: CollectionMap,
    /// List of collections visited during changeset calculation on a
    /// commit-phase of a `UnitOfWork`.
    visited_collections: CollectionMap,
    /// List of collections visited during the changeset calculation that
    /// contain to-be-removed entities and need to have keys removed
    /// post commit.
    pending_collection_element_removals: HashMap<usize, Vec<Key>>,
    /// Orphaned entities that are scheduled for removal.
    orphan_removals: ObjectMap,
    /// Read-Only objects are never evaluated.
    read_only_objects: HashSet<usize>,
    /// Map of entities to load eagerly (root class => id hash => id).
    eager_loading_entities: Ordered<String, Ordered<String, Value>>,
    /// Collections to load eagerly (`sourceEntity#fieldName`).
    eager_loading_collections: Ordered<
        String,
        (
            AssociationMapping,
            Ordered<String, Arc<PersistentCollection>>,
        ),
    >,
    /// The deferred postLoad invocations (`HydrationCompleteHandler`).
    deferred_post_load_invocations: Vec<(Arc<ClassMetadata>, i32, Object)>,
}

/// `Doctrine\ORM\UnitOfWork`.
pub struct UnitOfWork {
    em: Weak<EntityManager>,
    state: Mutex<State>,
    persisters: Mutex<HashMap<String, Arc<BasicEntityPersister>>>,
    listeners_invoker: ListenersInvoker,
}

impl std::fmt::Debug for UnitOfWork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnitOfWork").finish_non_exhaustive()
    }
}

/// `get_class($a) === get_class($b)` for objects: the same object.
fn same(a: &Object, b: &Object) -> bool {
    object_id(a) == object_id(b)
}

fn unexpected_state(state: i32, entity: &Object) -> Thrown {
    Thrown::new(
        "UnexpectedValueException",
        format!(
            "Unexpected entity state: {state}. {}",
            exception::obj_to_str(entity)
        ),
        0,
    )
}

/// The entities a relation property holds (a collection's elements, or
/// the related entity).
fn related_entities(value: &Value) -> Option<Vec<Value>> {
    if let Some(pc) = PersistentCollection::from_value(value) {
        return Some(pc.unwrap().values().cloned().collect());
    }
    if let Some(ac) = ArrayCollectionObject::from_value(value) {
        return Some(ac.lock().to_array().values().cloned().collect());
    }
    if let Value::Array(array) = value {
        return Some(array.values().cloned().collect());
    }
    None
}

impl UnitOfWork {
    /// Initializes a new `UnitOfWork` instance, bound to the given
    /// `EntityManager`.
    pub fn new(em: Weak<EntityManager>) -> Self {
        Self {
            em,
            state: Mutex::new(State::default()),
            persisters: Mutex::new(HashMap::new()),
            listeners_invoker: ListenersInvoker::new(),
        }
    }

    fn st(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn em(&self) -> Result<Arc<EntityManager>, Thrown> {
        self.em
            .upgrade()
            .ok_or_else(|| Thrown::new("Error", "The entity manager is gone", 0))
    }

    /// The listeners invoker.
    pub fn listeners_invoker(&self) -> &ListenersInvoker {
        &self.listeners_invoker
    }

    #[allow(clippy::unused_self, reason = "an instance method upstream")]
    fn subscribed(&self, em: &EntityManager, class: &ClassMetadata, event: &str) -> i32 {
        ListenersInvoker::get_subscribed_systems(&class.read(), &em.get_event_manager(), event)
    }

    fn invoke_lifecycle(
        &self,
        em: &Arc<EntityManager>,
        class: &ClassMetadata,
        event: &str,
        args_class: &'static str,
        entity: &Object,
        invoke: i32,
    ) -> Result<(), Thrown> {
        let args = LifecycleEventArgs::new(args_class, entity.clone(), em.clone());
        let info = class.read();
        self.listeners_invoker
            .invoke(&info, &em.get_event_manager(), event, entity, &args, invoke)
    }

    #[allow(clippy::unused_self, reason = "an instance method upstream")]
    fn dispatch(
        &self,
        em: &Arc<EntityManager>,
        name: &str,
        args_class: &'static str,
    ) -> Result<(), Thrown> {
        let args = ManagerEventArgs::new(args_class, em.clone());
        em.get_event_manager()
            .dispatch_event(name, Some(&args))
            .map_err(event::listener_error)
    }

    /// Commits the `UnitOfWork`, executing all operations that have been
    /// postponed up to this point. The state of all managed entities will
    /// be synchronized with the database.
    ///
    /// The operations are executed in the following order:
    ///
    /// 1) All entity insertions
    /// 2) All entity updates
    /// 3) All collection deletions
    /// 4) All collection updates
    /// 5) All entity deletions
    pub async fn commit(&self) -> Result<(), Thrown> {
        let em = self.em()?;
        let connection = em.get_connection().clone();

        // Raise preFlush
        self.dispatch(&em, events::PRE_FLUSH, event::PRE_FLUSH_EVENT_ARGS)?;

        // Compute changes done since last commit.
        self.compute_change_sets().await?;

        let nothing = {
            let st = self.st();
            st.entity_insertions.is_empty()
                && st.entity_deletions.is_empty()
                && st.entity_updates.is_empty()
                && st.collection_updates.is_empty()
                && st.collection_deletions.is_empty()
                && st.orphan_removals.is_empty()
        };
        if nothing {
            self.dispatch(&em, events::ON_FLUSH, event::ON_FLUSH_EVENT_ARGS)?;
            self.dispatch(&em, events::POST_FLUSH, event::POST_FLUSH_EVENT_ARGS)?;
            self.post_commit_cleanup();
            return Ok(()); // Nothing to do.
        }

        self.assert_that_there_are_no_unintentionally_non_persisted_associations()?;

        let orphans: Vec<Object> = self.st().orphan_removals.values().cloned().collect();
        for orphan in orphans {
            self.remove(&orphan).await?;
        }

        self.dispatch(&em, events::ON_FLUSH, event::ON_FLUSH_EVENT_ARGS)?;

        connection
            .begin_transaction()
            .await
            .map_err(|e| exception::from_dbal(&e))?;

        let outcome = self.commit_in_transaction(&em).await;
        if let Err(e) = outcome {
            em.close();
            if connection.is_transaction_active() {
                // A failing rollback replaces the error (PHP's `finally`).
                connection
                    .roll_back()
                    .await
                    .map_err(|e| exception::from_dbal(&e))?;
            }
            return Err(e);
        }

        // Take new snapshots from visited collections
        let visited: Vec<(usize, Arc<PersistentCollection>)> = self
            .st()
            .visited_collections
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        for (coid, coll) in visited {
            let keys = self
                .st()
                .pending_collection_element_removals
                .get(&coid)
                .cloned();
            if let Some(keys) = keys {
                for key in keys {
                    coll.remove(&key).await?;
                }
            }
            coll.take_snapshot();
        }

        self.dispatch(&em, events::POST_FLUSH, event::POST_FLUSH_EVENT_ARGS)?;
        self.post_commit_cleanup();
        Ok(())
    }

    async fn commit_in_transaction(&self, em: &Arc<EntityManager>) -> Result<(), Thrown> {
        // Collection deletions (deletions of complete collections)
        let deletions: Vec<Arc<PersistentCollection>> =
            self.st().collection_deletions.values().cloned().collect();
        for collection_to_delete in deletions {
            let Some(owner) = collection_to_delete.get_owner() else {
                continue;
            };
            let implicit = em
                .get_class_metadata(owner.class())?
                .read()
                .is_change_tracking_deferred_implicit();
            if implicit || self.is_scheduled_for_dirty_check(&owner)? {
                crate::persisters::collection::delete(em, &collection_to_delete).await?;
            }
        }

        if !self.st().entity_insertions.is_empty() {
            // Perform entity insertions first, so that all new entities
            // have their rows in the database and can be referred to by
            // foreign keys. The commit order only needs to take new
            // entities into account (new entities referring to other new
            // entities), since all other types (entities with updates or
            // scheduled deletions) are currently not a problem, since they
            // are already in the database.
            self.execute_inserts().await?;
        }

        if !self.st().entity_updates.is_empty() {
            // Updates do not need to follow a particular order
            self.execute_updates().await?;
        }

        // Extra updates that were requested by persisters. This may
        // include foreign keys that could not be set when an entity was
        // inserted, which may happen in the case of circular foreign key
        // relationships.
        if !self.st().extra_updates.is_empty() {
            self.execute_extra_updates().await?;
        }

        // Collection updates (deleteRows, updateRows, insertRows) No
        // particular order is necessary, since all entities themselves are
        // already in the database
        let updates: Vec<Arc<PersistentCollection>> =
            self.st().collection_updates.values().cloned().collect();
        for collection_to_update in updates {
            crate::persisters::collection::update(em, &collection_to_update).await?;
        }

        // Entity deletions come last. Their order only needs to take care
        // of other deletions (first delete entities depending upon others,
        // before deleting depended-upon entities).
        if !self.st().entity_deletions.is_empty() {
            self.execute_deletions().await?;
        }

        // Commit failed silently
        if em.get_connection().commit().await.is_err() {
            return Err(exception::optimistic_lock_commit_failed());
        }
        Ok(())
    }

    fn post_commit_cleanup(&self) {
        let mut st = self.st();
        st.entity_insertions = Ordered::default();
        st.entity_updates = Ordered::default();
        st.entity_deletions = Ordered::default();
        st.extra_updates = Ordered::default();
        st.collection_updates = Ordered::default();
        st.non_cascaded_new_detected_entities = Ordered::default();
        st.collection_deletions = Ordered::default();
        st.pending_collection_element_removals = HashMap::new();
        st.visited_collections = Ordered::default();
        st.orphan_removals = Ordered::default();
        st.entity_change_sets = HashMap::new();
        st.scheduled_for_synchronization = Ordered::default();
    }

    /// Computes the changesets of all entities scheduled for insertion.
    async fn compute_schedule_inserts_change_sets(&self) -> Result<(), Thrown> {
        let em = self.em()?;
        let insertions: Vec<Object> = self.st().entity_insertions.values().cloned().collect();
        for entity in insertions {
            let class = em.get_class_metadata(entity.class())?;
            if self.is_uninitialized_object(&Value::Object(entity.clone())) {
                self.initialize_object(&entity).await?;
            }
            self.compute_change_set(&class, &entity).await?;
        }
        Ok(())
    }

    /// Executes any extra updates that have been scheduled.
    async fn execute_extra_updates(&self) -> Result<(), Thrown> {
        let extra: Vec<(usize, (Object, PhpArray))> = self
            .st()
            .extra_updates
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        for (oid, (entity, changeset)) in extra {
            self.st().entity_change_sets.insert(oid, changeset);
            self.get_entity_persister(entity.class())?
                .update(&entity)
                .await?;
        }
        self.st().extra_updates = Ordered::default();
        Ok(())
    }

    /// Gets the changeset for an entity.
    pub fn get_entity_change_set(&self, entity: &Object) -> PhpArray {
        self.st()
            .entity_change_sets
            .get(&object_id(entity))
            .cloned()
            .unwrap_or_default()
    }

    /// Computes the changes that happened to a single entity.
    ///
    /// Modifies/populates the following properties:
    ///
    /// `originalEntityData`: If the entity is NEW or MANAGED but not yet
    /// fully persisted (only has an id) then it was not fetched from the
    /// database and therefore we have no original entity data yet. All of
    /// the current entity data is stored as the original entity data.
    ///
    /// `entityChangeSets`: The changes detected on all properties of the
    /// entity are stored there. A change is a tuple array where the first
    /// entry is the old value and the second entry is the new value of the
    /// property. Changesets are used by persisters to INSERT/UPDATE the
    /// persistent entity state.
    ///
    /// `entityUpdates`: If the entity is already fully MANAGED (has been
    /// fetched from the database before) and any changes to its properties
    /// are detected, then a reference to the entity is stored there to
    /// mark it for an update.
    ///
    /// `collectionDeletions`: If a `PersistentCollection` has been
    /// de-referenced in a fully MANAGED entity, the reference is stored
    /// there to mark it for deletion.
    pub fn compute_change_set<'a>(
        &'a self,
        class: &'a Arc<ClassMetadata>,
        entity: &'a Object,
    ) -> Fut<'a, ()> {
        Box::pin(async move { self.do_compute_change_set(class, entity).await })
    }

    #[allow(clippy::too_many_lines, reason = "computeChangeSet(), in its order")]
    async fn do_compute_change_set(
        &self,
        class: &Arc<ClassMetadata>,
        entity: &Object,
    ) -> Result<(), Thrown> {
        let em = self.em()?;
        let oid = object_id(entity);

        if self.st().read_only_objects.contains(&oid) {
            return Ok(());
        }

        let class = if class.read().is_inheritance_type_none() {
            class.clone()
        } else {
            em.get_class_metadata(entity.class())?
        };

        let invoke =
            self.subscribed(&em, &class, events::PRE_FLUSH) & !ListenersInvoker::INVOKE_MANAGER;
        if invoke != ListenersInvoker::INVOKE_NONE {
            let args = ManagerEventArgs::new(event::PRE_FLUSH_EVENT_ARGS, em.clone());
            let info = class.read();
            self.listeners_invoker.invoke(
                &info,
                &em.get_event_manager(),
                events::PRE_FLUSH,
                entity,
                &args,
                invoke,
            )?;
        }

        let accessors: Vec<(
            String,
            Arc<dyn crate::mapping::property_accessors::PropertyAccessor>,
        )> = class
            .read()
            .property_accessors
            .iter()
            .map(|(name, accessor)| (name.to_owned(), accessor.clone()))
            .collect();

        let mut actual_data = PhpArray::new();
        for (name, refl_prop) in &accessors {
            let mut value = refl_prop.get_value(entity)?;
            let (is_collection, assoc) = {
                let info = class.read();
                (
                    info.is_collection_valued_association(name),
                    info.association_mappings.get(name).cloned(),
                )
            };
            if is_collection && !value.is_null() {
                let elements = if let Some(pc) = PersistentCollection::from_value(&value) {
                    if pc.get_owner().is_some_and(|owner| same(&owner, entity)) {
                        actual_data.insert(name.as_str(), value);
                        continue;
                    }
                    pc.get_values().await?
                } else if let Some(ac) = ArrayCollectionObject::from_value(&value) {
                    ac.lock().to_array()
                } else if let Value::Array(array) = &value {
                    array.clone()
                } else {
                    return Err(Thrown::new(
                        "TypeError",
                        format!(
                            "Doctrine\\Common\\Collections\\ArrayCollection::__construct(): Argument #1 ($elements) must be of type array, {} given",
                            value.debug_type()
                        ),
                        0,
                    ));
                };
                let Some(assoc) = assoc else { continue };
                let target = em.get_class_metadata(&assoc.target_entity)?;
                let pc = PersistentCollection::new(
                    Some(&em),
                    Some(target),
                    ArrayCollection::new(elements),
                );
                pc.set_owner(entity, &assoc);
                let not_empty = !pc.unwrap().is_empty();
                pc.set_dirty(not_empty);
                value = pc.value();
                refl_prop.set_value(entity, value.clone())?;
                actual_data.insert(name.as_str(), value);
                continue;
            }
            let keep = {
                let info = class.read();
                (!info.is_identifier(name) || !info.is_id_generator_identity())
                    && info.version_field.as_deref() != Some(name.as_str())
            };
            if keep {
                actual_data.insert(name.as_str(), value);
            }
        }

        let original = self.st().original_entity_data.get(&oid).cloned();
        match original {
            None => {
                // Entity is either NEW or MANAGED but not yet fully
                // persisted (only has an id). These result in an INSERT.
                let mut change_set = PhpArray::new();
                {
                    let info = class.read();
                    for (prop_name, actual_value) in actual_data.iter() {
                        let prop = crate::utility::key_text(prop_name);
                        match info.association_mappings.get(&prop) {
                            None => {
                                change_set.insert(
                                    prop_name.clone(),
                                    pair(Value::Null, actual_value.clone()),
                                );
                            }
                            Some(assoc) if assoc.is_to_one_owning_side() => {
                                change_set.insert(
                                    prop_name.clone(),
                                    pair(Value::Null, actual_value.clone()),
                                );
                            }
                            Some(_) => {}
                        }
                    }
                }
                let mut st = self.st();
                st.original_entity_data.insert(oid, actual_data.clone());
                st.entity_change_sets.insert(oid, change_set);
            }
            Some(original_data) => {
                // Entity is "fully" MANAGED: it was already fully
                // persisted before and we have a copy of the original data
                let mut change_set = PhpArray::new();
                for (prop_key, actual_value) in actual_data.iter() {
                    let prop_name = crate::utility::key_text(prop_key);
                    // skip field, its a partially omitted one!
                    let Some(org_value) = original_data.get(prop_key).cloned() else {
                        continue;
                    };
                    let enum_type = class
                        .read()
                        .field_mappings
                        .get(&prop_name)
                        .and_then(|m| m.enum_type.clone());
                    let org_value = if enum_type.is_some_and(|t| !t.is_empty()) {
                        match org_value {
                            Value::Array(values) => {
                                let mut out = PhpArray::new();
                                for (k, v) in values.iter() {
                                    out.insert(
                                        k.clone(),
                                        crate::utility::identifier_flattener_backed(v.clone()),
                                    );
                                }
                                Value::Array(out)
                            }
                            other => crate::utility::identifier_flattener_backed(other),
                        }
                    } else {
                        org_value
                    };

                    if org_value.identical(actual_value) {
                        continue;
                    }

                    let assoc = class.read().association_mappings.get(&prop_name).cloned();
                    let Some(assoc) = assoc else {
                        change_set.insert(prop_key.clone(), pair(org_value, actual_value.clone()));
                        continue;
                    };

                    // Persistent collection was exchanged with the "originally"
                    // created one. This can only mean it was cloned and replaced
                    // on another entity.
                    if let Some(actual_coll) = PersistentCollection::from_value(actual_value) {
                        match actual_coll.get_owner() {
                            None => actual_coll.set_owner(entity, &assoc),
                            Some(owner) if !same(&owner, entity) => {
                                actual_coll.initialize().await?;
                                let new_value = actual_coll.clone_collection().await?;
                                new_value.set_owner(entity, &assoc);
                                class.read().set_field_value(
                                    entity,
                                    &prop_name,
                                    new_value.value(),
                                )?;
                            }
                            Some(_) => {}
                        }
                    }

                    if let Some(org_coll) = PersistentCollection::from_value(&org_value) {
                        // A PersistentCollection was de-referenced, so delete it.
                        let coid = org_coll.oid();
                        let mut st = self.st();
                        if st.collection_deletions.contains(&coid) {
                            continue;
                        }
                        st.collection_deletions.insert(coid, org_coll);
                        change_set.insert(prop_key.clone(), org_value.clone()); // Signal changeset, to-many assocs will be ignored.
                        continue;
                    }

                    if assoc.is_to_one() {
                        if assoc.is_owning_side() {
                            change_set.insert(
                                prop_key.clone(),
                                pair(org_value.clone(), actual_value.clone()),
                            );
                        }
                        if let (Value::Object(org), true) = (&org_value, assoc.orphan_removal) {
                            self.schedule_orphan_removal(org);
                        }
                    }
                }

                if !change_set.is_empty() {
                    let mut st = self.st();
                    st.entity_change_sets.insert(oid, change_set);
                    st.original_entity_data.insert(oid, actual_data.clone());
                    st.entity_updates.insert(oid, entity.clone());
                }
            }
        }

        // Look for changes in associations of the entity
        let associations: Vec<AssociationMapping> = class
            .read()
            .association_mappings
            .values()
            .cloned()
            .collect();
        for assoc in associations {
            let val = class.read().get_field_value(entity, &assoc.field_name)?;
            if val.is_null() {
                continue;
            }
            self.compute_association_changes(&assoc, &val).await?;

            if let Some(pc) = PersistentCollection::from_value(&val) {
                let mut st = self.st();
                if !st.entity_change_sets.contains_key(&oid)
                    && assoc.is_many_to_many_owning_side()
                    && pc.is_dirty()
                {
                    st.entity_change_sets.insert(oid, PhpArray::new());
                    st.original_entity_data.insert(oid, actual_data.clone());
                    st.entity_updates.insert(oid, entity.clone());
                }
            }
        }
        Ok(())
    }

    /// Computes all the changes that have been done to entities and
    /// collections since the last commit and stores these changes in the
    /// _entityChangeSet map temporarily for access by the persisters,
    /// until the `UoW` commit is finished.
    pub async fn compute_change_sets(&self) -> Result<(), Thrown> {
        let em = self.em()?;
        // Compute changes for INSERTed entities first. This must always
        // happen.
        self.compute_schedule_inserts_change_sets().await?;

        // Compute changes for other MANAGED entities. Change tracking
        // policies take effect here.
        let classes: Vec<(String, Vec<Object>)> = self
            .st()
            .identity_map
            .iter()
            .map(|(k, v)| (k.clone(), v.values().cloned().collect()))
            .collect();
        for (class_name, entities) in classes {
            let class = em.get_class_metadata(&class_name)?;

            // Skip class if instances are read-only
            if class.read().is_read_only {
                continue;
            }

            let entities_to_process: Vec<Object> =
                if class.read().is_change_tracking_deferred_implicit() {
                    entities
                } else {
                    self.st()
                        .scheduled_for_synchronization
                        .get(&class_name)
                        .map(|m| m.values().cloned().collect())
                        .unwrap_or_default()
                };

            for entity in entities_to_process {
                // Ignore uninitialized proxy objects
                if self.is_uninitialized_object(&Value::Object(entity.clone())) {
                    continue;
                }
                // Only MANAGED entities that are NOT SCHEDULED FOR INSERTION
                // OR DELETION are processed here.
                let oid = object_id(&entity);
                let process = {
                    let st = self.st();
                    !st.entity_insertions.contains(&oid)
                        && !st.entity_deletions.contains(&oid)
                        && st.entity_states.contains_key(&oid)
                };
                if process {
                    self.compute_change_set(&class, &entity).await?;
                }
            }
        }
        Ok(())
    }

    /// Computes the changes of an association.
    fn compute_association_changes<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        value: &'a Value,
    ) -> Fut<'a, ()> {
        Box::pin(async move {
            if self.is_uninitialized_object(value) {
                return Ok(());
            }
            let em = self.em()?;

            // If this collection is dirty, schedule it for updates
            if let Some(pc) = PersistentCollection::from_value(value)
                && pc.is_dirty()
            {
                let coid = pc.oid();
                let mut st = self.st();
                st.collection_updates.insert(coid, pc.clone());
                st.visited_collections.insert(coid, pc);
            }

            // Look through the entities, and in any of their associations,
            // for transient (new) entities, recursively. ("Persistence by
            // reachability") Unwrap. Uninitialized collections will simply
            // be empty.
            let unwrapped: Vec<(Key, Value)> = if assoc.is_to_one() {
                vec![(Key::Int(0), value.clone())]
            } else if let Some(pc) = PersistentCollection::from_value(value) {
                pc.unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            } else if let Some(ac) = ArrayCollectionObject::from_value(value) {
                ac.lock()
                    .to_array()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            } else if let Value::Array(array) = value {
                array.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                Vec::new()
            };
            let target_class = em.get_class_metadata(&assoc.target_entity)?;
            let target_name = target_class.read().name.clone();

            for (key, entry) in unwrapped {
                let Value::Object(entry_object) = &entry else {
                    return Err(exception::invalid_association(
                        &target_name,
                        assoc,
                        &entry.debug_type(),
                    ));
                };
                if !entry_object.instance_of(&target_name) {
                    return Err(exception::invalid_association(
                        &target_name,
                        assoc,
                        &entry.debug_type(),
                    ));
                }

                let state = self.get_entity_state_assumed(entry_object, STATE_NEW);

                if !entry_object.instance_of(&assoc.target_entity) {
                    return Err(exception::unexpected_association_value(
                        &assoc.source_entity,
                        &assoc.field_name,
                        &entry.debug_type(),
                        &assoc.target_entity,
                    ));
                }

                match state {
                    STATE_NEW => {
                        if !assoc.is_cascade_persist() {
                            // For now just record the details, because this
                            // may not be an issue if we later discover
                            // another pathway through the object graph where
                            // cascade-persistence is enabled for this
                            // object.
                            self.st().non_cascaded_new_detected_entities.insert(
                                object_id(entry_object),
                                (assoc.clone(), entry_object.clone()),
                            );
                            continue;
                        }
                        self.persist_new(&target_class, entry_object)?;
                        self.compute_change_set(&target_class, entry_object).await?;
                    }
                    STATE_REMOVED => {
                        // Consume the $value as array (it's either an array
                        // or an ArrayAccess) and remove the element from the
                        // collection.
                        if !assoc.is_to_many() {
                            continue;
                        }
                        if let Some(pc) = PersistentCollection::from_value(value) {
                            let coid = pc.oid();
                            let mut st = self.st();
                            st.visited_collections.insert(coid, pc);
                            let keys = st
                                .pending_collection_element_removals
                                .entry(coid)
                                .or_default();
                            if !keys.contains(&key) {
                                keys.push(key);
                            }
                        }
                    }
                    STATE_DETACHED => {
                        // Can actually not happen right now as we assume
                        // STATE_NEW, so the exception will be raised from
                        // the DBAL layer (constraint violation).
                        return Err(exception::detached_entity_found_through_relationship(
                            assoc,
                            entry_object,
                        ));
                    }
                    _ => {
                        // MANAGED associated entities are already taken
                        // into account during changeset calculation.
                    }
                }
            }
            Ok(())
        })
    }

    fn persist_new(&self, class: &Arc<ClassMetadata>, entity: &Object) -> Result<(), Thrown> {
        let em = self.em()?;
        let oid = object_id(entity);
        let invoke = self.subscribed(&em, class, events::PRE_PERSIST);
        if invoke != ListenersInvoker::INVOKE_NONE {
            self.invoke_lifecycle(
                &em,
                class,
                events::PRE_PERSIST,
                event::PRE_PERSIST_EVENT_ARGS,
                entity,
                invoke,
            )?;
        }

        let id_gen = class
            .read()
            .id_generator
            .clone()
            .unwrap_or(IdGenerator::Assigned);
        if !id_gen.is_post_insert_generator() {
            let id_value = match &id_gen {
                IdGenerator::Assigned => self.generate_assigned_id(&em, entity)?,
                IdGenerator::Sequence { sequence_name, .. } => {
                    // The sequence's next value needs a query the platform
                    // gives; SQLite has no sequences.
                    em.get_platform()
                        .get_sequence_next_val_sql(sequence_name)
                        .map_err(|e| exception::from_dbal(&e))?;
                    return Err(exception::not_supported());
                }
                _ => return Err(exception::not_supported()),
            };
            if !self.has_missing_ids_which_are_foreign_keys(class, &id_value) {
                self.st().entity_identifiers.insert(oid, id_value);
            }
        }

        let scheduled = {
            let mut st = self.st();
            st.entity_states.insert(oid, STATE_MANAGED);
            st.entity_insertions.contains(&oid)
        };
        if !scheduled {
            self.schedule_for_insert(entity)?;
        }
        Ok(())
    }

    /// `AssignedGenerator::generateId()`: the identifier values the entity
    /// was given.
    fn generate_assigned_id(
        &self,
        em: &EntityManager,
        entity: &Object,
    ) -> Result<PhpArray, Thrown> {
        let class = em.get_class_metadata(entity.class())?;
        let (id_fields, assoc_fields) = {
            let info = class.read();
            (
                info.get_identifier_field_names().to_vec(),
                info.association_mappings
                    .keys()
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
            )
        };
        let mut identifier = PhpArray::new();
        for id_field in id_fields {
            let mut value = class.read().get_field_value(entity, &id_field)?;
            if value.is_null() {
                return Err(exception::entity_missing_assigned_id(entity, &id_field));
            }
            if assoc_fields.contains(&id_field)
                && let Value::Object(related) = &value
            {
                value = self.get_single_identifier_value(related)?;
            }
            identifier.insert(id_field.as_str(), value);
        }
        Ok(identifier)
    }

    #[allow(clippy::unused_self, reason = "an instance method upstream")]
    fn has_missing_ids_which_are_foreign_keys(
        &self,
        class: &ClassMetadata,
        id_value: &PhpArray,
    ) -> bool {
        let info = class.read();
        id_value.iter().any(|(field, value)| {
            value.is_null()
                && info
                    .association_mappings
                    .contains_key(&crate::utility::key_text(field))
        })
    }

    /// INTERNAL: Computes the changeset of an individual entity,
    /// independently of the `computeChangeSets()` routine that is used at
    /// the beginning of a `UnitOfWork#commit()`.
    ///
    /// The passed entity must be a managed entity. If the entity already
    /// has a change set because this method is invoked during a commit
    /// cycle then the change sets are added whereby changes detected in
    /// this method prevail.
    pub fn recompute_single_entity_change_set(
        &self,
        class: &Arc<ClassMetadata>,
        entity: &Object,
    ) -> Result<(), Thrown> {
        let em = self.em()?;
        let oid = object_id(entity);
        if self.st().entity_states.get(&oid) != Some(&STATE_MANAGED) {
            return Err(exception::entity_not_managed(entity));
        }
        let class = if class.read().is_inheritance_type_none() {
            class.clone()
        } else {
            em.get_class_metadata(entity.class())?
        };

        let mut actual_data = PhpArray::new();
        {
            let info = class.read();
            for (name, refl_prop) in info.property_accessors.iter() {
                if (!info.is_identifier(name) || !info.is_id_generator_identity())
                    && info.version_field.as_deref() != Some(name)
                    && !info.is_collection_valued_association(name)
                {
                    actual_data.insert(name, refl_prop.get_value(entity)?);
                }
            }
        }

        let Some(original_data) = self.st().original_entity_data.get(&oid).cloned() else {
            return Err(Thrown::new(
                "RuntimeException",
                "Cannot call recomputeSingleEntityChangeSet before computeChangeSet on an entity.",
                0,
            ));
        };

        let mut change_set = PhpArray::new();
        for (prop_key, actual_value) in actual_data.iter() {
            let prop_name = crate::utility::key_text(prop_key);
            let mut org_value = original_data.get(prop_key).cloned().unwrap_or(Value::Null);
            let is_enum = class
                .read()
                .field_mappings
                .get(&prop_name)
                .is_some_and(|m| m.enum_type.is_some());
            if is_enum {
                org_value = match org_value {
                    Value::Array(values) => {
                        let mut out = PhpArray::new();
                        for (k, v) in values.iter() {
                            out.insert(
                                k.clone(),
                                crate::utility::identifier_flattener_backed(v.clone()),
                            );
                        }
                        Value::Array(out)
                    }
                    other => crate::utility::identifier_flattener_backed(other),
                };
            }
            if !org_value.identical(actual_value) {
                change_set.insert(prop_key.clone(), pair(org_value, actual_value.clone()));
            }
        }

        if !change_set.is_empty() {
            let mut st = self.st();
            if let Some(existing) = st.entity_change_sets.get_mut(&oid) {
                for (k, v) in change_set.iter() {
                    existing.insert(k.clone(), v.clone());
                }
            } else if !st.entity_insertions.contains(&oid) {
                st.entity_change_sets.insert(oid, change_set);
                st.entity_updates.insert(oid, entity.clone());
            }
            st.original_entity_data.insert(oid, actual_data);
        }
        Ok(())
    }

    /// Executes entity insertions.
    async fn execute_inserts(&self) -> Result<(), Thrown> {
        let em = self.em()?;
        let order = self.compute_insert_execution_order()?;

        // InsertBatch::batchByEntityType()
        let mut batches: Vec<(Arc<ClassMetadata>, Vec<Object>)> = Vec::new();
        for entity in order {
            let entity_class = em.get_class_metadata(entity.class())?;
            let assigned = matches!(
                entity_class.read().id_generator,
                Some(IdGenerator::Assigned)
            );
            match batches.last_mut() {
                Some((current, entities))
                    if current.read().name == entity_class.read().name && assigned =>
                {
                    entities.push(entity);
                }
                _ => batches.push((entity_class, vec![entity])),
            }
        }

        let mut events_to_dispatch: Vec<(Arc<ClassMetadata>, Object, i32)> = Vec::new();
        for (class, entities) in batches {
            let invoke = self.subscribed(&em, &class, events::POST_PERSIST);
            let persister = self.get_entity_persister(&class.read().name.clone())?;
            for entity in &entities {
                persister.add_insert(entity);
                self.st().entity_insertions.remove(&object_id(entity));
            }
            persister.execute_inserts().await?;
            for entity in entities {
                let oid = object_id(&entity);
                if !self.st().entity_identifiers.contains_key(&oid) {
                    // entity was not added to identity map because some
                    // identifiers are foreign keys to new entities. add it
                    // now
                    self.add_to_entity_identifiers_and_entity_map(&class, oid, &entity)?;
                }
                if invoke != ListenersInvoker::INVOKE_NONE {
                    events_to_dispatch.push((class.clone(), entity, invoke));
                }
            }
        }

        // Defer dispatching `postPersist` events to until all entities
        // have been inserted and post-insert IDs have been assigned.
        for (class, entity, invoke) in events_to_dispatch {
            self.invoke_lifecycle(
                &em,
                &class,
                events::POST_PERSIST,
                event::POST_PERSIST_EVENT_ARGS,
                &entity,
                invoke,
            )?;
        }
        Ok(())
    }

    fn add_to_entity_identifiers_and_entity_map(
        &self,
        class: &ClassMetadata,
        oid: usize,
        entity: &Object,
    ) -> Result<(), Thrown> {
        let mut identifier = PhpArray::new();
        let (id_fields, assoc_fields) = {
            let info = class.read();
            (
                info.get_identifier_field_names().to_vec(),
                info.association_mappings
                    .keys()
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
            )
        };
        for id_field in id_fields {
            let orig_value = class.read().get_field_value(entity, &id_field)?;
            let mut value = Value::Null;
            if assoc_fields.contains(&id_field)
                && let Value::Object(related) = &orig_value
            {
                // we have a temporary identifier
                value = self.get_single_identifier_value(related)?;
            }
            identifier.insert(
                id_field.as_str(),
                if value.is_null() {
                    orig_value.clone()
                } else {
                    value
                },
            );
            self.st()
                .original_entity_data
                .entry(oid)
                .or_default()
                .insert(id_field.as_str(), orig_value);
        }
        {
            let mut st = self.st();
            st.entity_states.insert(oid, STATE_MANAGED);
            st.entity_identifiers.insert(oid, identifier);
        }
        self.add_to_identity_map(entity)?;
        Ok(())
    }

    /// Executes all entity updates for entities of the specified type.
    async fn execute_updates(&self) -> Result<(), Thrown> {
        let em = self.em()?;
        let entities = self.compute_update_execution_order()?;
        for entity in entities {
            let oid = object_id(&entity);
            let class = em.get_class_metadata(entity.class())?;
            let persister = self.get_entity_persister(&class.read().name.clone())?;
            let pre_update_invoke = self.subscribed(&em, &class, events::PRE_UPDATE);
            let post_update_invoke = self.subscribed(&em, &class, events::POST_UPDATE);

            if pre_update_invoke != ListenersInvoker::INVOKE_NONE {
                let change_set = Handle::new(self.get_entity_change_set(&entity));
                let args = PreUpdateEventArgs::new(entity.clone(), em.clone(), change_set.clone());
                {
                    let info = class.read();
                    self.listeners_invoker.invoke(
                        &info,
                        &em.get_event_manager(),
                        events::PRE_UPDATE,
                        &entity,
                        &args,
                        pre_update_invoke,
                    )?;
                }
                {
                    let mut st = self.st();
                    if st.entity_change_sets.contains_key(&oid) {
                        st.entity_change_sets.insert(oid, change_set.get());
                    }
                }
                self.recompute_single_entity_change_set(&class, &entity)?;
            }

            let has_changes = self
                .st()
                .entity_change_sets
                .get(&oid)
                .is_some_and(|set| !set.is_empty());
            if has_changes {
                persister.update(&entity).await?;
            }

            self.st().entity_updates.remove(&oid);

            if post_update_invoke != ListenersInvoker::INVOKE_NONE {
                self.invoke_lifecycle(
                    &em,
                    &class,
                    events::POST_UPDATE,
                    event::POST_UPDATE_EVENT_ARGS,
                    &entity,
                    post_update_invoke,
                )?;
            }
        }
        Ok(())
    }

    /// Executes all entity deletions for entities of the specified type.
    async fn execute_deletions(&self) -> Result<(), Thrown> {
        let em = self.em()?;
        let entities = self.compute_delete_execution_order()?;
        let mut events_to_dispatch: Vec<(Arc<ClassMetadata>, Object, i32)> = Vec::new();

        for entity in entities {
            self.remove_from_identity_map(&entity)?;
            let oid = object_id(&entity);
            let class = em.get_class_metadata(entity.class())?;
            let persister = self.get_entity_persister(&class.read().name.clone())?;
            let invoke = self.subscribed(&em, &class, events::POST_REMOVE);

            persister.delete(&entity).await?;

            {
                let mut st = self.st();
                st.entity_deletions.remove(&oid);
                st.entity_identifiers.remove(&oid);
                st.original_entity_data.remove(&oid);
                st.entity_states.remove(&oid);
            }

            // Entity with this $oid after deletion treated as NEW, even if
            // the $oid is obtained by a new entity because the old one went
            // out of scope...
            {
                let info = class.read();
                if !info.is_identifier_natural()
                    && let Some(first) = info.identifier.first()
                    && let Some(accessor) = info.property_accessors.get(first)
                    && !accessor.class_name().ends_with("ReadonlyAccessor")
                {
                    accessor.set_value(&entity, Value::Null)?;
                }
            }

            if invoke != ListenersInvoker::INVOKE_NONE {
                events_to_dispatch.push((class, entity, invoke));
            }
        }

        // Defer dispatching `postRemove` events to until all entities have
        // been removed.
        for (class, entity, invoke) in events_to_dispatch {
            self.invoke_lifecycle(
                &em,
                &class,
                events::POST_REMOVE,
                event::POST_REMOVE_EVENT_ARGS,
                &entity,
                invoke,
            )?;
        }
        Ok(())
    }

    fn compute_insert_execution_order(&self) -> Result<Vec<Object>, Thrown> {
        let em = self.em()?;
        let insertions: Vec<Object> = self.st().entity_insertions.values().cloned().collect();
        let mut sort = TopologicalSort::new();
        for entity in &insertions {
            sort.add_node(entity);
        }

        // Now add edges
        for entity in &insertions {
            let class = em.get_class_metadata(entity.class())?;
            let info = class.read();
            for assoc in info.association_mappings.values() {
                // We only need to consider the owning sides of to-one
                // associations, since many-to-many associations are
                // persisted at a later step and have no insertion order
                // problems (all entities already in the database at that
                // time).
                if !assoc.is_to_one_owning_side() {
                    continue;
                }
                let Value::Object(target_entity) =
                    info.get_field_value(entity, &assoc.field_name)?
                else {
                    continue;
                };
                // An entity that references back to itself _and_ uses an
                // application-provided ID (the "NONE" generator strategy)
                // can be handled by inserting and updating it in one
                // step.
                if !sort.has_node(&target_entity) {
                    continue;
                }
                if same(&target_entity, entity) && info.is_identifier_natural() {
                    continue;
                }
                // According to https://www.doctrine-project.org/projects/doctrine-orm/en/2.14/reference/annotations-reference.html#annref_joincolumn,
                // the default for "nullable" is true. Unfortunately, it
                // seems this default is not applied at the metadata
                // driver, factory or other level, but in fact we may have
                // an undefined 'nullable' key here, so we must assume
                // that default here as well.
                //
                // Same in \Doctrine\ORM\Tools\EntityGenerator::isAssociationIsNullable
                // or \Doctrine\ORM\Persisters\Entity\BasicEntityPersister::getJoinSQLForJoinColumns,
                // to give two examples.
                let is_nullable = assoc
                    .join_columns
                    .first()
                    .is_none_or(|jc| jc.nullable.unwrap_or(true));

                // Edges point from the entity that must be inserted first
                // to the entity that must be inserted later. Thus,
                // reverse the direction of the edge.
                sort.add_edge(entity, &target_entity, is_nullable);
            }
        }

        sort.sort().map_err(|e| e.to_thrown())
    }

    fn compute_update_execution_order(&self) -> Result<Vec<Object>, Thrown> {
        let entities: Vec<Object> = self.st().entity_updates.values().cloned().collect();
        let mut keyed: Vec<(String, String, Object)> = Vec::with_capacity(entities.len());
        for entity in entities {
            let hash = self.get_id_hash_by_entity(&entity)?;
            keyed.push((entity.class().to_owned(), hash, entity));
        }
        keyed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        Ok(keyed.into_iter().map(|(_, _, e)| e).collect())
    }

    fn compute_delete_execution_order(&self) -> Result<Vec<Object>, Thrown> {
        let em = self.em()?;
        let deletions: Vec<Object> = self.st().entity_deletions.values().cloned().collect();
        let mut strongly_connected_components = StronglyConnectedComponents::new();
        let mut sort = TopologicalSort::new();

        for entity in &deletions {
            strongly_connected_components.add_node(entity);
            sort.add_node(entity);
        }

        // First, consider only "on delete cascade" associations between
        // entities and find strongly connected groups. Once we delete any
        // one of the entities in such a group, _all_ of the other entities
        // will be removed as well. So, we need to treat those groups like
        // a single entity when performing delete order topological
        // sorting.
        for entity in &deletions {
            let class = em.get_class_metadata(entity.class())?;
            let info = class.read();
            for assoc in info.association_mappings.values() {
                // We only need to consider the owning sides of to-one
                // associations, since many-to-many associations can
                // always be (and have already been) deleted in a
                // preceding step.
                if !assoc.is_to_one_owning_side() {
                    continue;
                }
                let Some(on_delete) = assoc
                    .join_columns
                    .first()
                    .and_then(|jc| jc.on_delete.clone())
                else {
                    continue;
                };
                if !on_delete.eq_ignore_ascii_case("cascade") {
                    continue;
                }
                let Value::Object(target_entity) =
                    info.get_field_value(entity, &assoc.field_name)?
                else {
                    continue;
                };
                // If the association does not refer to another entity or
                // that entity is not to be deleted, there is no ordering
                // problem and we can skip this particular association.
                if !strongly_connected_components.has_node(&target_entity) {
                    continue;
                }
                strongly_connected_components.add_edge(entity, &target_entity);
            }
        }

        strongly_connected_components.find_strongly_connected_components();

        // Now do the actual topological sorting to find the delete order.
        for entity in &deletions {
            let class = em.get_class_metadata(entity.class())?;
            let info = class.read();

            // Get the entities representing the SCC
            let entity_component = strongly_connected_components
                .get_node_representing_strongly_connected_component(entity)?;

            // When $entity is part of a non-trivial strongly connected
            // component group (a group containing not only those entities
            // alone), make sure we process it _after_ the entity
            // representing the group. The dependency direction implies
            // that "$entity depends on $entityComponent being deleted
            // first". The topological sort will output the depended-upon
            // nodes first.
            if !same(&entity_component, entity) {
                sort.add_edge(entity, &entity_component, false);
            }

            for assoc in info.association_mappings.values() {
                // We only need to consider the owning sides of to-one
                // associations, since many-to-many associations can
                // always be (and have already been) deleted in a
                // preceding step.
                if !assoc.is_to_one_owning_side() {
                    continue;
                }
                // For associations that implement a database-level "set
                // null" operation, we do not have to follow a particular
                // order: If the referred-to entity is deleted first, the
                // DBMS will temporarily set the foreign key to NULL (SET
                // NULL). If the other entity is deleted first, there's no
                // need to break the cycle.
                if let Some(on_delete) = assoc
                    .join_columns
                    .first()
                    .and_then(|jc| jc.on_delete.clone())
                    && on_delete.eq_ignore_ascii_case("set null")
                {
                    continue;
                }
                let Value::Object(target_entity) =
                    info.get_field_value(entity, &assoc.field_name)?
                else {
                    continue;
                };
                // If the association does not refer to another entity or
                // that entity is not to be deleted, there is no ordering
                // problem and we can skip this particular association.
                if !sort.has_node(&target_entity) {
                    continue;
                }
                let target_entity_component = strongly_connected_components
                    .get_node_representing_strongly_connected_component(&target_entity)?;

                // When we have a dependency between two different groups
                // of strongly connected nodes, add it to the dependency
                // graph. The dependency direction implies that "the
                // $targetEntityComponent depends on $entityComponent being
                // deleted first". The topological sort will output the
                // depended-upon nodes first, so we can work through the
                // result in the returned order.
                if !same(&target_entity_component, &entity_component) {
                    sort.add_edge(&target_entity_component, &entity_component, false);
                }
            }
        }

        sort.sort().map_err(|e| e.to_thrown())
    }

    /// Schedules an entity for insertion into the database. If the entity
    /// already has an identifier, it will be added to the identity map.
    pub fn schedule_for_insert(&self, entity: &Object) -> Result<(), Thrown> {
        let oid = object_id(entity);
        let has_identifier = {
            let mut st = self.st();
            if st.entity_updates.contains(&oid) {
                return Err(Thrown::new(
                    "InvalidArgumentException",
                    "Dirty entity can not be scheduled for insertion.",
                    0,
                ));
            }
            if st.entity_deletions.contains(&oid) {
                return Err(exception::schedule_insert_for_removed_entity(entity));
            }
            if st.original_entity_data.contains_key(&oid) && !st.entity_insertions.contains(&oid) {
                return Err(exception::schedule_insert_for_managed_entity(entity));
            }
            if st.entity_insertions.contains(&oid) {
                return Err(exception::schedule_insert_twice(entity));
            }
            st.entity_insertions.insert(oid, entity.clone());
            st.entity_identifiers.contains_key(&oid)
        };
        if has_identifier {
            self.add_to_identity_map(entity)?;
        }
        Ok(())
    }

    /// Checks whether an entity is scheduled for insertion.
    pub fn is_scheduled_for_insert(&self, entity: &Object) -> bool {
        self.st().entity_insertions.contains(&object_id(entity))
    }

    /// Schedules an entity for being updated.
    pub fn schedule_for_update(&self, entity: &Object) -> Result<(), Thrown> {
        let oid = object_id(entity);
        let mut st = self.st();
        if !st.entity_identifiers.contains_key(&oid) {
            return Err(exception::entity_has_no_identity(
                entity,
                "scheduling for update",
            ));
        }
        if st.entity_deletions.contains(&oid) {
            return Err(exception::entity_is_removed(entity, "schedule for update"));
        }
        if !st.entity_updates.contains(&oid) && !st.entity_insertions.contains(&oid) {
            st.entity_updates.insert(oid, entity.clone());
        }
        Ok(())
    }

    /// INTERNAL: Schedules an extra update that will be executed
    /// immediately after the regular entity updates within the currently
    /// running commit cycle.
    ///
    /// Extra updates for entities are stored as (entity, changeset)
    /// tuples.
    pub fn schedule_extra_update(&self, entity: &Object, changeset: PhpArray) {
        let oid = object_id(entity);
        let mut st = self.st();
        let mut changeset = changeset;
        if let Some((_, changeset2)) = st.extra_updates.get(&oid) {
            // $changeset + $changeset2
            changeset.union(changeset2);
        }
        st.extra_updates.insert(oid, (entity.clone(), changeset));
    }

    /// The extra updates scheduled for the running commit, as (entity,
    /// changeset) tuples: what a test reads of the private
    /// `$extraUpdates` through `ReflectionProperty`.
    pub fn get_scheduled_extra_updates(&self) -> Vec<(Object, PhpArray)> {
        self.st().extra_updates.values().cloned().collect()
    }

    /// Checks whether an entity is registered as dirty in the unit of
    /// work. Note: Is not very useful currently as dirty entities are only
    /// registered at commit time.
    pub fn is_scheduled_for_update(&self, entity: &Object) -> bool {
        self.st().entity_updates.contains(&object_id(entity))
    }

    /// Checks whether an entity is registered to be checked in the unit
    /// of work.
    pub fn is_scheduled_for_dirty_check(&self, entity: &Object) -> Result<bool, Thrown> {
        let root = self
            .em()?
            .get_class_metadata(entity.class())?
            .read()
            .root_entity_name
            .clone();
        Ok(self
            .st()
            .scheduled_for_synchronization
            .get(&root)
            .is_some_and(|m| m.contains(&object_id(entity))))
    }

    /// INTERNAL: Schedules an entity for deletion.
    pub fn schedule_for_delete(&self, entity: &Object) -> Result<(), Thrown> {
        let oid = object_id(entity);
        if self.st().entity_insertions.contains(&oid) {
            if self.is_in_identity_map(entity)? {
                self.remove_from_identity_map(entity)?;
            }
            let mut st = self.st();
            st.entity_insertions.remove(&oid);
            st.entity_states.remove(&oid);
            return Ok(()); // entity has not been persisted yet, so nothing more to do.
        }
        if !self.is_in_identity_map(entity)? {
            return Ok(());
        }
        let mut st = self.st();
        st.entity_updates.remove(&oid);
        if !st.entity_deletions.contains(&oid) {
            st.entity_deletions.insert(oid, entity.clone());
            st.entity_states.insert(oid, STATE_REMOVED);
        }
        Ok(())
    }

    /// Checks whether an entity is registered as removed/deleted with the
    /// unit of work.
    pub fn is_scheduled_for_delete(&self, entity: &Object) -> bool {
        self.st().entity_deletions.contains(&object_id(entity))
    }

    /// Checks whether an entity is scheduled for insertion, update or
    /// deletion.
    pub fn is_entity_scheduled(&self, entity: &Object) -> bool {
        let oid = object_id(entity);
        let st = self.st();
        st.entity_insertions.contains(&oid)
            || st.entity_updates.contains(&oid)
            || st.entity_deletions.contains(&oid)
    }

    /// INTERNAL: Registers an entity in the identity map. Note that
    /// entities in a hierarchy are registered with the class name of the
    /// root entity.
    pub fn add_to_identity_map(&self, entity: &Object) -> Result<bool, Thrown> {
        let class_metadata = self.em()?.get_class_metadata(entity.class())?;
        let id_hash = self.get_id_hash_by_entity(entity)?;
        let class_name = class_metadata.read().root_entity_name.clone();
        let mut st = self.st();
        if let Some(existing) = st
            .identity_map
            .get(&class_name)
            .and_then(|m| m.get(&id_hash))
        {
            if !same(existing, entity) {
                return Err(exception::entity_identity_collision(
                    existing, entity, &id_hash,
                ));
            }
            return Ok(false);
        }
        st.identity_map
            .get_or_default(class_name)
            .insert(id_hash, entity.clone());
        Ok(true)
    }

    /// Gets the id hash of an entity by its identifier.
    pub fn get_id_hash_by_identifier(identifier: &PhpArray) -> String {
        identifier
            .values()
            .map(|v| to_text(&crate::utility::identifier_flattener_backed(v.clone())))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Gets the id hash of an entity.
    pub fn get_id_hash_by_entity(&self, entity: &Object) -> Result<String, Thrown> {
        let identifier = self
            .st()
            .entity_identifiers
            .get(&object_id(entity))
            .cloned()
            .unwrap_or_default();
        if identifier.is_empty() || identifier.values().any(Value::is_null) {
            let name = self
                .em()?
                .get_class_metadata(entity.class())?
                .read()
                .name
                .clone();
            return Err(exception::entity_without_identity(&name, entity));
        }
        Ok(Self::get_id_hash_by_identifier(&identifier))
    }

    fn get_entity_state_assumed(&self, entity: &Object, assume: i32) -> i32 {
        self.st()
            .entity_states
            .get(&object_id(entity))
            .copied()
            .unwrap_or(assume)
    }

    /// Gets the state of an entity with regard to the current unit of
    /// work, assuming `assume` when the unit of work does not know it.
    pub fn get_entity_state_or(&self, entity: &Object, assume: i32) -> i32 {
        self.get_entity_state_assumed(entity, assume)
    }

    /// Gets the state of an entity with regard to the current unit of
    /// work (it may ask the database whether the entity exists).
    pub async fn get_entity_state(&self, entity: &Object) -> Result<i32, Thrown> {
        if let Some(state) = self.st().entity_states.get(&object_id(entity)) {
            return Ok(*state);
        }
        let em = self.em()?;
        let class = em.get_class_metadata(entity.class())?;
        let (mut id, contains_foreign, natural, versioned, version_field, post_insert, root) = {
            let info = class.read();
            (
                info.get_identifier_values(entity)?,
                info.contains_foreign_identifier || info.contains_enum_identifier,
                info.is_identifier_natural(),
                info.is_versioned,
                info.version_field.clone(),
                info.id_generator
                    .as_ref()
                    .is_some_and(IdGenerator::is_post_insert_generator),
                info.root_entity_name.clone(),
            )
        };
        if id.is_empty() {
            return Ok(STATE_NEW);
        }
        if contains_foreign {
            id = flatten_identifier(&em, &class.read(), &id)?;
        }

        if natural {
            // Check for a version field, if available, to avoid a db
            // lookup.
            if versioned && let Some(version_field) = version_field {
                return Ok(
                    if class
                        .read()
                        .get_field_value(entity, &version_field)?
                        .is_truthy()
                    {
                        STATE_DETACHED
                    } else {
                        STATE_NEW
                    },
                );
            }
            // Last try before db lookup: check the identity map.
            if self.try_get_by_id(&id, &root).is_some() {
                return Ok(STATE_DETACHED);
            }
            // db lookup
            let name = class.read().name.clone();
            if self
                .get_entity_persister(&name)?
                .exists(entity, None)
                .await?
            {
                return Ok(STATE_DETACHED);
            }
            return Ok(STATE_NEW);
        }
        if !post_insert {
            // if we have a pre insert generator we can't be sure that
            // having an id really means that the entity exists. We have
            // to verify this through the last resort: a db lookup
            if self.try_get_by_id(&id, &root).is_some() {
                return Ok(STATE_DETACHED);
            }
            let name = class.read().name.clone();
            if self
                .get_entity_persister(&name)?
                .exists(entity, None)
                .await?
            {
                return Ok(STATE_DETACHED);
            }
            return Ok(STATE_NEW);
        }
        Ok(STATE_DETACHED)
    }

    /// INTERNAL: Removes an entity from the identity map. This effectively
    /// detaches the entity from the persistence management of Doctrine.
    pub fn remove_from_identity_map(&self, entity: &Object) -> Result<bool, Thrown> {
        let oid = object_id(entity);
        let class_metadata = self.em()?.get_class_metadata(entity.class())?;
        let identifier = self
            .st()
            .entity_identifiers
            .get(&oid)
            .cloned()
            .unwrap_or_default();
        let id_hash = Self::get_id_hash_by_identifier(&identifier);
        if id_hash.is_empty() {
            return Err(exception::entity_has_no_identity(
                entity,
                "remove from identity map",
            ));
        }
        let class_name = class_metadata.read().root_entity_name.clone();
        let mut st = self.st();
        if let Some(map) = st.identity_map.get_mut(&class_name)
            && map.remove(&id_hash).is_some()
        {
            st.read_only_objects.remove(&oid);
            return Ok(true);
        }
        Ok(false)
    }

    /// INTERNAL: Gets an entity in the identity map by its identifier
    /// hash.
    pub fn get_by_id_hash(&self, id_hash: &str, root_class_name: &str) -> Option<Object> {
        self.st()
            .identity_map
            .get(&root_class_name.to_owned())
            .and_then(|m| m.get(&id_hash.to_owned()))
            .cloned()
    }

    /// INTERNAL: Tries to get an entity by its identifier hash. If no
    /// entity is found for the given hash, `None` is returned.
    pub fn try_get_by_id_hash(&self, id_hash: &str, root_class_name: &str) -> Option<Object> {
        self.get_by_id_hash(id_hash, root_class_name)
    }

    /// Checks whether an entity is registered in the identity map of this
    /// `UnitOfWork`.
    pub fn is_in_identity_map(&self, entity: &Object) -> Result<bool, Thrown> {
        let oid = object_id(entity);
        let Some(identifier) = self.st().entity_identifiers.get(&oid).cloned() else {
            return Ok(false);
        };
        if identifier.is_empty() {
            return Ok(false);
        }
        let class_metadata = self.em()?.get_class_metadata(entity.class())?;
        let id_hash = Self::get_id_hash_by_identifier(&identifier);
        let root = class_metadata.read().root_entity_name.clone();
        Ok(self
            .st()
            .identity_map
            .get(&root)
            .is_some_and(|m| m.contains(&id_hash)))
    }

    /// Persists an entity as part of the current unit of work.
    pub fn persist(&self, entity: &Object) -> Result<(), Thrown> {
        let mut visited = HashSet::new();
        self.do_persist(entity, &mut visited)
    }

    /// Persists an entity as part of the current unit of work.
    ///
    /// This method is internally called during `persist()` cascades as it
    /// tracks the already visited entities to prevent infinite recursions.
    fn do_persist(&self, entity: &Object, visited: &mut HashSet<usize>) -> Result<(), Thrown> {
        let oid = object_id(entity);
        if !visited.insert(oid) {
            return Ok(()); // Prevent infinite recursion
        }

        let class = self.em()?.get_class_metadata(entity.class())?;

        // We assume NEW, so DETACHED entities result in an exception on
        // flush (constraint violation). Why? Because we can not
        // distinguish NEW from DETACHED without hitting the database.
        let entity_state = self.get_entity_state_assumed(entity, STATE_NEW);

        match entity_state {
            STATE_MANAGED => {
                // Nothing to do, except if policy is "deferred explicit"
                if class.read().is_change_tracking_deferred_explicit() {
                    self.schedule_for_dirty_check(entity)?;
                }
            }
            STATE_NEW => self.persist_new(&class, entity)?,
            STATE_REMOVED => {
                // Entity becomes managed again
                self.st().entity_deletions.remove(&oid);
                self.add_to_identity_map(entity)?;
                self.st().entity_states.insert(oid, STATE_MANAGED);
                if class.read().is_change_tracking_deferred_explicit() {
                    self.schedule_for_dirty_check(entity)?;
                }
            }
            STATE_DETACHED => {
                // Can actually not happen right now since we assume
                // STATE_NEW.
                return Err(exception::detached_entity_cannot(entity, "persisted"));
            }
            other => return Err(unexpected_state(other, entity)),
        }

        self.cascade_persist(entity, visited)
    }

    /// Deletes an entity as part of the current unit of work.
    pub async fn remove(&self, entity: &Object) -> Result<(), Thrown> {
        let mut visited = HashSet::new();
        self.do_remove(entity, &mut visited).await
    }

    /// Deletes an entity as part of the current unit of work.
    ///
    /// This method is internally called during `delete()` cascades as it
    /// tracks the already visited entities to prevent infinite
    /// recursions.
    fn do_remove<'a>(&'a self, entity: &'a Object, visited: &'a mut HashSet<usize>) -> Fut<'a, ()> {
        Box::pin(async move {
            let oid = object_id(entity);
            if !visited.insert(oid) {
                return Ok(()); // Prevent infinite recursion
            }

            // Cascade first, because scheduleForDelete() removes the
            // entity from the identity map, which can cause problems when
            // a lazy proxy has to be initialized for the cascade
            // operation.
            self.cascade_remove(entity, visited).await?;

            let em = self.em()?;
            let class = em.get_class_metadata(entity.class())?;
            let entity_state = self.get_entity_state(entity).await?;

            match entity_state {
                STATE_NEW | STATE_REMOVED => {
                    // nothing to do
                }
                STATE_MANAGED => {
                    let invoke = self.subscribed(&em, &class, events::PRE_REMOVE);
                    if invoke != ListenersInvoker::INVOKE_NONE {
                        self.invoke_lifecycle(
                            &em,
                            &class,
                            events::PRE_REMOVE,
                            event::PRE_REMOVE_EVENT_ARGS,
                            entity,
                            invoke,
                        )?;
                    }
                    self.schedule_for_delete(entity)?;
                }
                STATE_DETACHED => {
                    return Err(exception::detached_entity_cannot(entity, "removed"));
                }
                other => return Err(unexpected_state(other, entity)),
            }
            Ok(())
        })
    }

    /// Detaches an entity from the persistence management. Its
    /// persistence will no longer be managed by Doctrine.
    pub fn detach(&self, entity: &Object) -> Result<(), Thrown> {
        let mut visited = HashSet::new();
        self.do_detach(entity, &mut visited, false)
    }

    /// Executes a detach operation on the given entity.
    fn do_detach(
        &self,
        entity: &Object,
        visited: &mut HashSet<usize>,
        no_cascade: bool,
    ) -> Result<(), Thrown> {
        let oid = object_id(entity);
        if !visited.insert(oid) {
            return Ok(()); // Prevent infinite recursion
        }

        match self.get_entity_state_assumed(entity, STATE_DETACHED) {
            STATE_MANAGED => {
                if self.is_in_identity_map(entity)? {
                    self.remove_from_identity_map(entity)?;
                }
                let mut st = self.st();
                st.entity_insertions.remove(&oid);
                st.entity_updates.remove(&oid);
                st.entity_deletions.remove(&oid);
                st.entity_identifiers.remove(&oid);
                st.entity_states.remove(&oid);
                st.original_entity_data.remove(&oid);
            }
            STATE_NEW | STATE_DETACHED => return Ok(()),
            _ => {}
        }

        if !no_cascade {
            self.cascade_detach(entity, visited)?;
        }
        Ok(())
    }

    /// Refreshes the state of the given entity from the database,
    /// overwriting any local, unpersisted changes.
    pub async fn refresh(
        &self,
        entity: &Object,
        lock_mode: Option<LockMode>,
    ) -> Result<(), Thrown> {
        let mut visited = HashSet::new();
        self.do_refresh(entity, &mut visited, lock_mode).await
    }

    /// Executes a refresh operation on an entity.
    fn do_refresh<'a>(
        &'a self,
        entity: &'a Object,
        visited: &'a mut HashSet<usize>,
        lock_mode: Option<LockMode>,
    ) -> Fut<'a, ()> {
        Box::pin(async move {
            if matches!(
                lock_mode,
                Some(LockMode::PessimisticRead | LockMode::PessimisticWrite)
            ) && !self.em()?.get_connection().is_transaction_active()
            {
                return Err(exception::transaction_required());
            }
            let oid = object_id(entity);
            if !visited.insert(oid) {
                return Ok(()); // Prevent infinite recursion
            }
            let class = self.em()?.get_class_metadata(entity.class())?;
            if self.get_entity_state(entity).await? != STATE_MANAGED {
                return Err(exception::entity_not_managed(entity));
            }
            self.cascade_refresh(entity, visited, lock_mode).await?;

            let id = self
                .st()
                .entity_identifiers
                .get(&oid)
                .cloned()
                .unwrap_or_default();
            let fields = class.read().get_identifier_field_names().to_vec();
            let mut criteria = PhpArray::new();
            for (field, value) in fields.iter().zip(id.values()) {
                criteria.insert(field.as_str(), value.clone());
            }
            let name = class.read().name.clone();
            self.get_entity_persister(&name)?
                .refresh(&criteria, entity, lock_mode)
                .await
        })
    }

    async fn cascade_refresh(
        &self,
        entity: &Object,
        visited: &mut HashSet<usize>,
        lock_mode: Option<LockMode>,
    ) -> Result<(), Thrown> {
        let class = self.em()?.get_class_metadata(entity.class())?;
        let associations: Vec<AssociationMapping> = class
            .read()
            .association_mappings
            .values()
            .filter(|a| a.is_cascade_refresh())
            .cloned()
            .collect();
        for assoc in associations {
            let related = class.read().get_field_value(entity, &assoc.field_name)?;
            if let Some(entities) = related_entities(&related) {
                for related_entity in entities {
                    if let Value::Object(o) = related_entity {
                        self.do_refresh(&o, visited, lock_mode).await?;
                    }
                }
            } else if let Value::Object(o) = related {
                self.do_refresh(&o, visited, lock_mode).await?;
            }
        }
        Ok(())
    }

    fn cascade_detach(&self, entity: &Object, visited: &mut HashSet<usize>) -> Result<(), Thrown> {
        let class = self.em()?.get_class_metadata(entity.class())?;
        let associations: Vec<AssociationMapping> = class
            .read()
            .association_mappings
            .values()
            .filter(|a| a.is_cascade_detach())
            .cloned()
            .collect();
        for assoc in associations {
            let related = class.read().get_field_value(entity, &assoc.field_name)?;
            if let Some(entities) = related_entities(&related) {
                for related_entity in entities {
                    if let Value::Object(o) = related_entity {
                        self.do_detach(&o, visited, false)?;
                    }
                }
            } else if let Value::Object(o) = related {
                self.do_detach(&o, visited, false)?;
            }
        }
        Ok(())
    }

    /// Cascades the save operation to associated entities.
    fn cascade_persist(&self, entity: &Object, visited: &mut HashSet<usize>) -> Result<(), Thrown> {
        if self.is_uninitialized_object(&Value::Object(entity.clone())) {
            return Ok(());
        }
        let em = self.em()?;
        let class = em.get_class_metadata(entity.class())?;
        let associations: Vec<AssociationMapping> = class
            .read()
            .association_mappings
            .values()
            .filter(|a| a.is_cascade_persist())
            .cloned()
            .collect();
        for assoc in associations {
            let related = class.read().get_field_value(entity, &assoc.field_name)?;
            if let Some(entities) = related_entities(&related) {
                if !assoc.is_to_many() {
                    let target = em.get_class_metadata(&assoc.target_entity)?;
                    let name = target.read().name.clone();
                    return Err(exception::invalid_association(
                        &name,
                        &assoc,
                        &related.debug_type(),
                    ));
                }
                for related_entity in entities {
                    match related_entity {
                        Value::Object(o) => self.do_persist(&o, visited)?,
                        other => {
                            return Err(Thrown::new(
                                "TypeError",
                                format!(
                                    "Doctrine\\ORM\\UnitOfWork::doPersist(): Argument #1 ($entity) must be of type object, {} given",
                                    other.debug_type()
                                ),
                                0,
                            ));
                        }
                    }
                }
            } else if !related.is_null() {
                let Value::Object(o) = &related else {
                    let target = em.get_class_metadata(&assoc.target_entity)?;
                    let name = target.read().name.clone();
                    return Err(exception::invalid_association(
                        &name,
                        &assoc,
                        &related.debug_type(),
                    ));
                };
                if !o.instance_of(&assoc.target_entity) {
                    let target = em.get_class_metadata(&assoc.target_entity)?;
                    let name = target.read().name.clone();
                    return Err(exception::invalid_association(
                        &name,
                        &assoc,
                        &related.debug_type(),
                    ));
                }
                self.do_persist(o, visited)?;
            }
        }
        Ok(())
    }

    /// Cascades the delete operation to associated entities.
    async fn cascade_remove(
        &self,
        entity: &Object,
        visited: &mut HashSet<usize>,
    ) -> Result<(), Thrown> {
        let class = self.em()?.get_class_metadata(entity.class())?;
        let associations: Vec<AssociationMapping> = class
            .read()
            .association_mappings
            .values()
            .filter(|a| a.is_cascade_remove())
            .cloned()
            .collect();
        if !associations.is_empty() {
            self.initialize_object(entity).await?;
        }

        let mut entities_to_cascade: Vec<Object> = Vec::new();
        for assoc in associations {
            let related = class.read().get_field_value(entity, &assoc.field_name)?;
            if let Some(pc) = PersistentCollection::from_value(&related) {
                // If its a PersistentCollection initialization is intended! No unwrap!
                for related_entity in pc.to_array().await?.values() {
                    if let Value::Object(o) = related_entity {
                        entities_to_cascade.push(o.clone());
                    }
                }
            } else if let Some(entities) = related_entities(&related) {
                for related_entity in entities {
                    if let Value::Object(o) = related_entity {
                        entities_to_cascade.push(o);
                    }
                }
            } else if let Value::Object(o) = related {
                entities_to_cascade.push(o);
            }
        }

        for related_entity in entities_to_cascade {
            self.do_remove(&related_entity, visited).await?;
        }
        Ok(())
    }

    /// Clears the `UnitOfWork`.
    pub fn clear(&self) -> Result<(), Thrown> {
        {
            let mut st = self.st();
            let deferred = std::mem::take(&mut st.deferred_post_load_invocations);
            *st = State::default();
            st.deferred_post_load_invocations = deferred;
        }
        self.listeners_invoker.clear();
        let em = self.em()?;
        self.dispatch(&em, events::ON_CLEAR, event::ON_CLEAR_EVENT_ARGS)
    }

    /// INTERNAL: Schedules an orphaned entity for removal. The `remove()`
    /// operation will be invoked on that entity at the beginning of the
    /// next commit of this `UnitOfWork`.
    pub fn schedule_orphan_removal(&self, entity: &Object) {
        self.st()
            .orphan_removals
            .insert(object_id(entity), entity.clone());
    }

    /// INTERNAL: Cancels a previously scheduled orphan removal.
    pub fn cancel_orphan_removal(&self, entity: &Object) {
        self.st().orphan_removals.remove(&object_id(entity));
    }

    /// INTERNAL: Schedules a complete collection for removal when this
    /// `UnitOfWork` commits.
    pub fn schedule_collection_deletion(&self, coll: &Arc<PersistentCollection>) {
        let coid = coll.oid();
        let mut st = self.st();
        // TODO: if $coll is already scheduled for recreation ... what to do?
        // Just remove $coll from the scheduled recreations?
        st.collection_updates.remove(&coid);
        st.collection_deletions.insert(coid, coll.clone());
    }

    pub fn is_collection_scheduled_for_deletion(&self, coll: &Arc<PersistentCollection>) -> bool {
        self.st().collection_deletions.contains(&coll.oid())
    }

    /// INTERNAL: Creates an entity. Used for reconstitution of persistent
    /// entities.
    ///
    /// Internal note: Highly performance-sensitive method.
    pub fn create_entity<'a>(
        &'a self,
        class_name: &'a str,
        data: PhpArray,
        hints: &'a mut Hints,
    ) -> Fut<'a, Object> {
        Box::pin(async move { self.do_create_entity(class_name, data, hints).await })
    }

    #[allow(clippy::too_many_lines, reason = "createEntity(), in its order")]
    async fn do_create_entity(
        &self,
        class_name: &str,
        data: PhpArray,
        hints: &mut Hints,
    ) -> Result<Object, Thrown> {
        let em = self.em()?;
        let class = em.get_class_metadata(class_name)?;

        let id = flatten_identifier(&em, &class.read(), &data)?;
        let id_hash = Self::get_id_hash_by_identifier(&id);
        let root = class.read().root_entity_name.clone();

        let existing = self.get_by_id_hash(&id_hash, &root);
        let entity: Object;
        let oid: usize;
        if let Some(found) = existing {
            let mut found = found;
            if hints.refresh
                && let Some(unmanaged_proxy) = hints.refresh_entity.clone()
                && !same(&unmanaged_proxy, &found)
                && self.is_identifier_equals(&unmanaged_proxy, &found)?
            {
                // DDC-1238 - we have a managed instance, but it isn't the
                // provided one. Therefore we clear its identifier. Also,
                // we must re-fetch metadata since the entity may be of
                // an inherited type.
                found = unmanaged_proxy;
            }
            entity = found;
            oid = object_id(&entity);

            if self.is_uninitialized_object(&Value::Object(entity.clone())) {
                if let Some(ghost) = downcast::<EntityObject>(&entity) {
                    ghost.set_lazy(false);
                }
            } else if !hints.refresh
                || hints
                    .refresh_entity
                    .as_ref()
                    .is_some_and(|r| !same(r, &entity))
            {
                return Ok(entity);
            }

            self.st().original_entity_data.insert(oid, data.clone());
        } else {
            entity = class.read().new_instance()?;
            oid = object_id(&entity);
            self.register_managed(&entity, id.clone(), data.clone())?;
            if hints.read_only {
                self.st().read_only_objects.insert(oid);
            }
        }

        {
            let info = class.read();
            for (field, value) in data.iter() {
                let field = crate::utility::key_text(field);
                if info.field_mappings.contains_key(&field) {
                    info.set_field_value(&entity, &field, value.clone())?;
                }
            }
        }

        // Loading the entity right here, if its in the eager loading map
        // get rid of it there just to be sure
        {
            let mut st = self.st();
            let mut emptied = false;
            if let Some(map) = st.eager_loading_entities.get_mut(&root) {
                map.remove(&id_hash);
                emptied = map.is_empty();
            }
            if emptied {
                st.eager_loading_entities.remove(&root);
            }
        }

        let associations: Vec<(String, AssociationMapping)> = class
            .read()
            .association_mappings
            .iter()
            .map(|(k, v)| (k.to_owned(), v.clone()))
            .collect();
        let class_name_of = class.read().name.clone();

        for (field, assoc) in associations {
            // Check if the association is not among the fetch-joined
            // associations already.
            if hints.is_fetched(&field) {
                continue;
            }
            let fetch_mode = hints.fetch_mode(&class_name_of, &field, assoc.fetch);
            let target_class = em.get_class_metadata(&assoc.target_entity)?;
            let field_key = Key::from(field.as_str());

            if assoc.is_to_one() {
                // Inverse side of x-to-one can never be lazy
                if !assoc.is_owning_side() {
                    if let Some(Value::Object(related)) = data.get(&field_key).cloned()
                        && self.st().entity_states.contains_key(&object_id(&related))
                    {
                        self.st()
                            .original_entity_data
                            .entry(oid)
                            .or_default()
                            .insert(field.as_str(), Value::Object(related.clone()));
                        class.read().set_field_value(
                            &entity,
                            &field,
                            Value::Object(related.clone()),
                        )?;
                        if let Some(mapped_by) = &assoc.mapped_by {
                            target_class.read().set_field_value(
                                &related,
                                mapped_by,
                                Value::Object(entity.clone()),
                            )?;
                        }
                        continue;
                    }
                    // use the given entity association
                    let loaded = self
                        .get_entity_persister(&assoc.target_entity)?
                        .load_one_to_one_entity(&assoc, &entity, None)
                        .await?;
                    class.read().set_field_value(
                        &entity,
                        &field,
                        loaded.map_or(Value::Null, Value::Object),
                    )?;
                    continue;
                }

                if let Some(Value::Object(related)) = data.get(&field_key).cloned()
                    && self.st().entity_states.contains_key(&object_id(&related))
                {
                    // use the entity association
                    class.read().set_field_value(
                        &entity,
                        &field,
                        Value::Object(related.clone()),
                    )?;
                    self.st()
                        .original_entity_data
                        .entry(oid)
                        .or_default()
                        .insert(field.as_str(), Value::Object(related));
                    continue;
                }

                let mut associated_id = PhpArray::new();
                {
                    let target = target_class.read();
                    for (target_column, src_column) in assoc.target_to_source_key_columns.iter() {
                        let target_column = crate::utility::key_text(target_column);
                        let src_column = to_text(src_column);
                        let join_column_value = data
                            .get(&src_column.as_str().into())
                            .cloned()
                            .unwrap_or(Value::Null);
                        if !join_column_value.is_null() {
                            let join_column_value =
                                crate::utility::identifier_flattener_backed(join_column_value);
                            if target.contains_foreign_identifier {
                                associated_id.insert(
                                    target.get_field_for_column(&target_column)?.as_str(),
                                    join_column_value,
                                );
                            } else {
                                let field_name = target
                                    .field_names
                                    .get(&target_column)
                                    .cloned()
                                    .unwrap_or_default();
                                associated_id.insert(field_name.as_str(), join_column_value);
                            }
                        } else if target
                            .identifier
                            .contains(&target.get_field_for_column(&target_column)?)
                        {
                            // the missing key is part of target's entity
                            // primary key
                            associated_id = PhpArray::new();
                            break;
                        }
                    }
                }

                if associated_id.is_empty() {
                    // Foreign key is NULL
                    class.read().set_field_value(&entity, &field, Value::Null)?;
                    self.st()
                        .original_entity_data
                        .entry(oid)
                        .or_default()
                        .insert(field.as_str(), Value::Null);
                    continue;
                }

                // Foreign key is set
                // Check identity map first
                // FIXME: Can break easily with composite keys if join
                // column values are in wrong order. The correct order is
                // the one in ClassMetadata#identifier.
                let related_id_hash = Self::get_id_hash_by_identifier(&associated_id);
                let (target_root, target_subclasses, target_composite) = {
                    let target = target_class.read();
                    (
                        target.root_entity_name.clone(),
                        !target.sub_classes.is_empty(),
                        target.is_identifier_composite,
                    )
                };

                let new_value: Option<Object> =
                    if let Some(found) = self.get_by_id_hash(&related_id_hash, &target_root) {
                        // If this is an uninitialized proxy, we are deferring
                        // eager loads, this association is marked as eager
                        // fetch, and its an uninitialized proxy, we add it to
                        // the eager loading map.
                        if fetch_mode == Some(FETCH_EAGER)
                            && hints.defer_eager_load
                            && !target_composite
                            && self.is_uninitialized_object(&Value::Object(found.clone()))
                        {
                            let first = associated_id
                                .values()
                                .next()
                                .cloned()
                                .unwrap_or(Value::Null);
                            self.st()
                                .eager_loading_entities
                                .get_or_default(target_root.clone())
                                .insert(related_id_hash.clone(), first);
                        }
                        Some(found)
                    } else if target_subclasses {
                        // If it might be a subtype, it can not be lazy. There
                        // isn't even a way to use deferred eager loading here.
                        self.get_entity_persister(&assoc.target_entity)?
                            .load_one_to_one_entity(&assoc, &entity, Some(&associated_id))
                            .await?
                    } else {
                        let normalized_associated_id = self
                            .normalize_identifier(&target_class, &associated_id)
                            .await?;
                        if fetch_mode != Some(FETCH_EAGER) {
                            // TODO: Is there a faster approach?
                            let proxy =
                                em.get_proxy(&assoc.target_entity, &normalized_associated_id)?;
                            self.register_managed_proxy(&proxy, associated_id.clone())?;
                            Some(proxy)
                        } else if hints.defer_eager_load && !target_composite {
                            // Deferred eager load only works for single
                            // identifier classes
                            let first = normalized_associated_id
                                .values()
                                .next()
                                .cloned()
                                .unwrap_or(Value::Null);
                            self.st()
                                .eager_loading_entities
                                .get_or_default(target_root.clone())
                                .insert(related_id_hash.clone(), first);
                            let proxy =
                                em.get_proxy(&assoc.target_entity, &normalized_associated_id)?;
                            self.register_managed_proxy(&proxy, associated_id.clone())?;
                            Some(proxy)
                        } else {
                            // TODO: This is very imperformant, ignore it?
                            em.find(&assoc.target_entity, Value::Array(normalized_associated_id))
                                .await?
                        }
                    };

                let new_value_v = new_value.clone().map_or(Value::Null, Value::Object);
                self.st()
                    .original_entity_data
                    .entry(oid)
                    .or_default()
                    .insert(field.as_str(), new_value_v.clone());
                class.read().set_field_value(&entity, &field, new_value_v)?;

                if let (Some(inversed_by), true, Some(new_value)) =
                    (&assoc.inversed_by, assoc.is_one_to_one(), &new_value)
                {
                    let inverse_field = target_class
                        .read()
                        .association_mappings
                        .get(inversed_by)
                        .map(|a| a.field_name.clone());
                    if let Some(inverse_field) = inverse_field {
                        target_class.read().set_field_value(
                            new_value,
                            &inverse_field,
                            Value::Object(entity.clone()),
                        )?;
                    }
                }
                continue;
            }

            // Ignore if its a cached collection
            if let Some(Value::Object(_)) = data.get(&field_key)
                && let Some(pc) = data
                    .get(&field_key)
                    .and_then(PersistentCollection::from_value)
            {
                pc.set_owner(&entity, &assoc);
                class.read().set_field_value(&entity, &field, pc.value())?;
                self.st()
                    .original_entity_data
                    .entry(oid)
                    .or_default()
                    .insert(field.as_str(), pc.value());
                continue;
            }

            // Inject collection
            let p_coll = PersistentCollection::new(
                Some(&em),
                Some(target_class.clone()),
                ArrayCollection::new(PhpArray::new()),
            );
            p_coll.set_owner(&entity, &assoc);
            p_coll.set_initialized(false);
            class
                .read()
                .set_field_value(&entity, &field, p_coll.value())?;

            if fetch_mode == Some(FETCH_EAGER) {
                let mapped_by_join_columns = assoc
                    .mapped_by
                    .as_ref()
                    .and_then(|m| {
                        target_class
                            .read()
                            .association_mappings
                            .get(m)
                            .map(|a| a.join_columns.len())
                    })
                    .unwrap_or(0);
                if assoc.is_one_to_many() && mapped_by_join_columns <= 1 && !assoc.is_indexed() {
                    self.schedule_collection_for_batch_loading(&p_coll, &class)?;
                } else {
                    self.load_collection(&p_coll).await?;
                    p_coll.take_snapshot();
                }
            }

            self.st()
                .original_entity_data
                .entry(oid)
                .or_default()
                .insert(field.as_str(), p_coll.value());
        }

        // defer invoking of postLoad event to hydration complete step
        self.defer_post_load_invoking(&class, &entity);
        Ok(entity)
    }

    /// `HydrationCompleteHandler::deferPostLoadInvoking()`.
    fn defer_post_load_invoking(&self, class: &Arc<ClassMetadata>, entity: &Object) {
        let Ok(em) = self.em() else { return };
        let invoke = self.subscribed(&em, class, events::POST_LOAD);
        if invoke == ListenersInvoker::INVOKE_NONE {
            return;
        }
        self.st()
            .deferred_post_load_invocations
            .push((class.clone(), invoke, entity.clone()));
    }

    /// INTERNAL: To be called by the hydrators when hydration is complete
    /// (`HydrationCompleteHandler::hydrationComplete()`).
    pub fn hydration_complete(&self) -> Result<(), Thrown> {
        let to_invoke = std::mem::take(&mut self.st().deferred_post_load_invocations);
        let em = self.em()?;
        for (class, invoke, entity) in to_invoke {
            self.invoke_lifecycle(
                &em,
                &class,
                events::POST_LOAD,
                event::POST_LOAD_EVENT_ARGS,
                &entity,
                invoke,
            )?;
        }
        Ok(())
    }

    /// Triggers the deferred eager loads.
    pub async fn trigger_eager_loads(&self) -> Result<(), Thrown> {
        let (entities, collections) = {
            let mut st = self.st();
            if st.eager_loading_entities.is_empty() && st.eager_loading_collections.is_empty() {
                return Ok(());
            }
            (
                std::mem::take(&mut st.eager_loading_entities),
                std::mem::take(&mut st.eager_loading_collections),
            )
        };
        let em = self.em()?;
        let batch_size = usize::try_from(em.get_configuration().get_eager_fetch_batch_size())
            .unwrap_or(1)
            .max(1);

        for (entity_name, ids) in entities.iter() {
            if ids.is_empty() {
                continue;
            }
            let class = em.get_class_metadata(entity_name)?;
            let id_field = class.read().identifier.first().cloned().unwrap_or_default();
            let ids: Vec<Value> = ids.values().cloned().collect();
            for batch in ids.chunks(batch_size) {
                let mut criteria = PhpArray::new();
                criteria.insert(
                    id_field.as_str(),
                    Value::Array(PhpArray::list(batch.iter().cloned())),
                );
                self.get_entity_persister(entity_name)?
                    .load_all(&criteria, None, None, None)
                    .await?;
            }
        }

        for (_, (mapping, items)) in collections.iter() {
            self.eager_load_collections(items, mapping).await?;
        }
        Ok(())
    }

    /// Load all data into the given collections, according to the
    /// specified mapping
    async fn eager_load_collections(
        &self,
        collections: &Ordered<String, Arc<PersistentCollection>>,
        mapping: &AssociationMapping,
    ) -> Result<(), Thrown> {
        let em = self.em()?;
        let target_entity = mapping.target_entity.clone();
        let class = em.get_class_metadata(&mapping.source_entity)?;
        let mapped_by = mapping.mapped_by.clone().unwrap_or_default();
        let batch_size = usize::try_from(em.get_configuration().get_eager_fetch_batch_size())
            .unwrap_or(1)
            .max(1);
        let all: Vec<(String, Arc<PersistentCollection>)> = collections
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for batch in all.chunks(batch_size) {
            let entities: Vec<Value> = batch
                .iter()
                .filter_map(|(_, c)| c.get_owner().map(Value::Object))
                .collect();
            let mut criteria = PhpArray::new();
            criteria.insert(mapped_by.as_str(), Value::Array(PhpArray::list(entities)));
            let found = self
                .get_entity_persister(&target_entity)?
                .load_all(&criteria, Some(&mapping.order_by), None, None)
                .await?;

            let target_class = em.get_class_metadata(&target_entity)?;
            for target_value in found {
                let source_entity = target_class
                    .read()
                    .get_field_value(&target_value, &mapped_by)?;
                let id_hash = if let Value::Object(source) = &source_entity {
                    let info = class.read();
                    let values = info.get_identifier_values(source)?;
                    crate::utility::implode_values(&flatten_identifier(&em, &info, &values)?)
                } else {
                    let data = self.get_original_entity_data(&target_value);
                    let join_columns = target_class
                        .read()
                        .association_mappings
                        .get(&mapped_by)
                        .map(|a| a.join_columns.clone())
                        .unwrap_or_default();
                    join_columns
                        .iter()
                        .map(|jc| {
                            to_text(data.get(&jc.name.as_str().into()).unwrap_or(&Value::Null))
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                let Some((_, coll)) = batch.iter().find(|(k, _)| *k == id_hash) else {
                    continue;
                };
                if let Some(index_by) = &mapping.index_by {
                    let key =
                        crate::internal::hydration::index_by_value(&em, &target_value, index_by)?;
                    let key = Key::from_value(&key).unwrap_or(Key::Int(0));
                    coll.hydrate_set(key, Value::Object(target_value))?;
                } else {
                    coll.add(Value::Object(target_value))?;
                }
            }
        }

        for (_, association) in &all {
            association.set_initialized(true);
            association.take_snapshot();
        }
        Ok(())
    }

    /// Initializes (loads) an uninitialized persistent collection of an
    /// entity.
    pub async fn load_collection(
        &self,
        collection: &Arc<PersistentCollection>,
    ) -> Result<(), Thrown> {
        let assoc = collection.get_mapping()?;
        let persister = self.get_entity_persister(&assoc.target_entity)?;
        let owner = collection
            .get_owner()
            .ok_or_else(|| Thrown::new("TypeError", "The collection has no owner", 0))?;
        if assoc.is_one_to_many() {
            persister
                .load_one_to_many_collection(&assoc, &owner, collection)
                .await?;
        } else if assoc.is_many_to_many() {
            persister
                .load_many_to_many_collection(&assoc, &owner, collection)
                .await?;
        }
        collection.set_initialized(true);
        Ok(())
    }

    fn schedule_collection_for_batch_loading(
        &self,
        collection: &Arc<PersistentCollection>,
        source_class: &Arc<ClassMetadata>,
    ) -> Result<(), Thrown> {
        let em = self.em()?;
        let mapping = collection.get_mapping()?;
        let name = format!("{}#{}", mapping.source_entity, mapping.field_name);
        let owner = collection
            .get_owner()
            .ok_or_else(|| Thrown::new("AssertionError", "assert($owner !== null)", 0))?;
        let id_hash = {
            let info = source_class.read();
            let values = info.get_identifier_values(&owner)?;
            crate::utility::implode_values(&flatten_identifier(&em, &info, &values)?)
        };
        let mut st = self.st();
        if !st.eager_loading_collections.contains(&name) {
            st.eager_loading_collections
                .insert(name.clone(), (mapping, Ordered::default()));
        }
        if let Some((_, items)) = st.eager_loading_collections.get_mut(&name) {
            items.insert(id_hash, collection.clone());
        }
        Ok(())
    }

    /// Gets the identity map of the `UnitOfWork`: root class name => id hash
    /// => entity.
    pub fn get_identity_map(&self) -> Vec<(String, Vec<(String, Object)>)> {
        self.st()
            .identity_map
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    v.iter().map(|(h, e)| (h.clone(), e.clone())).collect(),
                )
            })
            .collect()
    }

    /// Gets the original data of an entity. The original data is the data
    /// that was present at the time the entity was reconstituted from the
    /// database.
    pub fn get_original_entity_data(&self, entity: &Object) -> PhpArray {
        self.st()
            .original_entity_data
            .get(&object_id(entity))
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_original_entity_data(&self, entity: &Object, data: PhpArray) {
        self.st()
            .original_entity_data
            .insert(object_id(entity), data);
    }

    /// INTERNAL: Sets a property value of the original data array of an
    /// entity.
    pub fn set_original_entity_property(&self, oid: usize, property: &str, value: Value) {
        self.st()
            .original_entity_data
            .entry(oid)
            .or_default()
            .insert(property, value);
    }

    /// Gets the identifier of an entity. The returned value is always an
    /// array of identifier values. If the entity has a composite
    /// identifier then the identifier values are in the same order as the
    /// identifier field names as returned by
    /// `ClassMetadata#getIdentifierFieldNames()`.
    pub fn get_entity_identifier(&self, entity: &Object) -> Result<PhpArray, Thrown> {
        self.st()
            .entity_identifiers
            .get(&object_id(entity))
            .cloned()
            .ok_or_else(|| {
                exception::no_identifier_found(&Value::Object(entity.clone()).debug_type())
            })
    }

    /// Processes an entity instance to extract their identifier values.
    pub fn get_single_identifier_value(&self, entity: &Object) -> Result<Value, Thrown> {
        let class = self.em()?.get_class_metadata(entity.class())?;
        if class.read().is_identifier_composite {
            return Err(exception::invalid_composite_identifier());
        }
        let values = if self.is_in_identity_map(entity)? {
            self.get_entity_identifier(entity)?
        } else {
            class.read().get_identifier_values(entity)?
        };
        let first = class.read().identifier.first().cloned().unwrap_or_default();
        Ok(values
            .get(&first.as_str().into())
            .cloned()
            .unwrap_or(Value::Null))
    }

    /// Tries to find an entity with the given identifier in the identity
    /// map of this `UnitOfWork`.
    pub fn try_get_by_id(&self, id: &PhpArray, root_class_name: &str) -> Option<Object> {
        let id_hash = Self::get_id_hash_by_identifier(id);
        self.get_by_id_hash(&id_hash, root_class_name)
    }

    /// Schedules an entity for dirty-checking at commit-time.
    pub fn schedule_for_dirty_check(&self, entity: &Object) -> Result<(), Thrown> {
        let root = self
            .em()?
            .get_class_metadata(entity.class())?
            .read()
            .root_entity_name
            .clone();
        self.st()
            .scheduled_for_synchronization
            .get_or_default(root)
            .insert(object_id(entity), entity.clone());
        Ok(())
    }

    /// Checks whether the `UnitOfWork` has any pending insertions.
    pub fn has_pending_insertions(&self) -> bool {
        !self.st().entity_insertions.is_empty()
    }

    /// Calculates the size of the `UnitOfWork`. The size of the `UnitOfWork`
    /// is the number of entities in the identity map.
    pub fn size(&self) -> usize {
        self.st().identity_map.values().map(Ordered::len).sum()
    }

    /// Gets the `EntityPersister` for an Entity.
    pub fn get_entity_persister(
        &self,
        entity_name: &str,
    ) -> Result<Arc<BasicEntityPersister>, Thrown> {
        if let Some(persister) = self
            .persisters
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(entity_name)
        {
            return Ok(persister.clone());
        }
        let em = self.em()?;
        let class = em.get_class_metadata(entity_name)?;
        let persister = Arc::new(BasicEntityPersister::new(&em, class));
        self.persisters
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(entity_name.to_owned(), persister.clone());
        Ok(persister)
    }

    /// INTERNAL: Registers an entity as managed.
    pub fn register_managed(
        &self,
        entity: &Object,
        id: PhpArray,
        data: PhpArray,
    ) -> Result<(), Thrown> {
        let oid = object_id(entity);
        {
            let mut st = self.st();
            st.entity_identifiers.insert(oid, id);
            st.entity_states.insert(oid, STATE_MANAGED);
            st.original_entity_data.insert(oid, data);
        }
        self.add_to_identity_map(entity)?;
        Ok(())
    }

    /// INTERNAL: Registers a proxy as managed (with no original data).
    pub fn register_managed_proxy(&self, entity: &Object, id: PhpArray) -> Result<(), Thrown> {
        self.register_managed(entity, id, PhpArray::new())
    }

    /// Gets the currently scheduled entity insertions in this `UnitOfWork`.
    pub fn get_scheduled_entity_insertions(&self) -> Vec<Object> {
        self.st().entity_insertions.values().cloned().collect()
    }

    /// Gets the currently scheduled entity updates in this `UnitOfWork`.
    pub fn get_scheduled_entity_updates(&self) -> Vec<Object> {
        self.st().entity_updates.values().cloned().collect()
    }

    /// Gets the currently scheduled entity deletions in this `UnitOfWork`.
    pub fn get_scheduled_entity_deletions(&self) -> Vec<Object> {
        self.st().entity_deletions.values().cloned().collect()
    }

    /// Gets the currently scheduled complete collection deletions
    pub fn get_scheduled_collection_deletions(&self) -> Vec<Arc<PersistentCollection>> {
        self.st().collection_deletions.values().cloned().collect()
    }

    /// Gets the currently scheduled collection inserts, updates and
    /// deletes.
    pub fn get_scheduled_collection_updates(&self) -> Vec<Arc<PersistentCollection>> {
        self.st().collection_updates.values().cloned().collect()
    }

    /// Acquire a lock on the given entity.
    ///
    /// A pessimistic lock runs no query: the `SQLite` platform has no
    /// lock clause (PHP's `SELECT 1` query reads nothing).
    pub async fn lock(
        &self,
        entity: &Object,
        lock_mode: LockMode,
        lock_version: Option<&Value>,
    ) -> Result<(), Thrown> {
        if self.get_entity_state_or(entity, STATE_DETACHED) != STATE_MANAGED {
            return Err(exception::entity_not_managed(entity));
        }
        let em = self.em()?;
        let class = em.get_class_metadata(entity.class())?;
        match lock_mode {
            LockMode::Optimistic => {
                let (versioned, name, version_field) = {
                    let info = class.read();
                    (
                        info.is_versioned,
                        info.name.clone(),
                        info.version_field.clone(),
                    )
                };
                if !versioned {
                    return Err(exception::not_versioned(&name));
                }
                let Some(lock_version) = lock_version.filter(|v| !v.is_null()) else {
                    return Ok(());
                };
                self.initialize_object(entity).await?;
                let entity_version = class
                    .read()
                    .get_field_value(entity, &version_field.unwrap_or_default())?;
                if !php_value::compare::loose_equals(&entity_version, lock_version) {
                    return Err(exception::lock_failed_version_mismatch(
                        &crate::utility::to_text(lock_version),
                        &crate::utility::to_text(&entity_version),
                    ));
                }
            }
            LockMode::PessimisticRead | LockMode::PessimisticWrite => {
                if !em.get_connection().is_transaction_active() {
                    return Err(exception::transaction_required());
                }
            }
            LockMode::None => {}
        }
        Ok(())
    }

    /// Helper method to initialize a lazy loading proxy or persistent
    /// collection.
    pub async fn initialize_object(&self, obj: &Object) -> Result<(), Thrown> {
        if let Some(pc) = downcast::<PersistentCollection>(obj) {
            return pc.initialize().await;
        }
        if let Some(ghost) = downcast::<EntityObject>(obj)
            && ghost.is_uninitialized_lazy_object()
        {
            let em = self.em()?;
            let class = em.get_class_metadata(obj.class())?;
            let name = class.read().name.clone();
            let id = self
                .st()
                .entity_identifiers
                .get(&object_id(obj))
                .cloned()
                .unwrap_or_default();
            let identifier = self.normalize_identifier(&class, &id).await?;
            let found = self
                .get_entity_persister(&name)?
                .load_by_id(&identifier, Some(obj))
                .await?;
            if found.is_none() {
                return Err(exception::entity_not_found_from_class_name_and_identifier(
                    &name,
                    &crate::utility::string_pairs(&id),
                ));
            }
            ghost.set_lazy(false);
        }
        Ok(())
    }

    /// Helper method to show if an object is an uninitialized lazy object
    /// (a proxy).
    pub fn is_uninitialized_object(&self, obj: &Value) -> bool {
        let Value::Object(object) = obj else {
            return false;
        };
        downcast::<EntityObject>(object).is_some_and(|o| o.is_uninitialized_lazy_object())
    }

    /// Marks an entity as read-only so that it will not be considered for
    /// updates during `UnitOfWork#commit()`.
    ///
    /// This operation cannot be undone as some parts of the `UnitOfWork`
    /// now keep gathering information on the entity.
    pub fn mark_read_only(&self, object: &Object) -> Result<(), Thrown> {
        if !self.is_in_identity_map(object)? {
            return Err(exception::read_only_requires_managed_entity(object));
        }
        self.st().read_only_objects.insert(object_id(object));
        Ok(())
    }

    /// Is this entity read only?
    pub fn is_read_only(&self, object: &Object) -> bool {
        self.st().read_only_objects.contains(&object_id(object))
    }

    fn is_identifier_equals(&self, entity1: &Object, entity2: &Object) -> Result<bool, Thrown> {
        if same(entity1, entity2) {
            return Ok(true);
        }
        let em = self.em()?;
        let class = em.get_class_metadata(entity1.class())?;
        if !Arc::ptr_eq(&class, &em.get_class_metadata(entity2.class())?) {
            return Ok(false);
        }
        let identifier = |entity: &Object| -> Result<PhpArray, Thrown> {
            if let Some(id) = self
                .st()
                .entity_identifiers
                .get(&object_id(entity))
                .cloned()
            {
                return Ok(id);
            }
            let info = class.read();
            let values = info.get_identifier_values(entity)?;
            flatten_identifier(&em, &info, &values)
        };
        let id1 = identifier(entity1)?;
        let id2 = identifier(entity2)?;
        Ok(id1 == id2
            || Self::get_id_hash_by_identifier(&id1) == Self::get_id_hash_by_identifier(&id2))
    }

    fn assert_that_there_are_no_unintentionally_non_persisted_associations(
        &self,
    ) -> Result<(), Thrown> {
        let entities_needing_cascade_persist: Vec<(AssociationMapping, Object)> = {
            let mut st = self.st();
            let detected = std::mem::take(&mut st.non_cascaded_new_detected_entities);
            detected
                .iter()
                .filter(|(oid, _)| !st.entity_insertions.contains(oid))
                .map(|(_, v)| v.clone())
                .collect()
        };
        if !entities_needing_cascade_persist.is_empty() {
            return Err(exception::new_entities_found_through_relationships(
                &entities_needing_cascade_persist,
            ));
        }
        Ok(())
    }

    /// `convertSingleFieldIdentifierToPHPValue()`.
    fn convert_single_field_identifier_to_php_value(
        &self,
        class: &ClassMetadata,
        identifier_value: &Value,
    ) -> Result<Value, Thrown> {
        let em = self.em()?;
        let ty = {
            let info = class.read();
            let field = info.get_single_identifier_field_name()?;
            info.get_type_of_field(&field)
        };
        let Some(ty) = ty else {
            return Ok(identifier_value.clone());
        };
        let ty = php2rust_doctrine_dbal::types::get_type(&ty)
            .map_err(|e| Thrown::new(e.class(), e.message(), 0))?;
        ty.convert_to_php_value(identifier_value, em.get_platform())
            .map_err(|e| crate::exception::from_conversion(&e))
    }

    /// Given a flat identifier, this method will produce another
    /// identifier, with association identifiers replaced by references to
    /// the associated entities.
    fn normalize_identifier<'a>(
        &'a self,
        target_class: &'a ClassMetadata,
        flat_identifier: &'a PhpArray,
    ) -> Fut<'a, PhpArray> {
        Box::pin(self.do_normalize_identifier(target_class, flat_identifier))
    }

    async fn do_normalize_identifier(
        &self,
        target_class: &ClassMetadata,
        flat_identifier: &PhpArray,
    ) -> Result<PhpArray, Thrown> {
        let em = self.em()?;
        let mut normalized_associated_id = PhpArray::new();
        let fields = target_class.read().get_identifier_field_names().to_vec();
        for name in fields {
            let Some(value) = flat_identifier.get(&name.as_str().into()) else {
                continue;
            };
            let single = target_class.read().is_single_valued_association(&name);
            if !single {
                normalized_associated_id.insert(name.as_str(), value.clone());
                continue;
            }
            let target_id_class =
                em.get_class_metadata(&target_class.read().get_association_target_class(&name)?)?;
            let first = target_id_class
                .read()
                .identifier
                .first()
                .cloned()
                .unwrap_or_default();
            let mut inner = PhpArray::new();
            inner.insert(first.as_str(), value.clone());
            let inner = self.normalize_identifier(&target_id_class, &inner).await?;
            let target_name = target_id_class.read().get_name().to_owned();
            let reference = em.get_reference(&target_name, Value::Array(inner)).await?;
            normalized_associated_id
                .insert(name.as_str(), reference.map_or(Value::Null, Value::Object));
        }
        Ok(normalized_associated_id)
    }

    /// Assign a post-insert generated ID to an entity
    ///
    /// This is used by `EntityPersisters` after they inserted entities into
    /// the DB. It will place the assigned ID values in the entity's fields
    /// and start tracking the entity in the identity map.
    pub fn assign_post_insert_id(
        &self,
        entity: &Object,
        generated_id: &Value,
    ) -> Result<(), Thrown> {
        let class = self.em()?.get_class_metadata(entity.class())?;
        let id_field = class.read().get_single_identifier_field_name()?;
        let id_value = self.convert_single_field_identifier_to_php_value(&class, generated_id)?;
        let oid = object_id(entity);

        class
            .read()
            .set_field_value(entity, &id_field, id_value.clone())?;
        {
            let mut st = self.st();
            let mut identifier = PhpArray::new();
            identifier.insert(id_field.as_str(), id_value.clone());
            st.entity_identifiers.insert(oid, identifier);
            st.entity_states.insert(oid, STATE_MANAGED);
            st.original_entity_data
                .entry(oid)
                .or_default()
                .insert(id_field.as_str(), id_value);
        }
        self.add_to_identity_map(entity)?;
        Ok(())
    }
}

/// `[$old, $new]`.
fn pair(old: Value, new: Value) -> Value {
    Value::Array(PhpArray::list([old, new]))
}
