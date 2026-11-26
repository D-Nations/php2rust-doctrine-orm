//! The `ClassMetadataFactory` is used to create `ClassMetadata` objects that
//! contain all the metadata mapping information of a class which describes
//! how a class should be mapped to a relational database.
//!
//! This class was abstracted from the ORM `ClassMetadataFactory`.
//!
//! The abstract class is the [`AbstractClassMetadataFactory`] trait: an
//! implementation holds a [`ClassMetadataFactoryState`] (the class's
//! properties) and implements the abstract methods; the others come with
//! the trait and can be overridden (`parent::method()` is the function of
//! the same name in this module). Every implementation is a
//! [`ClassMetadataFactory`].
//!
//! Upstream: doctrine/persistence src/Mapping/AbstractClassMetadataFactory.php

use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};

use php_value::Thrown;

use crate::cache::{CacheItemPool, CacheValue};
use crate::mapping::driver::MappingDriver;
use crate::mapping::{
    ClassMetadata, ClassMetadataFactory, ProxyClassNameResolver, ReflectionService,
    RuntimeReflectionService, mapping_exception,
};
use crate::php_reflection::{self, ReflectionClass};
use crate::proxy;

/// Salt used by specific Object Manager implementation.
pub const DEFAULT_CACHE_SALT: &str = "__CLASSMETADATA__";

struct Loaded<M> {
    order: Vec<String>,
    by_name: HashMap<String, Arc<M>>,
}

impl<M> Loaded<M> {
    fn get(&self, name: &str) -> Option<Arc<M>> {
        self.by_name.get(name).cloned()
    }

    fn set(&mut self, name: &str, class: Arc<M>) {
        if self.by_name.insert(name.to_owned(), class).is_none() {
            self.order.push(name.to_owned());
        }
    }
}

/// The properties of `AbstractClassMetadataFactory`.
pub struct ClassMetadataFactoryState<M> {
    /// `$cacheSalt`.
    cache_salt: RwLock<String>,
    cache: RwLock<Option<Arc<dyn CacheItemPool>>>,
    loaded_metadata: Mutex<Loaded<M>>,
    /// `$initialized`.
    initialized: AtomicBool,
    reflection_service: RwLock<Option<Arc<dyn ReflectionService>>>,
    proxy_class_name_resolver: RwLock<Option<Arc<dyn ProxyClassNameResolver>>>,
}

impl<M> Default for ClassMetadataFactoryState<M> {
    fn default() -> Self {
        Self {
            cache_salt: RwLock::new(DEFAULT_CACHE_SALT.to_owned()),
            cache: RwLock::new(None),
            loaded_metadata: Mutex::new(Loaded {
                order: Vec::new(),
                by_name: HashMap::new(),
            }),
            initialized: AtomicBool::new(false),
            reflection_service: RwLock::new(None),
            proxy_class_name_resolver: RwLock::new(None),
        }
    }
}

impl<M> ClassMetadataFactoryState<M> {
    pub fn new() -> Self {
        Self::default()
    }

    /// `$this->initialized`.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// `$this->initialized = $initialized`.
    pub fn set_initialized(&self, initialized: bool) {
        self.initialized.store(initialized, Ordering::SeqCst);
    }

    /// `$this->cacheSalt`.
    pub fn cache_salt(&self) -> String {
        self.cache_salt
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// `$this->cacheSalt = $salt`.
    pub fn set_cache_salt(&self, salt: impl Into<String>) {
        *self
            .cache_salt
            .write()
            .unwrap_or_else(PoisonError::into_inner) = salt.into();
    }

    fn loaded(&self) -> std::sync::MutexGuard<'_, Loaded<M>> {
        self.loaded_metadata
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn loaded_get(&self, name: &str) -> Option<Arc<M>> {
        self.loaded().get(name)
    }

    fn loaded_set(&self, name: &str, class: Arc<M>) {
        self.loaded().set(name, class);
    }
}

/// `Doctrine\Persistence\Mapping\AbstractClassMetadataFactory`.
pub trait AbstractClassMetadataFactory: Send + Sync {
    /// The metadata class the factory creates.
    type Metadata: ClassMetadata;

    fn factory_state(&self) -> &ClassMetadataFactoryState<Self::Metadata>;

    /// Lazy initialization of this stuff, especially the metadata driver,
    /// since these are not needed at all when a metadata cache is active.
    fn initialize(&self) -> Result<(), Thrown>;

    /// Returns the mapping driver implementation.
    fn get_driver(&self) -> Result<Arc<dyn MappingDriver>, Thrown>;

    /// Wakes up reflection after `ClassMetadata` gets unserialized from
    /// cache.
    fn wakeup_reflection(
        &self,
        class: &Arc<Self::Metadata>,
        refl_service: &dyn ReflectionService,
    ) -> Result<(), Thrown>;

    /// Initializes Reflection after `ClassMetadata` was constructed.
    fn initialize_reflection(
        &self,
        class: &Arc<Self::Metadata>,
        refl_service: &dyn ReflectionService,
    ) -> Result<(), Thrown>;

    /// Checks whether the class metadata is an entity.
    ///
    /// This method should return false for mapped superclasses or embedded
    /// classes.
    fn is_entity(&self, class: &Self::Metadata) -> bool;

    /// Actually loads the metadata from the underlying metadata.
    ///
    /// `non_superclass_parents` are all parent class names that are not
    /// marked as mapped superclasses, with the direct parent first.
    fn do_load_metadata(
        &self,
        class: &Arc<Self::Metadata>,
        parent: Option<&Arc<Self::Metadata>>,
        root_entity_found: bool,
        non_superclass_parents: &[String],
    ) -> Result<(), Thrown>;

    /// Creates a new `ClassMetadata` instance for the given class name.
    fn new_class_metadata_instance(&self, class_name: &str) -> Result<Arc<Self::Metadata>, Thrown>;

    /// Provides a fallback hook for loading metadata when loading failed
    /// due to reflection/mapping exceptions.
    fn on_not_found_metadata(
        &self,
        class_name: &str,
    ) -> Result<Option<Arc<Self::Metadata>>, Thrown> {
        let _ = class_name;
        Ok(None)
    }

    /// Sets the cache driver used by the factory to cache `ClassMetadata`
    /// instances.
    fn set_cache(&self, cache: Arc<dyn CacheItemPool>) {
        *self
            .factory_state()
            .cache
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(cache);
    }

    /// `getCache()` (final, protected in PHP).
    fn get_cache(&self) -> Option<Arc<dyn CacheItemPool>> {
        self.factory_state()
            .cache
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Returns an array of all the loaded metadata currently in memory.
    fn get_loaded_metadata(&self) -> Vec<(String, Arc<Self::Metadata>)> {
        let loaded = self.factory_state().loaded();
        loaded
            .order
            .iter()
            .filter_map(|name| loaded.get(name).map(|class| (name.clone(), class)))
            .collect()
    }

    /// Forces the factory to load the metadata of all classes known to the
    /// underlying mapping driver.
    fn get_all_metadata(&self) -> Result<Vec<Arc<Self::Metadata>>, Thrown> {
        get_all_metadata(self)
    }

    fn set_proxy_class_name_resolver(&self, resolver: Arc<dyn ProxyClassNameResolver>) {
        *self
            .factory_state()
            .proxy_class_name_resolver
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(resolver);
    }

    /// Gets the class metadata descriptor for a class.
    fn get_metadata_for(&self, class_name: &str) -> Result<Arc<Self::Metadata>, Thrown> {
        get_metadata_for(self, class_name)
    }

    /// Checks whether the factory has the metadata for a class loaded
    /// already.
    fn has_metadata_for(&self, class_name: &str) -> bool {
        self.factory_state()
            .loaded_get(normalize_class_name(class_name))
            .is_some()
    }

    /// Sets the metadata descriptor for a specific class.
    ///
    /// NOTE: This is only useful in very special cases, like when
    /// generating proxy classes.
    fn set_metadata_for(&self, class_name: &str, class: Arc<Self::Metadata>) {
        self.factory_state()
            .loaded_set(normalize_class_name(class_name), class);
    }

    /// Gets an array of parent classes for the given entity class, the
    /// topmost first.
    fn get_parent_classes(&self, name: &str) -> Result<Vec<String>, Thrown> {
        get_parent_classes(self, name)
    }

    /// Loads the metadata of the class in question and all it's ancestors
    /// whose metadata is still not loaded.
    ///
    /// Important: The class name does not necessarily exist at this point
    /// here. Scenarios in a code-generation setup might have access to
    /// XML/YAML Mapping files without the actual PHP code existing here.
    /// That is why the `ClassMetadataFactory` has to use the
    /// `ReflectionService` interface to get access to the actual classes.
    ///
    /// Returns the names of the classes that have been loaded.
    fn load_metadata(&self, name: &str) -> Result<Vec<String>, Thrown> {
        load_metadata(self, name)
    }

    /// Returns whether the class with the specified name should have its
    /// metadata loaded. This is only the case if it is either mapped
    /// directly or as a `MappedSuperclass`.
    fn is_transient(&self, class_name: &str) -> Result<bool, Thrown> {
        is_transient(self, class_name)
    }

    /// Sets the reflectionService.
    fn set_reflection_service(&self, reflection_service: Arc<dyn ReflectionService>) {
        *self
            .factory_state()
            .reflection_service
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(reflection_service);
    }

    /// Gets the reflection service associated with this metadata factory.
    fn get_reflection_service(&self) -> Arc<dyn ReflectionService> {
        let state = self.factory_state();
        if let Some(service) = state
            .reflection_service
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
        {
            return service;
        }
        let mut slot = state
            .reflection_service
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        Arc::clone(slot.get_or_insert_with(|| Arc::new(RuntimeReflectionService::new())))
    }

    /// `getCacheKey()`.
    fn get_cache_key(&self, real_class_name: &str) -> String {
        get_cache_key(self, real_class_name)
    }
}

fn normalize_class_name(class_name: &str) -> &str {
    class_name.trim_start_matches('\\')
}

/// `parent::getCacheKey()`.
pub fn get_cache_key<F: AbstractClassMetadataFactory + ?Sized>(
    factory: &F,
    real_class_name: &str,
) -> String {
    format!(
        "{}{}",
        real_class_name.replace('\\', "__"),
        factory.factory_state().cache_salt()
    )
}

fn ensure_initialized<F: AbstractClassMetadataFactory + ?Sized>(factory: &F) -> Result<(), Thrown> {
    if !factory.factory_state().is_initialized() {
        factory.initialize()?;
    }
    Ok(())
}

/// `parent::getAllMetadata()`.
pub fn get_all_metadata<F: AbstractClassMetadataFactory + ?Sized>(
    factory: &F,
) -> Result<Vec<Arc<F::Metadata>>, Thrown> {
    ensure_initialized(factory)?;
    let driver = factory.get_driver()?;
    let mut metadata = Vec::new();
    for class_name in driver.get_all_class_names()? {
        metadata.push(factory.get_metadata_for(&class_name)?);
    }
    Ok(metadata)
}

/// `class_exists($className, false) && (new ReflectionClass($className))->isAnonymous()`.
fn is_anonymous_class(class_name: &str) -> bool {
    php_reflection::class_exists(class_name)
        && ReflectionClass::new(class_name).is_ok_and(|c| c.is_anonymous())
}

/// `parent::getMetadataFor()`.
pub fn get_metadata_for<F: AbstractClassMetadataFactory + ?Sized>(
    factory: &F,
    class_name: &str,
) -> Result<Arc<F::Metadata>, Thrown> {
    let class_name = normalize_class_name(class_name);
    let state = factory.factory_state();
    if let Some(loaded) = state.loaded_get(class_name) {
        return Ok(loaded);
    }

    if is_anonymous_class(class_name) {
        return Err(mapping_exception::class_is_anonymous(class_name));
    }

    if !php_reflection::class_exists(class_name) && class_name.contains(':') {
        return Err(mapping_exception::non_existing_class(class_name));
    }

    let real_class_name = get_real_class(factory, class_name);

    if let Some(loaded) = state.loaded_get(&real_class_name) {
        // We do not have the alias name in the map, include it
        state.loaded_set(class_name, Arc::clone(&loaded));
        return Ok(loaded);
    }

    match load_through_cache(factory, &real_class_name) {
        Ok(()) => {}
        Err(loading_exception) if mapping_exception::is_mapping_exception(&loading_exception) => {
            let Some(fallback_metadata_response) =
                factory.on_not_found_metadata(&real_class_name)?
            else {
                return Err(loading_exception);
            };
            state.loaded_set(&real_class_name, fallback_metadata_response);
        }
        Err(other) => return Err(other),
    }

    let Some(loaded) = state.loaded_get(&real_class_name) else {
        return Err(mapping_exception::non_existing_class(&real_class_name));
    };
    if class_name != real_class_name {
        // We do not have the alias name in the map, include it
        state.loaded_set(class_name, Arc::clone(&loaded));
    }
    Ok(loaded)
}

/// The `try` block of `getMetadataFor()`.
fn load_through_cache<F: AbstractClassMetadataFactory + ?Sized>(
    factory: &F,
    real_class_name: &str,
) -> Result<(), Thrown> {
    let state = factory.factory_state();
    let Some(cache) = factory.get_cache() else {
        factory.load_metadata(real_class_name)?;
        return Ok(());
    };
    let cached = cache
        .get_item(&factory.get_cache_key(real_class_name))?
        .get()
        .and_then(|value| value.downcast_ref::<Arc<F::Metadata>>().cloned());
    if let Some(cached) = cached {
        state.loaded_set(real_class_name, Arc::clone(&cached));
        factory.wakeup_reflection(&cached, &*factory.get_reflection_service())?;
        return Ok(());
    }
    let loaded_metadata = factory.load_metadata(real_class_name)?;
    let class_names: Vec<(String, String)> = loaded_metadata
        .iter()
        .map(|name| (factory.get_cache_key(name), name.clone()))
        .collect();
    let keys: Vec<String> = class_names.iter().map(|(key, _)| key.clone()).collect();
    for item in cache.get_items(&keys)? {
        let key = item.get_key();
        let Some((_, name)) = class_names.iter().find(|(k, _)| *k == key) else {
            continue;
        };
        let value = state
            .loaded_get(name)
            .map(|class| Arc::new(class) as CacheValue);
        item.set(value);
        cache.save_deferred(item);
    }
    cache.commit();
    Ok(())
}

/// `parent::getParentClasses()`.
pub fn get_parent_classes<F: AbstractClassMetadataFactory + ?Sized>(
    factory: &F,
    name: &str,
) -> Result<Vec<String>, Thrown> {
    // Collect parent classes, ignoring transient (not-mapped) classes.
    let mut parent_classes = Vec::new();
    let mut parents = factory.get_reflection_service().get_parent_classes(name)?;
    parents.reverse();
    for parent_class in parents {
        if factory.get_driver()?.is_transient(&parent_class)? {
            continue;
        }
        parent_classes.push(parent_class);
    }
    Ok(parent_classes)
}

/// `parent::loadMetadata()`.
pub fn load_metadata<F: AbstractClassMetadataFactory + ?Sized>(
    factory: &F,
    name: &str,
) -> Result<Vec<String>, Thrown> {
    ensure_initialized(factory)?;
    let state = factory.factory_state();

    let mut loaded = Vec::new();

    let mut parent_classes = factory.get_parent_classes(name)?;
    parent_classes.push(name.to_owned());

    // Move down the hierarchy of parent classes, starting from the topmost class
    let mut parent: Option<Arc<F::Metadata>> = None;
    let mut root_entity_found = false;
    let mut visited: Vec<String> = Vec::new();
    let refl_service = factory.get_reflection_service();

    for class_name in parent_classes {
        if let Some(loaded_parent) = state.loaded_get(&class_name) {
            if factory.is_entity(&loaded_parent) {
                root_entity_found = true;
                visited.insert(0, class_name);
            }
            parent = Some(loaded_parent);
            continue;
        }

        let class = factory.new_class_metadata_instance(&class_name)?;
        factory.initialize_reflection(&class, &*refl_service)?;

        factory.do_load_metadata(&class, parent.as_ref(), root_entity_found, &visited)?;

        state.loaded_set(&class_name, Arc::clone(&class));

        if factory.is_entity(&class) {
            root_entity_found = true;
            visited.insert(0, class_name.clone());
        }

        factory.wakeup_reflection(&class, &*refl_service)?;

        parent = Some(class);
        loaded.push(class_name);
    }

    Ok(loaded)
}

/// `parent::isTransient()`.
pub fn is_transient<F: AbstractClassMetadataFactory + ?Sized>(
    factory: &F,
    class_name: &str,
) -> Result<bool, Thrown> {
    ensure_initialized(factory)?;

    if is_anonymous_class(class_name) {
        return Ok(false);
    }

    if !php_reflection::class_exists(class_name) && class_name.contains(':') {
        return Err(mapping_exception::non_existing_class(class_name));
    }

    factory.get_driver()?.is_transient(class_name)
}

/// The resolver `createDefaultProxyClassNameResolver()` creates.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultProxyClassNameResolver;

impl ProxyClassNameResolver for DefaultProxyClassNameResolver {
    fn resolve_class_name(&self, class_name: &str) -> String {
        let marker = format!("\\{}\\", proxy::MARKER);
        match class_name.rfind(&marker) {
            None => class_name.to_owned(),
            Some(pos) => class_name
                .get(pos + proxy::MARKER_LENGTH + 2..)
                .unwrap_or_default()
                .to_owned(),
        }
    }
}

fn get_real_class<F: AbstractClassMetadataFactory + ?Sized>(factory: &F, class: &str) -> String {
    let state = factory.factory_state();
    let resolver = state
        .proxy_class_name_resolver
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let resolver = resolver.unwrap_or_else(|| {
        let resolver: Arc<dyn ProxyClassNameResolver> = Arc::new(DefaultProxyClassNameResolver);
        *state
            .proxy_class_name_resolver
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&resolver));
        resolver
    });
    resolver.resolve_class_name(class)
}

impl<F: AbstractClassMetadataFactory> ClassMetadataFactory for F {
    fn get_all_metadata(&self) -> Result<Vec<Arc<dyn ClassMetadata>>, Thrown> {
        Ok(AbstractClassMetadataFactory::get_all_metadata(self)?
            .into_iter()
            .map(|m| m as Arc<dyn ClassMetadata>)
            .collect())
    }

    fn get_metadata_for(&self, class_name: &str) -> Result<Arc<dyn ClassMetadata>, Thrown> {
        Ok(AbstractClassMetadataFactory::get_metadata_for(
            self, class_name,
        )?)
    }

    fn has_metadata_for(&self, class_name: &str) -> bool {
        AbstractClassMetadataFactory::has_metadata_for(self, class_name)
    }

    fn set_metadata_for(
        &self,
        class_name: &str,
        class: Arc<dyn ClassMetadata>,
    ) -> Result<(), Thrown> {
        let any: Arc<dyn Any + Send + Sync> = class;
        let class = any.downcast::<F::Metadata>().map_err(|_| {
            Thrown::new(
                "TypeError",
                format!(
                    "the metadata for {class_name} is not a {}",
                    std::any::type_name::<F::Metadata>()
                ),
                0,
            )
        })?;
        AbstractClassMetadataFactory::set_metadata_for(self, class_name, class);
        Ok(())
    }

    fn is_transient(&self, class_name: &str) -> Result<bool, Thrown> {
        AbstractClassMetadataFactory::is_transient(self, class_name)
    }
}
