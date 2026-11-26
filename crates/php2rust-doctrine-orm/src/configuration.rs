//! Configuration container for all configuration options of Doctrine.
//! It combines all configuration options from DBAL & ORM.
//!
//! The entity manager takes the configuration by value and shares it
//! (PHP shares the object, and changes after the entity manager was
//! created are seen by it; here they are not, except the filters, which
//! the clones share). Caches, custom
//! hydration modes, the entity listener resolver and the second level
//! cache are not ported; the custom DQL functions are class names, as
//! upstream keeps them.
//!
//! Upstream: doctrine/orm src/Configuration.php

use std::fmt;
use std::sync::{Arc, PoisonError, RwLock};

use php_value::{PhpArray, Thrown};
use php2rust_doctrine_persistence::mapping::driver::MappingDriver;
use php2rust_doctrine_persistence::php_reflection;

use crate::mapping::naming_strategy::{DefaultNamingStrategy, NamingStrategy};
use crate::mapping::quote_strategy::{DefaultQuoteStrategy, QuoteStrategy};
use crate::mapping::typed_field_mapper::{DefaultTypedFieldMapper, TypedFieldMapper};
use crate::query::context::FunctionFactory;
use crate::repository::{DefaultRepositoryFactory, RepositoryFactory};

/// `Doctrine\ORM\EntityRepository`, the default repository class.
pub const ENTITY_REPOSITORY_CLASS: &str = "Doctrine\\ORM\\EntityRepository";

/// `Doctrine\ORM\Configuration`.
#[derive(Clone)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "upstream's configuration flags"
)]
pub struct Configuration {
    identity_generation_preferences: Vec<(String, i64)>,
    proxy_dir: Option<String>,
    proxy_namespace: Option<String>,
    auto_generate_proxy_classes: i64,
    use_dbal_editor_api: bool,
    metadata_driver_impl: Option<Arc<dyn MappingDriver>>,
    entity_namespaces: Vec<(String, String)>,
    custom_string_functions: Vec<(String, String)>,
    custom_numeric_functions: Vec<(String, String)>,
    custom_datetime_functions: Vec<(String, String)>,
    typed_field_mapper: Option<Arc<dyn TypedFieldMapper>>,
    default_repository_class_name: Option<String>,
    naming_strategy: Option<Arc<dyn NamingStrategy>>,
    quote_strategy: Option<Arc<dyn QuoteStrategy>>,
    repository_factory: Option<Arc<dyn RepositoryFactory>>,
    second_level_cache_enabled: bool,
    default_query_hints: PhpArray,
    schema_ignore_classes: Vec<String>,
    native_lazy_objects: bool,
    reject_id_collision: bool,
    eager_fetch_batch_size: Option<i64>,
    default_string_type_schema_length: Option<i64>,
    /// `addFilter()`: name → class name, shared by the clones (PHP shares
    /// the configuration object, so a filter added after the entity
    /// manager was created is seen by it).
    filters: Arc<RwLock<Vec<(String, String)>>>,
    /// The custom functions given as a callable (`callable(string):
    /// FunctionNode`) instead of a class name: lowercased name → factory.
    custom_string_function_callables: Vec<(String, FunctionFactory)>,
    custom_numeric_function_callables: Vec<(String, FunctionFactory)>,
    custom_datetime_function_callables: Vec<(String, FunctionFactory)>,
}

impl fmt::Debug for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Configuration").finish_non_exhaustive()
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self::new()
    }
}

impl Configuration {
    pub fn new() -> Self {
        Self {
            identity_generation_preferences: Vec::new(),
            proxy_dir: None,
            proxy_namespace: None,
            auto_generate_proxy_classes: 0,
            use_dbal_editor_api: false,
            metadata_driver_impl: None,
            entity_namespaces: Vec::new(),
            custom_string_functions: Vec::new(),
            custom_numeric_functions: Vec::new(),
            custom_datetime_functions: Vec::new(),
            custom_string_function_callables: Vec::new(),
            custom_numeric_function_callables: Vec::new(),
            custom_datetime_function_callables: Vec::new(),
            typed_field_mapper: None,
            default_repository_class_name: None,
            naming_strategy: None,
            quote_strategy: None,
            repository_factory: None,
            second_level_cache_enabled: false,
            default_query_hints: PhpArray::new(),
            schema_ignore_classes: Vec::new(),
            // PHP 8.4: ORMSetup enables native lazy objects.
            native_lazy_objects: true,
            reject_id_collision: true,
            eager_fetch_batch_size: None,
            default_string_type_schema_length: None,
            filters: Arc::default(),
        }
    }

    /// `setIdentityGenerationPreferences()`: platform class → generator
    /// type.
    pub fn set_identity_generation_preferences(&mut self, value: Vec<(String, i64)>) {
        self.identity_generation_preferences = value;
    }

    pub fn get_identity_generation_preferences(&self) -> &[(String, i64)] {
        &self.identity_generation_preferences
    }

    pub fn set_proxy_dir(&mut self, dir: impl Into<String>) {
        self.proxy_dir = Some(dir.into());
    }

    pub fn get_proxy_dir(&self) -> Option<&str> {
        self.proxy_dir.as_deref()
    }

    pub fn get_auto_generate_proxy_classes(&self) -> i64 {
        self.auto_generate_proxy_classes
    }

    pub fn set_auto_generate_proxy_classes(&mut self, auto_generate: i64) {
        self.auto_generate_proxy_classes = auto_generate;
    }

    pub fn get_proxy_namespace(&self) -> Option<&str> {
        self.proxy_namespace.as_deref()
    }

    pub fn set_proxy_namespace(&mut self, ns: impl Into<String>) {
        self.proxy_namespace = Some(ns.into());
    }

    pub fn get_use_dbal_editor_api(&self) -> bool {
        self.use_dbal_editor_api
    }

    pub fn set_use_dbal_editor_api(&mut self, use_api: bool) {
        self.use_dbal_editor_api = use_api;
    }

    pub fn set_metadata_driver_impl(&mut self, driver: Arc<dyn MappingDriver>) {
        self.metadata_driver_impl = Some(driver);
    }

    pub fn get_metadata_driver_impl(&self) -> Option<&Arc<dyn MappingDriver>> {
        self.metadata_driver_impl.as_ref()
    }

    pub fn set_entity_namespaces(&mut self, namespaces: Vec<(String, String)>) {
        self.entity_namespaces = namespaces;
    }

    pub fn get_entity_namespaces(&self) -> &[(String, String)] {
        &self.entity_namespaces
    }

    fn get_function(list: &[(String, String)], name: &str) -> Option<String> {
        let name = name.to_ascii_lowercase();
        list.iter()
            .find(|(known, _)| *known == name)
            .map(|(_, class)| class.clone())
    }

    fn remove_callable(callables: &mut Vec<(String, FunctionFactory)>, name: &str) {
        let name = name.to_ascii_lowercase();
        callables.retain(|(known, _)| *known != name);
    }

    fn add_callable(
        callables: &mut Vec<(String, FunctionFactory)>,
        classes: &mut Vec<(String, String)>,
        name: &str,
        factory: FunctionFactory,
    ) {
        let name = name.to_ascii_lowercase();
        classes.retain(|(known, _)| *known != name);
        callables.retain(|(known, _)| *known != name);
        callables.push((name, factory));
    }

    fn get_callable(
        callables: &[(String, FunctionFactory)],
        name: &str,
    ) -> Option<FunctionFactory> {
        let name = name.to_ascii_lowercase();
        callables
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, factory)| Arc::clone(factory))
    }

    fn add_function(list: &mut Vec<(String, String)>, name: &str, class: &str) {
        let name = name.to_ascii_lowercase();
        list.retain(|(known, _)| *known != name);
        list.push((name, class.to_owned()));
    }

    /// Adds a filter to the list of possible filters. The filters are
    /// shared by the clones of the configuration, and the entity manager
    /// sees the filters added after it was created, as PHP's shared
    /// configuration object does.
    pub fn add_filter(&self, name: &str, class_name: &str) {
        let mut filters = self.filters.write().unwrap_or_else(PoisonError::into_inner);
        match filters.iter_mut().find(|(known, _)| known == name) {
            Some(entry) => class_name.clone_into(&mut entry.1),
            None => filters.push((name.to_owned(), class_name.to_owned())),
        }
    }

    /// Gets the class name for a given filter name (`None` when it is not
    /// defined).
    pub fn get_filter_class_name(&self, name: &str) -> Option<String> {
        self.filters
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .find(|(known, _)| known == name)
            .map(|(_, class)| class.clone())
    }

    /// `addCustomStringFunction()`: the name is case-insensitive.
    pub fn add_custom_string_function(&mut self, name: &str, class_name: &str) {
        Self::remove_callable(&mut self.custom_string_function_callables, name);
        Self::add_function(&mut self.custom_string_functions, name, class_name);
    }

    /// `addCustomStringFunction()` with a callable (`callable(string):
    /// FunctionNode`) instead of a class name.
    pub fn add_custom_string_function_callable(&mut self, name: &str, factory: FunctionFactory) {
        Self::add_callable(
            &mut self.custom_string_function_callables,
            &mut self.custom_string_functions,
            name,
            factory,
        );
    }

    /// `getCustomStringFunction()` when it gives a callable.
    pub fn get_custom_string_function_callable(&self, name: &str) -> Option<FunctionFactory> {
        Self::get_callable(&self.custom_string_function_callables, name)
    }

    pub fn get_custom_string_function(&self, name: &str) -> Option<String> {
        Self::get_function(&self.custom_string_functions, name)
    }

    pub fn set_custom_string_functions(&mut self, functions: &[(&str, &str)]) {
        for (name, class) in functions {
            self.add_custom_string_function(name, class);
        }
    }

    pub fn add_custom_numeric_function(&mut self, name: &str, class_name: &str) {
        Self::remove_callable(&mut self.custom_numeric_function_callables, name);
        Self::add_function(&mut self.custom_numeric_functions, name, class_name);
    }

    /// `addCustomNumericFunction()` with a callable.
    pub fn add_custom_numeric_function_callable(&mut self, name: &str, factory: FunctionFactory) {
        Self::add_callable(
            &mut self.custom_numeric_function_callables,
            &mut self.custom_numeric_functions,
            name,
            factory,
        );
    }

    /// `getCustomNumericFunction()` when it gives a callable.
    pub fn get_custom_numeric_function_callable(&self, name: &str) -> Option<FunctionFactory> {
        Self::get_callable(&self.custom_numeric_function_callables, name)
    }

    pub fn get_custom_numeric_function(&self, name: &str) -> Option<String> {
        Self::get_function(&self.custom_numeric_functions, name)
    }

    pub fn set_custom_numeric_functions(&mut self, functions: &[(&str, &str)]) {
        for (name, class) in functions {
            self.add_custom_numeric_function(name, class);
        }
    }

    pub fn add_custom_datetime_function(&mut self, name: &str, class_name: &str) {
        Self::remove_callable(&mut self.custom_datetime_function_callables, name);
        Self::add_function(&mut self.custom_datetime_functions, name, class_name);
    }

    /// `addCustomDatetimeFunction()` with a callable.
    pub fn add_custom_datetime_function_callable(&mut self, name: &str, factory: FunctionFactory) {
        Self::add_callable(
            &mut self.custom_datetime_function_callables,
            &mut self.custom_datetime_functions,
            name,
            factory,
        );
    }

    /// `getCustomDatetimeFunction()` when it gives a callable.
    pub fn get_custom_datetime_function_callable(&self, name: &str) -> Option<FunctionFactory> {
        Self::get_callable(&self.custom_datetime_function_callables, name)
    }

    pub fn get_custom_datetime_function(&self, name: &str) -> Option<String> {
        Self::get_function(&self.custom_datetime_functions, name)
    }

    pub fn set_custom_datetime_functions(&mut self, functions: &[(&str, &str)]) {
        for (name, class) in functions {
            self.add_custom_datetime_function(name, class);
        }
    }

    pub fn set_typed_field_mapper(&mut self, mapper: Option<Arc<dyn TypedFieldMapper>>) {
        self.typed_field_mapper = mapper;
    }

    pub fn get_typed_field_mapper(&self) -> Arc<dyn TypedFieldMapper> {
        self.typed_field_mapper
            .clone()
            .unwrap_or_else(|| Arc::new(DefaultTypedFieldMapper::default()))
    }

    /// Sets default repository class: `Doctrine\\ORM\\EntityRepository` or a
    /// registered class extending it.
    pub fn set_default_repository_class_name(&mut self, class_name: &str) -> Result<(), Thrown> {
        let is_repository = class_name.eq_ignore_ascii_case(ENTITY_REPOSITORY_CLASS)
            || php_reflection::class_exists(class_name)
                && php_reflection::is_a(class_name, ENTITY_REPOSITORY_CLASS);
        if !is_repository {
            return Err(crate::exception::invalid_entity_repository(class_name));
        }
        self.default_repository_class_name = Some(class_name.to_owned());
        Ok(())
    }

    pub fn get_default_repository_class_name(&self) -> String {
        self.default_repository_class_name
            .clone()
            .unwrap_or_else(|| ENTITY_REPOSITORY_CLASS.to_owned())
    }

    pub fn set_naming_strategy(&mut self, strategy: Arc<dyn NamingStrategy>) {
        self.naming_strategy = Some(strategy);
    }

    pub fn get_naming_strategy(&self) -> Arc<dyn NamingStrategy> {
        self.naming_strategy
            .clone()
            .unwrap_or_else(|| Arc::new(DefaultNamingStrategy::new()))
    }

    pub fn set_quote_strategy(&mut self, strategy: Arc<dyn QuoteStrategy>) {
        self.quote_strategy = Some(strategy);
    }

    pub fn get_quote_strategy(&self) -> Arc<dyn QuoteStrategy> {
        self.quote_strategy
            .clone()
            .unwrap_or_else(|| Arc::new(DefaultQuoteStrategy::new()))
    }

    pub fn set_repository_factory(&mut self, factory: Arc<dyn RepositoryFactory>) {
        self.repository_factory = Some(factory);
    }

    pub fn get_repository_factory(&self) -> Arc<dyn RepositoryFactory> {
        self.repository_factory
            .clone()
            .unwrap_or_else(|| Arc::new(DefaultRepositoryFactory::new()))
    }

    pub fn is_second_level_cache_enabled(&self) -> bool {
        self.second_level_cache_enabled
    }

    /// `setSecondLevelCacheEnabled()`: the second level cache is not
    /// ported; enabling it is refused.
    pub fn set_second_level_cache_enabled(&mut self, flag: bool) -> Result<(), Thrown> {
        if flag {
            return Err(crate::exception::not_supported());
        }
        self.second_level_cache_enabled = false;
        Ok(())
    }

    pub fn get_default_query_hints(&self) -> &PhpArray {
        &self.default_query_hints
    }

    pub fn set_default_query_hints(&mut self, hints: PhpArray) {
        self.default_query_hints = hints;
    }

    pub fn get_default_query_hint(&self, name: &str) -> Option<&php_value::Value> {
        self.default_query_hints.get(&name.into())
    }

    pub fn set_default_query_hint(&mut self, name: &str, value: php_value::Value) {
        self.default_query_hints.insert(name, value);
    }

    pub fn get_schema_ignore_classes(&self) -> &[String] {
        &self.schema_ignore_classes
    }

    pub fn set_schema_ignore_classes(&mut self, classes: Vec<String>) {
        self.schema_ignore_classes = classes;
    }

    pub fn is_native_lazy_objects_enabled(&self) -> bool {
        self.native_lazy_objects
    }

    /// `enableNativeLazyObjects()`: this port only has native lazy
    /// objects (PHP 8.4); disabling them is refused.
    pub fn enable_native_lazy_objects(&mut self, native_lazy_objects: bool) -> Result<(), Thrown> {
        if !native_lazy_objects {
            return Err(crate::exception::not_supported());
        }
        self.native_lazy_objects = true;
        Ok(())
    }

    pub fn set_reject_id_collision_in_identity_map(&mut self, reject: bool) {
        self.reject_id_collision = reject;
    }

    pub fn is_reject_id_collision_in_identity_map_enabled(&self) -> bool {
        self.reject_id_collision
    }

    pub fn set_eager_fetch_batch_size(&mut self, batch_size: i64) {
        self.eager_fetch_batch_size = Some(batch_size);
    }

    pub fn get_eager_fetch_batch_size(&self) -> i64 {
        self.eager_fetch_batch_size.unwrap_or(100)
    }

    pub fn set_default_string_type_schema_length(&mut self, length: i64) {
        self.default_string_type_schema_length = Some(length);
    }

    pub fn get_default_string_type_schema_length(&self) -> i64 {
        self.default_string_type_schema_length.unwrap_or(255)
    }
}
