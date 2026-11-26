//! The `EntityManager` is the central access point to ORM functionality.
//!
//! It is a facade to all different ORM subsystems such as `UnitOfWork`,
//! Query Language and Repository API. The entity manager is shared
//! (`Arc`): [`EntityManager::create`] gives it, and the unit of work, the
//! persisters and the repositories hold it weakly.
//!
//! The DBAL is async, so the methods that reach the database are `async`.
//! The second level cache and the metadata cache are not ported. The platform is the `SQLite` platform,
//! the one database the DBAL port drives.
//!
//! Upstream: doctrine/orm src/EntityManager.php src/Proxy/ProxyFactory.php

use std::sync::{Arc, PoisonError, RwLock};

use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::platforms::{AbstractPlatform, SQLitePlatform};
use php2rust_doctrine_dbal::{Connection, LockMode};
use php2rust_doctrine_event_manager::EventManager;
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;

use crate::collection_object::downcast;
use crate::configuration::Configuration;
use crate::entity_repository::EntityRepository;
use crate::exception;
use crate::mapping::class_metadata_factory::{FactoryConfiguration, SharedPlatform};
use crate::mapping::{ClassMetadata, ClassMetadataFactory};
use crate::object::EntityObject;
use crate::repository::RepositoryFactory;
use crate::unit_of_work::{Hints, UnitOfWork};

/// `Doctrine\ORM\EntityManager`.
pub struct EntityManager {
    conn: Arc<Connection>,
    config: Configuration,
    metadata_factory: Arc<ClassMetadataFactory>,
    repository_factory: Arc<dyn RepositoryFactory>,
    unit_of_work: UnitOfWork,
    event_manager: RwLock<EventManager>,
    platform: SharedPlatform,
    closed: RwLock<bool>,
    /// The collection of query filters (`$filterCollection`).
    filter_collection: crate::query::FilterCollection,
    /// Whether `getFilters()` created the collection (`$filterCollection
    /// !== null`).
    filters_used: std::sync::atomic::AtomicBool,
}

impl std::fmt::Debug for EntityManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityManager")
            .field("closed", &self.is_closed())
            .finish_non_exhaustive()
    }
}

impl EntityManager {
    /// Creates a new `EntityManager` that operates on the given database
    /// connection and uses the given Configuration and `EventManager`
    /// implementations.
    pub fn create(
        conn: Arc<Connection>,
        config: Configuration,
        event_manager: Option<EventManager>,
    ) -> Result<Arc<Self>, Thrown> {
        let Some(driver) = config.get_metadata_driver_impl().cloned() else {
            return Err(exception::missing_mapping_driver_implementation());
        };
        let platform: SharedPlatform = Arc::new(SQLitePlatform::new());
        let metadata_factory = Arc::new(ClassMetadataFactory::new());
        let mut factory_config = FactoryConfiguration::new(driver);
        factory_config.naming_strategy = config.get_naming_strategy();
        factory_config.typed_field_mapper = config.get_typed_field_mapper();
        factory_config.quote_strategy = config.get_quote_strategy();
        factory_config.identity_generation_preferences =
            config.get_identity_generation_preferences().to_vec();
        factory_config.platform = platform.clone();
        metadata_factory.set_entity_manager(factory_config);
        let repository_factory = config.get_repository_factory();
        Ok(Arc::new_cyclic(|weak| Self {
            conn,
            config,
            metadata_factory,
            repository_factory,
            unit_of_work: UnitOfWork::new(weak.clone()),
            event_manager: RwLock::new(event_manager.unwrap_or_default()),
            platform,
            closed: RwLock::new(false),
            filter_collection: crate::query::FilterCollection::new(weak.clone()),
            filters_used: std::sync::atomic::AtomicBool::new(false),
        }))
    }

    /// Creates a new `Query` object.
    pub fn create_query(self: &Arc<Self>, dql: &str) -> crate::query::Query {
        let query = crate::query::Query::new(self);
        // `! empty($dql)`: "0" is empty too.
        if !dql.is_empty() && dql != "0" {
            query.set_dql(dql);
        }
        query
    }

    /// Creates a `NativeQuery` from some native SQL.
    pub fn create_native_query(
        self: &Arc<Self>,
        sql: &str,
        rsm: crate::query::ResultSetMapping,
    ) -> Result<crate::native_query::NativeQuery, Thrown> {
        let query = crate::native_query::NativeQuery::new(self);
        query.set_sql(sql);
        query.set_result_set_mapping(rsm)?;
        Ok(query)
    }

    /// Gets the enabled filters.
    pub fn get_filters(&self) -> &crate::query::FilterCollection {
        self.filters_used
            .store(true, std::sync::atomic::Ordering::Relaxed);
        &self.filter_collection
    }

    /// Checks whether the state of the filter collection is clean.
    pub fn is_filters_state_clean(&self) -> bool {
        !self.has_filters() || self.filter_collection.is_clean()
    }

    /// Checks whether the Entity Manager has filters.
    pub fn has_filters(&self) -> bool {
        self.filters_used.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gets the database connection object used by the `EntityManager`.
    pub fn get_connection(&self) -> &Arc<Connection> {
        &self.conn
    }

    /// The connection's database platform (`getConnection()->getDatabasePlatform()`).
    pub fn get_platform(&self) -> &(dyn AbstractPlatform + Send + Sync) {
        &*self.platform
    }

    /// Gets the metadata factory used to gather the metadata of classes.
    pub fn get_metadata_factory(&self) -> &Arc<ClassMetadataFactory> {
        &self.metadata_factory
    }

    /// Starts a transaction on the underlying database connection.
    pub async fn begin_transaction(&self) -> Result<(), Thrown> {
        self.conn
            .begin_transaction()
            .await
            .map_err(|e| exception::from_dbal(&e))
    }

    /// Executes a function in a transaction: the function gets the entity
    /// manager; the entity manager is flushed before the commit, and
    /// closed (and the transaction rolled back) when anything fails.
    pub async fn wrap_in_transaction<T, F, Fut>(self: &Arc<Self>, func: F) -> Result<T, Thrown>
    where
        F: FnOnce(Arc<Self>) -> Fut,
        Fut: Future<Output = Result<T, Thrown>>,
    {
        self.begin_transaction().await?;
        let outcome = async {
            let value = func(self.clone()).await?;
            self.flush().await?;
            self.commit().await?;
            Ok(value)
        }
        .await;
        if outcome.is_err() {
            self.close();
            if self.conn.is_transaction_active() {
                // A failing rollback replaces the error (PHP's `finally`).
                self.rollback().await?;
            }
        }
        outcome
    }

    /// Commits a transaction on the underlying database connection.
    pub async fn commit(&self) -> Result<(), Thrown> {
        self.conn
            .commit()
            .await
            .map_err(|e| exception::from_dbal(&e))
    }

    /// Performs a rollback on the underlying database connection.
    pub async fn rollback(&self) -> Result<(), Thrown> {
        self.conn
            .roll_back()
            .await
            .map_err(|e| exception::from_dbal(&e))
    }

    /// Returns the ORM metadata descriptor for a class.
    pub fn get_class_metadata(&self, class_name: &str) -> Result<Arc<ClassMetadata>, Thrown> {
        self.metadata_factory.get_metadata_for(class_name)
    }

    /// `$em->getMetadataFactory()->isTransient($className)`.
    pub fn is_transient(&self, class_name: &str) -> Result<bool, Thrown> {
        self.metadata_factory.is_transient(class_name)
    }

    /// Flushes all changes to objects that have been queued up to now to
    /// the database.
    pub async fn flush(&self) -> Result<(), Thrown> {
        self.error_if_closed()?;
        self.unit_of_work.commit().await
    }

    /// Finds an Entity by its identifier (`LockMode::NONE`).
    ///
    /// An uninitialized proxy of the identity map is initialized before it
    /// is returned (PHP initializes it on its first property access).
    pub async fn find(&self, class_name: &str, id: Value) -> Result<Option<Object>, Thrown> {
        self.find_with_lock(class_name, id, LockMode::None, None)
            .await
    }

    /// Finds an Entity by its identifier, with a lock mode and the
    /// version for an optimistic lock.
    ///
    /// The `SQLite` platform has no lock clause, so a pessimistic lock
    /// loads as `LockMode::NONE` does (inside the required transaction).
    #[allow(clippy::too_many_lines, reason = "find(), in its order")]
    pub async fn find_with_lock(
        &self,
        class_name: &str,
        id: Value,
        lock_mode: LockMode,
        lock_version: Option<Value>,
    ) -> Result<Option<Object>, Thrown> {
        let class = self
            .metadata_factory
            .get_metadata_for(class_name.trim_start_matches('\\'))?;
        self.check_lock_requirements(lock_mode, &class)?;
        let (identifier, is_composite, name, root) = {
            let info = class.read();
            (
                info.identifier.clone(),
                info.is_identifier_composite,
                info.name.clone(),
                info.root_entity_name.clone(),
            )
        };

        let mut id = match id {
            Value::Array(id) => id,
            other => {
                if is_composite {
                    return Err(exception::invalid_composite_identifier());
                }
                let mut array = PhpArray::new();
                array.insert(
                    identifier.first().cloned().unwrap_or_default().as_str(),
                    other,
                );
                array
            }
        };

        let keys: Vec<Key> = id.keys().cloned().collect();
        for key in keys {
            let Some(Value::Object(value)) = id.get(&key).cloned() else {
                continue;
            };
            let value_class = value.class().to_owned();
            if self.metadata_factory.has_metadata_for(&value_class) {
                let single = self.unit_of_work.get_single_identifier_value(&value)?;
                if single.is_null() {
                    return Err(exception::invalid_identifier_binding_entity(&value_class));
                }
                id.insert(key, single);
            }
        }

        let mut sorted_id = PhpArray::new();
        for field in &identifier {
            let key = Key::from(field.as_str());
            match id.get(&key) {
                Some(value) if !value.is_null() => {
                    let value = crate::utility::identifier_flattener_backed(value.clone());
                    sorted_id.insert(field.as_str(), value);
                }
                _ => return Err(exception::missing_identifier_field(field, &name)),
            }
            id.remove(&key);
        }
        if !id.is_empty() {
            let fields: Vec<String> = id.keys().map(crate::utility::key_text).collect();
            return Err(exception::unrecognized_identifier_fields(&name, &fields));
        }

        // Check identity map first
        if let Some(entity) = self.unit_of_work.try_get_by_id(&sorted_id, &root) {
            if !entity.instance_of(&name) {
                return Ok(None);
            }
            match lock_mode {
                LockMode::Optimistic => {
                    self.lock(&entity, lock_mode, lock_version.as_ref()).await?;
                }
                LockMode::PessimisticRead | LockMode::PessimisticWrite => {
                    self.unit_of_work
                        .get_entity_persister(&name)?
                        .refresh(&sorted_id, &entity)
                        .await?;
                }
                LockMode::None => {}
            }
            if self
                .unit_of_work
                .is_uninitialized_object(&Value::Object(entity.clone()))
            {
                self.unit_of_work.initialize_object(&entity).await?;
            }
            return Ok(Some(entity)); // Hit!
        }

        let persister = self.unit_of_work.get_entity_persister(&name)?;
        match lock_mode {
            LockMode::Optimistic => {
                let entity = persister
                    .load(&sorted_id, None, None, Hints::default(), None, None)
                    .await?;
                if let Some(entity) = &entity {
                    self.unit_of_work
                        .lock(entity, lock_mode, lock_version.as_ref())
                        .await?;
                }
                Ok(entity)
            }
            LockMode::PessimisticRead | LockMode::PessimisticWrite => {
                persister
                    .load(&sorted_id, None, None, Hints::default(), None, None)
                    .await
            }
            LockMode::None => persister.load_by_id(&sorted_id, None).await,
        }
    }

    /// Acquire a lock on the given entity.
    pub async fn lock(
        &self,
        entity: &Object,
        lock_mode: LockMode,
        lock_version: Option<&Value>,
    ) -> Result<(), Thrown> {
        self.unit_of_work
            .lock(entity, lock_mode, lock_version)
            .await
    }

    fn check_lock_requirements(
        &self,
        lock_mode: LockMode,
        class: &ClassMetadata,
    ) -> Result<(), Thrown> {
        match lock_mode {
            LockMode::Optimistic => {
                let info = class.read();
                if !info.is_versioned {
                    return Err(exception::not_versioned(&info.name));
                }
            }
            LockMode::PessimisticRead | LockMode::PessimisticWrite => {
                if !self.get_connection().is_transaction_active() {
                    return Err(exception::transaction_required());
                }
            }
            LockMode::None => {}
        }
        Ok(())
    }

    /// Gets a reference to the entity identified by the given type and
    /// identifier without actually loading it, if the entity is not yet
    /// loaded.
    pub async fn get_reference(
        &self,
        entity_name: &str,
        id: Value,
    ) -> Result<Option<Object>, Thrown> {
        let class = self
            .metadata_factory
            .get_metadata_for(entity_name.trim_start_matches('\\'))?;
        let (identifier, name, root, has_subclasses) = {
            let info = class.read();
            (
                info.identifier.clone(),
                info.name.clone(),
                info.root_entity_name.clone(),
                !info.sub_classes.is_empty(),
            )
        };
        let mut id = match id {
            Value::Array(id) => id,
            other => {
                let mut array = PhpArray::new();
                array.insert(
                    identifier.first().cloned().unwrap_or_default().as_str(),
                    other,
                );
                array
            }
        };
        let mut sorted_id = PhpArray::new();
        for field in &identifier {
            let key = Key::from(field.as_str());
            match id.get(&key) {
                Some(value) if !value.is_null() => {
                    sorted_id.insert(field.as_str(), value.clone());
                }
                _ => return Err(exception::missing_identifier_field(field, &name)),
            }
            id.remove(&key);
        }
        if !id.is_empty() {
            let fields: Vec<String> = id.keys().map(crate::utility::key_text).collect();
            return Err(exception::unrecognized_identifier_fields(&name, &fields));
        }

        // Check identity map first, if its already in there just return it.
        if let Some(entity) = self.unit_of_work.try_get_by_id(&sorted_id, &root) {
            return Ok(entity.instance_of(&name).then_some(entity));
        }

        if has_subclasses {
            return self.find(entity_name, Value::Array(sorted_id)).await;
        }

        let entity = self.get_proxy(&name, &sorted_id)?;
        self.unit_of_work
            .register_managed_proxy(&entity, sorted_id)?;
        Ok(Some(entity))
    }

    /// `ProxyFactory::getProxy()`: an uninitialized lazy ghost of the class
    /// with its identifier set.
    pub fn get_proxy(&self, class_name: &str, identifier: &PhpArray) -> Result<Object, Thrown> {
        let class = self.get_class_metadata(class_name)?;
        let info = class.read();
        let proxy = info.new_instance()?;
        info.set_identifier_values(&proxy, identifier)?;
        if let Some(ghost) = downcast::<EntityObject>(&proxy) {
            ghost.set_lazy(true);
        }
        Ok(proxy)
    }

    /// Clears the `EntityManager`. All entities that are currently managed
    /// by this `EntityManager` become detached.
    pub fn clear(&self) -> Result<(), Thrown> {
        self.unit_of_work.clear()
    }

    /// Closes the `EntityManager`. All entities that are currently managed
    /// by this `EntityManager` become detached. The `EntityManager` may no
    /// longer be used after it is closed.
    pub fn close(&self) {
        let _ = self.clear();
        *self.closed.write().unwrap_or_else(PoisonError::into_inner) = true;
    }

    fn is_closed(&self) -> bool {
        *self.closed.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Tells the `EntityManager` to make an instance managed and persistent.
    pub fn persist(&self, object: &Object) -> Result<(), Thrown> {
        self.error_if_closed()?;
        self.unit_of_work.persist(object)
    }

    /// Removes an entity instance.
    pub async fn remove(&self, object: &Object) -> Result<(), Thrown> {
        self.error_if_closed()?;
        self.unit_of_work.remove(object).await
    }

    /// Refreshes the persistent state of an entity from the database,
    /// overriding any local changes that have not yet been persisted.
    pub async fn refresh(&self, object: &Object) -> Result<(), Thrown> {
        self.error_if_closed()?;
        self.unit_of_work.refresh(object).await
    }

    /// Detaches an entity from the `EntityManager`, causing a managed entity
    /// to become detached.
    pub fn detach(&self, object: &Object) -> Result<(), Thrown> {
        self.unit_of_work.detach(object)
    }

    /// Gets the repository for an entity class.
    pub fn get_repository(
        self: &Arc<Self>,
        class_name: &str,
    ) -> Result<Arc<EntityRepository>, Thrown> {
        self.repository_factory.get_repository(self, class_name)
    }

    /// Determines whether an entity instance is managed in this
    /// `EntityManager`.
    pub fn contains(&self, object: &Object) -> Result<bool, Thrown> {
        Ok(self.unit_of_work.is_scheduled_for_insert(object)
            || self.unit_of_work.is_in_identity_map(object)?
                && !self.unit_of_work.is_scheduled_for_delete(object))
    }

    /// The event manager (a copy; [`Self::with_event_manager`] changes it).
    pub fn get_event_manager(&self) -> EventManager {
        self.event_manager
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Changes the event manager (`$em->getEventManager()->addEventListener(...)`).
    pub fn with_event_manager<T>(&self, f: impl FnOnce(&mut EventManager) -> T) -> T {
        f(&mut self
            .event_manager
            .write()
            .unwrap_or_else(PoisonError::into_inner))
    }

    pub fn get_configuration(&self) -> &Configuration {
        &self.config
    }

    fn error_if_closed(&self) -> Result<(), Thrown> {
        if self.is_closed() {
            return Err(exception::entity_manager_closed());
        }
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        !self.is_closed()
    }

    pub fn get_unit_of_work(&self) -> &UnitOfWork {
        &self.unit_of_work
    }

    /// Helper method to initialize a lazy loading proxy or persistent
    /// collection.
    pub async fn initialize_object(&self, obj: &Object) -> Result<(), Thrown> {
        self.unit_of_work.initialize_object(obj).await
    }

    /// Helper method to check whether a lazy loading proxy or persistent
    /// collection has been initialized.
    pub fn is_uninitialized_object(&self, value: &Value) -> bool {
        self.unit_of_work.is_uninitialized_object(value)
    }
}

/// The DQL builders of the entity manager.
impl EntityManager {
    /// Gets an `ExpressionBuilder` used for object-oriented construction of query expressions.
    ///
    /// Upstream keeps one `Expr` per entity manager; `Expr` has no state,
    /// so this is a new one.
    pub fn get_expression_builder(&self) -> crate::query::Expr {
        crate::query::Expr::new()
    }

    /// Create a `QueryBuilder` instance
    pub fn create_query_builder(self: &Arc<Self>) -> crate::query_builder::QueryBuilder {
        crate::query_builder::QueryBuilder::new(
            Arc::clone(self) as Arc<dyn crate::query::context::QueryContext>
        )
    }
}
