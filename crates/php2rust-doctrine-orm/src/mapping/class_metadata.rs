//! `ClassMetadata`: all the mapping information of an entity and its
//! associations.
//!
//! PHP's object is shared by reference and changed by the mapping driver
//! and the factory; here [`ClassMetadata`] is that shared object (what
//! persistence's factory and drivers hold) and [`ClassMetadataInfo`],
//! behind its lock, has the public properties and the methods. Mapping
//! arrays stay `PhpArray`s, as the drivers build them.
//!
//! Not ported: `getMetadataValue()` (a dynamic property read),
//! `LegacyReflectionFields` (the deprecated `$reflFields`), and
//! `invokeLifecycleCallbacks()` (calls into the entity's methods).
//!
//! Upstream: doctrine/orm src/Mapping/ClassMetadata.php src/Mapping/GetReflectionClassImplementation.php

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::query::SortDirection;
use php_value::object::{EnumCase, GenericObject};
use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_persistence::mapping::{
    ClassMetadata as PersistenceClassMetadata, ReflectionService,
};
use php2rust_doctrine_persistence::php_reflection::{self, ClassKind, ReflectionClass};

use crate::deprecations;
use crate::id::IdGenerator;
use crate::mapping::association_mapping::{
    AssociationClass, AssociationKind, AssociationMapping, MANY_TO_MANY, MANY_TO_ONE, ONE_TO_MANY,
    ONE_TO_ONE,
};
use crate::mapping::embedded_class_mapping::{
    ColumnPrefix, DiscriminatorColumnMapping, EmbeddedClassMapping,
};
use crate::mapping::field_mapping::FieldMapping;
use crate::mapping::mapping_exception;
use crate::mapping::naming_strategy::{DefaultNamingStrategy, NamingStrategy};
use crate::mapping::ordered_map::OrderedMap;
use crate::mapping::property_accessors::{
    EmbeddablePropertyAccessor, EnumPropertyAccessor, PropertyAccessor, create_property_accessor,
};
use crate::mapping::property_value as pv;
use crate::mapping::typed_field_mapper::{
    DefaultTypedFieldMapper, TypedFieldMapper, enum_exists, enum_values,
};
use crate::object::EntityObject;

pub const INHERITANCE_TYPE_NONE: i64 = 1;
pub const INHERITANCE_TYPE_JOINED: i64 = 2;
pub const INHERITANCE_TYPE_SINGLE_TABLE: i64 = 3;
pub const GENERATOR_TYPE_AUTO: i64 = 1;
pub const GENERATOR_TYPE_SEQUENCE: i64 = 2;
pub const GENERATOR_TYPE_IDENTITY: i64 = 4;
pub const GENERATOR_TYPE_NONE: i64 = 5;
pub const GENERATOR_TYPE_CUSTOM: i64 = 7;
pub const CHANGETRACKING_DEFERRED_IMPLICIT: i64 = 1;
pub const CHANGETRACKING_DEFERRED_EXPLICIT: i64 = 2;
pub const FETCH_LAZY: i64 = 2;
pub const FETCH_EAGER: i64 = 3;
pub const FETCH_EXTRA_LAZY: i64 = 4;
pub const TO_ONE: i64 = 3;
pub const TO_MANY: i64 = 12;
pub const CACHE_USAGE_READ_ONLY: i64 = 1;
pub const CACHE_USAGE_NONSTRICT_READ_WRITE: i64 = 2;
pub const CACHE_USAGE_READ_WRITE: i64 = 3;
pub const GENERATED_NEVER: i64 = 0;
pub const GENERATED_INSERT: i64 = 1;
pub const GENERATED_ALWAYS: i64 = 2;
pub use crate::mapping::association_mapping::{
    MANY_TO_MANY as ASSOCIATION_MANY_TO_MANY, MANY_TO_ONE as ASSOCIATION_MANY_TO_ONE,
    ONE_TO_MANY as ASSOCIATION_ONE_TO_MANY, ONE_TO_ONE as ASSOCIATION_ONE_TO_ONE,
};

/// The class name.
pub const CLASS: &str = "Doctrine\\ORM\\Mapping\\ClassMetadata";

/// The deprecation of mapping a class to several discriminator values.
pub const DUPLICATE_DISCRIMINATOR_DEPRECATION: &str = "https://github.com/doctrine/orm/issues/3519";
/// The deprecation of string order by directions.
pub const ORDER_BY_DIRECTION_DEPRECATION: &str = "https://github.com/doctrine/orm/issues/11313";
/// The deprecation of `fullyQualifiedClassName(null)`.
pub const NULL_CLASS_NAME_DEPRECATION: &str = "https://github.com/doctrine/orm/pull/11294";

/// An entity listener (`['class' => ..., 'method' => ...]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityListener {
    pub class: String,
    pub method: String,
}

/// The shared `ClassMetadata` object.
#[derive(Debug)]
pub struct ClassMetadata {
    inner: RwLock<ClassMetadataInfo>,
    id: u64,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl ClassMetadata {
    /// `new ClassMetadata($entityName, $namingStrategy, $typedFieldMapper)`.
    pub fn new(
        name: impl Into<String>,
        naming_strategy: Option<Arc<dyn NamingStrategy>>,
        typed_field_mapper: Option<Arc<dyn TypedFieldMapper>>,
    ) -> Self {
        Self::from_info(ClassMetadataInfo::new(
            name,
            naming_strategy,
            typed_field_mapper,
        ))
    }

    pub fn from_info(info: ClassMetadataInfo) -> Self {
        Self {
            inner: RwLock::new(info),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, ClassMetadataInfo> {
        self.inner.read().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, ClassMetadataInfo> {
        self.inner.write().unwrap_or_else(PoisonError::into_inner)
    }

    /// `__toString()`: the class and the object's id.
    pub fn to_php_string(&self) -> String {
        format!("{CLASS}@{}", self.id)
    }
}

impl PersistenceClassMetadata for ClassMetadata {
    fn get_name(&self) -> String {
        self.read().name.clone()
    }

    fn get_identifier(&self) -> Vec<String> {
        self.read().identifier.clone()
    }

    fn get_reflection_class(&self) -> Result<ReflectionClass, Thrown> {
        let info = self.read();
        match &info.refl_class {
            Some(class) => Ok(class.clone()),
            None => ReflectionClass::new(&info.name),
        }
    }

    fn is_identifier(&self, field_name: &str) -> bool {
        self.read().is_identifier(field_name)
    }

    fn has_field(&self, field_name: &str) -> bool {
        self.read().has_field(field_name)
    }

    fn has_association(&self, field_name: &str) -> bool {
        self.read().has_association(field_name)
    }

    fn is_single_valued_association(&self, field_name: &str) -> bool {
        self.read().is_single_valued_association(field_name)
    }

    fn is_collection_valued_association(&self, field_name: &str) -> bool {
        self.read().is_collection_valued_association(field_name)
    }

    fn get_field_names(&self) -> Vec<String> {
        self.read().get_field_names()
    }

    fn get_identifier_field_names(&self) -> Vec<String> {
        self.read().identifier.clone()
    }

    fn get_association_names(&self) -> Vec<String> {
        self.read().get_association_names()
    }

    fn get_type_of_field(&self, field_name: &str) -> Result<Option<String>, Thrown> {
        Ok(self.read().get_type_of_field(field_name))
    }

    fn get_association_target_class(&self, assoc_name: &str) -> Result<Option<String>, Thrown> {
        self.read()
            .get_association_target_class(assoc_name)
            .map(Some)
    }

    fn is_association_inverse_side(&self, assoc_name: &str) -> bool {
        self.read().is_association_inverse_side(assoc_name)
    }

    fn get_association_mapped_by_target_field(&self, assoc_name: &str) -> Result<String, Thrown> {
        self.read()
            .get_association_mapped_by_target_field(assoc_name)
    }

    fn get_identifier_values(&self, object: &Object) -> Result<PhpArray, Thrown> {
        self.read().get_identifier_values(object)
    }
}

/// The properties and methods of `Doctrine\ORM\Mapping\ClassMetadata`.
#[derive(Debug, Clone)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "ClassMetadata's public flags, as PHP declares them"
)]
pub struct ClassMetadataInfo {
    /// The name of the entity class.
    pub name: String,
    /// The namespace the entity class is contained in.
    pub namespace: Option<String>,
    /// The name of the entity class that is at the root of the mapped
    /// entity inheritance hierarchy.
    pub root_entity_name: String,
    /// The definition of custom generator.
    pub custom_generator_definition: Option<PhpArray>,
    /// The name of the custom repository class used for the entity class.
    pub custom_repository_class_name: Option<String>,
    pub is_mapped_superclass: bool,
    pub is_embedded_class: bool,
    /// The names of the parent *entity* classes (ancestors), starting
    /// with the nearest one and ending with the root entity class.
    pub parent_classes: Vec<String>,
    /// The names of all *entity* subclasses of the class.
    pub sub_classes: Vec<String>,
    /// The names of all embedded classes based on properties.
    pub embedded_classes: OrderedMap<EmbeddedClassMapping>,
    /// The field names of all fields that are part of the identifier/primary
    /// key of the mapped entity class.
    pub identifier: Vec<String>,
    /// The position of each identifier field.
    pub identifier_positions: OrderedMap<i64>,
    pub inheritance_type: i64,
    pub generator_type: i64,
    /// The field mappings of the class.
    pub field_mappings: OrderedMap<FieldMapping>,
    /// An array of field names, used to look up field names from column
    /// names.
    pub field_names: OrderedMap<String>,
    /// A map of field names to column names.
    pub column_names: OrderedMap<String>,
    /// The discriminator value of this class.
    pub discriminator_value: Value,
    /// The discriminator map of all mapped classes in the hierarchy.
    pub discriminator_map: PhpArray,
    pub discriminator_column: Option<DiscriminatorColumnMapping>,
    /// The primary table definition (`name`, `schema`, `indexes`,
    /// `uniqueConstraints`, `options`, `quoted`); empty until set.
    pub table: PhpArray,
    /// The registered lifecycle callbacks for entities of this class.
    pub lifecycle_callbacks: OrderedMap<Vec<String>>,
    /// The registered entity listeners.
    pub entity_listeners: OrderedMap<Vec<EntityListener>>,
    /// The association mappings of this class.
    pub association_mappings: OrderedMap<AssociationMapping>,
    pub is_identifier_composite: bool,
    pub contains_foreign_identifier: bool,
    pub contains_enum_identifier: bool,
    /// The ID generator used for generating IDs for this class (an
    /// uninitialized typed property until set).
    pub id_generator: Option<IdGenerator>,
    /// The definition of the sequence generator of this class.
    pub sequence_generator_definition: Option<PhpArray>,
    pub change_tracking_policy: i64,
    pub requires_fetch_after_change: bool,
    pub is_versioned: bool,
    pub version_field: Option<String>,
    pub cache: Option<PhpArray>,
    /// The `ReflectionClass` instance of the mapped class.
    pub refl_class: Option<ReflectionClass>,
    pub is_read_only: bool,
    naming_strategy: Arc<dyn NamingStrategy>,
    /// The accessors of the mapped properties.
    pub property_accessors: OrderedMap<Arc<dyn PropertyAccessor>>,
    typed_field_mapper: Arc<dyn TypedFieldMapper>,
}

fn text_of(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) => Some(pv::text(s)),
        Value::Int(i) => Some(i.to_string()),
        _ => None,
    }
}

fn int_of(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Int(i) => Some(*i),
        Value::Bool(b) => Some(i64::from(*b)),
        _ => None,
    }
}

/// `$value === true`.
fn is_true(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Bool(true)))
}

/// PHP truthiness.
fn truthy(value: Option<&Value>) -> bool {
    !pv::empty(value)
}

fn logic_exception(message: String) -> Thrown {
    Thrown::new("LogicException", message, 0)
}

impl ClassMetadataInfo {
    /// Initializes a new `ClassMetadata` instance that will hold the
    /// object-relational mapping metadata of the class with the given name.
    pub fn new(
        name: impl Into<String>,
        naming_strategy: Option<Arc<dyn NamingStrategy>>,
        typed_field_mapper: Option<Arc<dyn TypedFieldMapper>>,
    ) -> Self {
        crate::mapping::register_orm_classes();
        let name = name.into();
        Self {
            root_entity_name: name.clone(),
            name,
            namespace: None,
            custom_generator_definition: None,
            custom_repository_class_name: None,
            is_mapped_superclass: false,
            is_embedded_class: false,
            parent_classes: Vec::new(),
            sub_classes: Vec::new(),
            embedded_classes: OrderedMap::new(),
            identifier: Vec::new(),
            identifier_positions: OrderedMap::new(),
            inheritance_type: INHERITANCE_TYPE_NONE,
            generator_type: GENERATOR_TYPE_NONE,
            field_mappings: OrderedMap::new(),
            field_names: OrderedMap::new(),
            column_names: OrderedMap::new(),
            discriminator_value: Value::Null,
            discriminator_map: PhpArray::new(),
            discriminator_column: None,
            table: PhpArray::new(),
            lifecycle_callbacks: OrderedMap::new(),
            entity_listeners: OrderedMap::new(),
            association_mappings: OrderedMap::new(),
            is_identifier_composite: false,
            contains_foreign_identifier: false,
            contains_enum_identifier: false,
            id_generator: None,
            sequence_generator_definition: None,
            change_tracking_policy: CHANGETRACKING_DEFERRED_IMPLICIT,
            requires_fetch_after_change: false,
            is_versioned: false,
            version_field: None,
            cache: None,
            refl_class: None,
            is_read_only: false,
            naming_strategy: naming_strategy
                .unwrap_or_else(|| Arc::new(DefaultNamingStrategy::new())),
            property_accessors: OrderedMap::new(),
            typed_field_mapper: typed_field_mapper
                .unwrap_or_else(|| Arc::new(DefaultTypedFieldMapper::default())),
        }
    }

    pub fn naming_strategy(&self) -> &Arc<dyn NamingStrategy> {
        &self.naming_strategy
    }

    /// `getReflectionClass()`.
    pub fn get_reflection_class(&self) -> Option<&ReflectionClass> {
        self.refl_class.as_ref()
    }

    /// `getPropertyAccessors()`.
    pub fn get_property_accessors(&self) -> &OrderedMap<Arc<dyn PropertyAccessor>> {
        &self.property_accessors
    }

    /// `getPropertyAccessor()`.
    pub fn get_property_accessor(&self, name: &str) -> Option<Arc<dyn PropertyAccessor>> {
        self.property_accessors.get(name).cloned()
    }

    fn composite_identifier(&self) -> Thrown {
        Thrown::new(
            "BadMethodCallException",
            format!("Class {} has a composite identifier.", self.name),
            0,
        )
    }

    /// `getSingleIdPropertyAccessor()`.
    pub fn get_single_id_property_accessor(
        &self,
    ) -> Result<Option<Arc<dyn PropertyAccessor>>, Thrown> {
        if self.is_identifier_composite {
            return Err(self.composite_identifier());
        }
        Ok(self
            .identifier
            .first()
            .and_then(|id| self.property_accessors.get(id).cloned()))
    }

    fn accessor(&self, field: &str) -> Result<Arc<dyn PropertyAccessor>, Thrown> {
        self.property_accessors.get(field).cloned().ok_or_else(|| {
            Thrown::new(
                "Error",
                format!("Call to a member function getValue() on null (no property accessor for {}::${field})", self.name),
                0,
            )
        })
    }

    /// `getIdentifierValues()`: the identifier values of the entity, by
    /// field; null values are left out.
    pub fn get_identifier_values(&self, entity: &Object) -> Result<PhpArray, Thrown> {
        let mut id = PhpArray::new();
        if self.is_identifier_composite {
            for id_field in &self.identifier {
                let value = self.accessor(id_field)?.get_value(entity)?;
                if !matches!(value, Value::Null) {
                    id.insert(id_field.as_str(), value);
                }
            }
            return Ok(id);
        }
        let Some(field) = self.identifier.first() else {
            return Ok(id);
        };
        let value = self.accessor(field)?.get_value(entity)?;
        if !matches!(value, Value::Null) {
            id.insert(field.as_str(), value);
        }
        Ok(id)
    }

    /// `setIdentifierValues()`.
    pub fn set_identifier_values(&self, entity: &Object, id: &PhpArray) -> Result<(), Thrown> {
        for (field, value) in id.iter() {
            self.accessor(&pv::key_name(field))?
                .set_value(entity, value.clone())?;
        }
        Ok(())
    }

    /// `setFieldValue()`.
    pub fn set_field_value(
        &self,
        entity: &Object,
        field: &str,
        value: Value,
    ) -> Result<(), Thrown> {
        self.accessor(field)?.set_value(entity, value)
    }

    /// `getFieldValue()`.
    pub fn get_field_value(&self, entity: &Object, field: &str) -> Result<Value, Thrown> {
        self.accessor(field)?.get_value(entity)
    }

    /// `newInstance()`: an instance of the class, without calling its
    /// constructor.
    pub fn new_instance(&self) -> Result<Object, Thrown> {
        EntityObject::object(&self.name)
            .ok_or_else(|| php_reflection::class_does_not_exist(&self.name))
    }

    /// `wakeupReflection()`: restores the property accessors after
    /// unserialization.
    pub fn wakeup_reflection(
        &mut self,
        refl_service: &dyn ReflectionService,
    ) -> Result<(), Thrown> {
        self.refl_class = Some(refl_service.get_class(&self.name)?);
        let mut parent_accessors: OrderedMap<Arc<dyn PropertyAccessor>> = OrderedMap::new();
        let embedded: Vec<(String, EmbeddedClassMapping)> = self
            .embedded_classes
            .iter()
            .map(|(k, v)| (k.to_owned(), v.clone()))
            .collect();
        for (property, embedded_class) in &embedded {
            if let Some(declared_field) = &embedded_class.declared_field {
                let parent_class = self
                    .embedded_classes
                    .get(declared_field)
                    .map(|e| e.class.clone())
                    .unwrap_or_default();
                let child = create_property_accessor(
                    &parent_class,
                    embedded_class.original_field.as_deref().unwrap_or_default(),
                )?;
                let parent = parent_accessors
                    .get(declared_field)
                    .cloned()
                    .ok_or_else(|| logic_exception(format!("No accessor for {declared_field}")))?;
                parent_accessors.insert(
                    property.clone(),
                    Arc::new(EmbeddablePropertyAccessor::new(parent, child, parent_class)),
                );
                continue;
            }
            let accessor = create_property_accessor(
                embedded_class.declared.as_deref().unwrap_or(&self.name),
                property,
            )?;
            parent_accessors.insert(property.clone(), accessor.clone());
            self.property_accessors.insert(property.clone(), accessor);
        }
        let fields: Vec<(String, FieldMapping)> = self
            .field_mappings
            .iter()
            .map(|(k, v)| (k.to_owned(), v.clone()))
            .collect();
        for (field, mapping) in &fields {
            if let Some(parent) = mapping
                .declared_field
                .as_ref()
                .and_then(|declared| parent_accessors.get(declared))
            {
                let original_class = mapping.original_class.clone().unwrap_or_default();
                let mut accessor = create_property_accessor(
                    &original_class,
                    mapping.original_field.as_deref().unwrap_or_default(),
                )?;
                if let Some(enum_type) = &mapping.enum_type {
                    accessor = Arc::new(EnumPropertyAccessor::new(accessor, enum_type.clone()));
                }
                self.property_accessors.insert(
                    field.clone(),
                    Arc::new(EmbeddablePropertyAccessor::new(
                        parent.clone(),
                        accessor,
                        original_class,
                    )),
                );
                continue;
            }
            let mut accessor =
                create_property_accessor(mapping.declared.as_deref().unwrap_or(&self.name), field)?;
            if let Some(enum_type) = &mapping.enum_type {
                accessor = Arc::new(EnumPropertyAccessor::new(accessor, enum_type.clone()));
            }
            self.property_accessors.insert(field.clone(), accessor);
        }
        let associations: Vec<(String, Option<String>)> = self
            .association_mappings
            .iter()
            .map(|(k, v)| (k.to_owned(), v.declared.clone()))
            .collect();
        for (field, declared) in associations {
            let accessor =
                create_property_accessor(declared.as_deref().unwrap_or(&self.name), &field)?;
            self.property_accessors.insert(field, accessor);
        }
        Ok(())
    }

    /// `initializeReflection()`: initializes a new `ClassMetadata`
    /// instance from the reflection service.
    pub fn initialize_reflection(
        &mut self,
        refl_service: &dyn ReflectionService,
    ) -> Result<(), Thrown> {
        let class = refl_service.get_class(&self.name).ok();
        self.namespace = refl_service.get_class_namespace(&self.name).ok();
        if let Some(class) = &class {
            self.name = class.get_name().to_owned();
            self.root_entity_name = self.name.clone();
        }
        self.refl_class = class;
        let table_name = self.naming_strategy.class_to_table_name(&self.name);
        self.table.insert("name", Value::from(table_name));
        Ok(())
    }

    /// `validateIdentifier()`.
    pub fn validate_identifier(&self) -> Result<(), Thrown> {
        if self.is_mapped_superclass || self.is_embedded_class {
            return Ok(());
        }
        if self.identifier.is_empty() {
            return Err(mapping_exception::identifier_required(&self.name));
        }
        if self.uses_id_generator() && self.is_identifier_composite {
            return Err(
                mapping_exception::composite_key_assigned_id_generator_required(&self.name),
            );
        }
        Ok(())
    }

    /// `validateAssociations()`.
    pub fn validate_associations(&self) -> Result<(), Thrown> {
        for mapping in self.association_mappings.values() {
            if php_reflection::class_info(&mapping.target_entity).is_none() {
                return Err(mapping_exception::invalid_target_entity_class(
                    &mapping.target_entity,
                    &self.name,
                    &mapping.field_name,
                ));
            }
        }
        Ok(())
    }

    /// `validateLifecycleCallbacks()`.
    pub fn validate_lifecycle_callbacks(
        &self,
        refl_service: &dyn ReflectionService,
    ) -> Result<(), Thrown> {
        for callbacks in self.lifecycle_callbacks.values() {
            for callback_func_name in callbacks {
                if !refl_service.has_public_method(&self.name, callback_func_name) {
                    return Err(mapping_exception::lifecycle_callback_method_not_found(
                        &self.name,
                        callback_func_name,
                    ));
                }
            }
        }
        Ok(())
    }

    fn region(&self) -> String {
        self.root_entity_name
            .replace('\\', "_")
            .to_ascii_lowercase()
    }

    /// `enableCache()`.
    pub fn enable_cache(&mut self, mut cache: PhpArray) {
        if !pv::isset(&cache, "usage") {
            cache.insert("usage", Value::Int(CACHE_USAGE_READ_ONLY));
        }
        if !pv::isset(&cache, "region") {
            cache.insert("region", Value::from(self.region()));
        }
        self.cache = Some(cache);
    }

    /// `enableAssociationCache()`.
    pub fn enable_association_cache(&mut self, field_name: &str, cache: PhpArray) {
        let cache = self.get_association_cache_defaults(field_name, cache);
        if let Some(mapping) = self.association_mappings.get_mut(field_name) {
            mapping.cache = Some(cache);
        }
    }

    /// `getAssociationCacheDefaults()`.
    pub fn get_association_cache_defaults(
        &self,
        field_name: &str,
        mut cache: PhpArray,
    ) -> PhpArray {
        if !pv::isset(&cache, "usage") {
            let usage = self
                .cache
                .as_ref()
                .and_then(|c| pv::get(c, "usage").cloned())
                .filter(|v| !matches!(v, Value::Null))
                .unwrap_or(Value::Int(CACHE_USAGE_READ_ONLY));
            cache.insert("usage", usage);
        }
        if !pv::isset(&cache, "region") {
            cache.insert(
                "region",
                Value::from(format!("{}__{field_name}", self.region())),
            );
        }
        cache
    }

    pub fn set_change_tracking_policy(&mut self, policy: i64) {
        self.change_tracking_policy = policy;
    }

    pub fn is_change_tracking_deferred_explicit(&self) -> bool {
        self.change_tracking_policy == CHANGETRACKING_DEFERRED_EXPLICIT
    }

    pub fn is_change_tracking_deferred_implicit(&self) -> bool {
        self.change_tracking_policy == CHANGETRACKING_DEFERRED_IMPLICIT
    }

    /// `isIdentifier()`.
    pub fn is_identifier(&self, field_name: &str) -> bool {
        if self.identifier.is_empty() {
            return false;
        }
        if !self.is_identifier_composite {
            return self.identifier[0] == field_name;
        }
        self.identifier.iter().any(|id| id == field_name)
    }

    /// `isUniqueField()`.
    pub fn is_unique_field(&self, field_name: &str) -> Result<bool, Thrown> {
        Ok(self.get_field_mapping(field_name)?.unique == Some(true))
    }

    /// `isNullable()`.
    pub fn is_nullable(&self, field_name: &str) -> Result<bool, Thrown> {
        Ok(self.get_field_mapping(field_name)?.nullable == Some(true))
    }

    /// `isIndexed()`.
    pub fn is_indexed(&self, field_name: &str) -> Result<bool, Thrown> {
        Ok(self.get_field_mapping(field_name)?.index == Some(true))
    }

    /// `getColumnName()`.
    pub fn get_column_name(&self, field_name: &str) -> String {
        self.column_names
            .get(field_name)
            .cloned()
            .unwrap_or_else(|| field_name.to_owned())
    }

    /// `getFieldMapping()`.
    pub fn get_field_mapping(&self, field_name: &str) -> Result<&FieldMapping, Thrown> {
        self.field_mappings
            .get(field_name)
            .ok_or_else(|| mapping_exception::mapping_not_found(&self.name, field_name))
    }

    /// `getAssociationMapping()`.
    pub fn get_association_mapping(&self, field_name: &str) -> Result<&AssociationMapping, Thrown> {
        self.association_mappings
            .get(field_name)
            .ok_or_else(|| mapping_exception::mapping_not_found(&self.name, field_name))
    }

    /// `getAssociationMappings()`.
    pub fn get_association_mappings(&self) -> &OrderedMap<AssociationMapping> {
        &self.association_mappings
    }

    /// `getFieldName()`.
    pub fn get_field_name(&self, column_name: &str) -> String {
        self.field_names
            .get(column_name)
            .cloned()
            .unwrap_or_else(|| column_name.to_owned())
    }

    fn is_typed_property(&self, name: &str) -> bool {
        self.refl_class.as_ref().is_some_and(|class| {
            class.has_property(name) && class.get_property(name).is_ok_and(|p| p.has_type())
        })
    }

    fn validate_and_complete_typed_field_mapping(
        &self,
        mapping: PhpArray,
    ) -> Result<PhpArray, Thrown> {
        let Some(class) = &self.refl_class else {
            return Ok(mapping);
        };
        let field =
            class.get_property(&text_of(pv::get(&mapping, "fieldName")).unwrap_or_default())?;
        self.typed_field_mapper
            .validate_and_complete(mapping, &field)
    }

    fn validate_and_complete_typed_association_mapping(
        &self,
        mut mapping: PhpArray,
    ) -> Result<PhpArray, Thrown> {
        let Some(class) = &self.refl_class else {
            return Ok(mapping);
        };
        let field =
            class.get_property(&text_of(pv::get(&mapping, "fieldName")).unwrap_or_default())?;
        let Some(ty) = field.get_type() else {
            return Ok(mapping);
        };
        if int_of(pv::get(&mapping, "type")).unwrap_or(0) & TO_ONE == 0 {
            return Ok(mapping);
        }
        if !pv::isset(&mapping, "targetEntity") && !ty.get_name().contains(['|', '&']) {
            mapping.insert("targetEntity", Value::from(ty.get_name()));
        }
        Ok(mapping)
    }

    /// `validateAndCompleteFieldMapping()`: validates and completes the
    /// field mapping.
    pub fn validate_and_complete_field_mapping(
        &mut self,
        mut mapping: PhpArray,
    ) -> Result<FieldMapping, Thrown> {
        if !truthy(pv::get(&mapping, "fieldName")) {
            return Err(mapping_exception::missing_field_name(&self.name));
        }
        let field_name = text_of(pv::get(&mapping, "fieldName")).unwrap_or_default();
        if self.is_typed_property(&field_name) {
            mapping = self.validate_and_complete_typed_field_mapping(mapping)?;
        }
        if !pv::isset(&mapping, "type") {
            mapping.insert("type", Value::from("string"));
        }
        if !pv::isset(&mapping, "columnName") {
            mapping.insert(
                "columnName",
                Value::from(
                    self.naming_strategy
                        .property_to_column_name(&field_name, &self.name),
                ),
            );
        }
        let mut mapping = FieldMapping::from_mapping_array(&mapping)?;
        if mapping.column_name.starts_with('`') {
            mapping.column_name = mapping.column_name.trim_matches('`').to_owned();
            mapping.quoted = Some(true);
        }
        self.column_names
            .insert(mapping.field_name.clone(), mapping.column_name.clone());
        if self.field_names.contains_key(&mapping.column_name)
            || self
                .discriminator_column
                .as_ref()
                .is_some_and(|column| column.name == mapping.column_name)
        {
            return Err(mapping_exception::duplicate_column_name(
                &self.name,
                &mapping.column_name,
            ));
        }
        self.field_names
            .insert(mapping.column_name.clone(), mapping.field_name.clone());
        if mapping.id == Some(true) {
            if self.version_field.as_deref() == Some(mapping.field_name.as_str()) {
                return Err(mapping_exception::cannot_version_id_field(
                    &self.name,
                    &mapping.field_name,
                ));
            }
            if !self.identifier.contains(&mapping.field_name) {
                self.register_identifier_field(&mapping.field_name, mapping.id_position);
            }
            if !self.is_identifier_composite && self.identifier.len() > 1 {
                self.is_identifier_composite = true;
            }
        }
        if let Some(generated) = mapping.generated {
            if ![GENERATED_NEVER, GENERATED_INSERT, GENERATED_ALWAYS].contains(&generated) {
                return Err(mapping_exception::invalid_generated_mode(
                    &generated.to_string(),
                ));
            }
            if generated == GENERATED_NEVER {
                mapping.generated = None;
            }
        }
        if let Some(enum_type) = mapping.enum_type.clone() {
            if !enum_exists(&enum_type) {
                return Err(mapping_exception::non_enum_type_mapped(
                    &self.name,
                    &mapping.field_name,
                    &enum_type,
                ));
            }
            if mapping.id == Some(true) {
                self.contains_enum_identifier = true;
            }
            let has_values = mapping
                .options
                .as_ref()
                .is_some_and(|options| pv::isset(options, "values"));
            if mapping.r#type == "enum" && !has_values {
                mapping
                    .options
                    .get_or_insert_with(PhpArray::new)
                    .insert("values", Value::Array(enum_values(&enum_type)));
            }
        }
        Ok(mapping)
    }

    /// `_validateAndCompleteAssociationMapping()`: validates and completes
    /// the basic mapping information that is common to all association
    /// mappings.
    #[allow(clippy::too_many_lines, reason = "one PHP method")]
    pub fn validate_and_complete_association_mapping(
        &mut self,
        mut mapping: PhpArray,
    ) -> Result<AssociationMapping, Thrown> {
        for key in ["mappedBy", "inversedBy"] {
            if matches!(pv::get(&mapping, key), Some(Value::Null)) {
                mapping.remove(&Key::from(key));
            }
        }
        if matches!(pv::get(&mapping, "joinColumns"), Some(Value::Null))
            || matches!(pv::get(&mapping, "joinColumns"), Some(Value::Array(a)) if a.is_empty())
        {
            mapping.remove(&Key::from("joinColumns"));
        }
        // assume owning side until we hit mappedBy
        let mut is_owning_side = true;
        if pv::empty(pv::get(&mapping, "indexBy")) {
            mapping.remove(&Key::from("indexBy"));
        }
        mapping.insert("sourceEntity", Value::from(self.name.as_str()));
        let field_name = text_of(pv::get(&mapping, "fieldName")).unwrap_or_default();
        if self.is_typed_property(&field_name) {
            mapping = self.validate_and_complete_typed_association_mapping(mapping)?;
        }
        if let Some(target_entity) = text_of(pv::get(&mapping, "targetEntity")) {
            let target_entity = self.fully_qualified_class_name(&target_entity);
            mapping.insert(
                "targetEntity",
                Value::from(target_entity.trim_start_matches('\\')),
            );
        }
        let ty = int_of(pv::get(&mapping, "type")).unwrap_or(0);
        if ty & MANY_TO_ONE > 0 && truthy(pv::get(&mapping, "orphanRemoval")) {
            return Err(mapping_exception::illegal_orphan_removal(
                &self.name,
                &field_name,
            ));
        }
        if is_true(pv::get(&mapping, "id")) {
            if truthy(pv::get(&mapping, "orphanRemoval")) {
                return Err(
                    mapping_exception::illegal_orphan_removal_on_identifier_association(
                        &self.name,
                        &field_name,
                    ),
                );
            }
            if !self.identifier.contains(&field_name) {
                if matches!(pv::get(&mapping, "joinColumns"), Some(Value::Array(c)) if c.len() >= 2)
                {
                    return Err(
                        mapping_exception::cannot_map_composite_primary_key_entities_as_foreign_id(
                            &text_of(pv::get(&mapping, "targetEntity")).unwrap_or_default(),
                            &self.name,
                            &field_name,
                        ),
                    );
                }
                self.register_identifier_field(
                    &field_name,
                    int_of(pv::get(&mapping, "idPosition")),
                );
                self.contains_foreign_identifier = true;
            }
            if !self.is_identifier_composite && self.identifier.len() > 1 {
                self.is_identifier_composite = true;
            }
            if self.cache.as_ref().is_some_and(|c| !c.is_empty()) && !pv::isset(&mapping, "cache") {
                return Err(Thrown::new(
                    "Doctrine\\ORM\\Cache\\Exception\\NonCacheableEntityAssociation",
                    format!(
                        "Entity association field \"{}#{field_name}\" not configured as part of the second-level cache.",
                        self.name
                    ),
                    0,
                ));
            }
        }
        mapping.remove(&Key::from("idPosition"));
        if !truthy(pv::get(&mapping, "fieldName")) {
            return Err(mapping_exception::missing_field_name(&self.name));
        }
        if !pv::isset(&mapping, "targetEntity") {
            return Err(mapping_exception::missing_target_entity(&field_name));
        }
        if pv::isset(&mapping, "mappedBy") {
            is_owning_side = false;
        } else if let Some(Value::Array(join_table)) = mapping.get_mut(&Key::from("joinTable"))
            && let Some(name) = text_of(pv::get(join_table, "name"))
            && name.starts_with('`')
        {
            join_table.insert("name", Value::from(name.trim_matches('`')));
            join_table.insert("quoted", Value::Bool(true));
        }
        mapping.insert("isOwningSide", Value::Bool(is_owning_side));
        if is_true(pv::get(&mapping, "id")) && ty & TO_MANY != 0 {
            return Err(mapping_exception::illegal_to_many_identifier_association(
                &self.name,
                &field_name,
            ));
        }
        if !pv::isset(&mapping, "fetch") {
            mapping.insert("fetch", Value::Int(FETCH_LAZY));
        }
        let cascades: Vec<String> = match pv::get(&mapping, "cascade") {
            Some(Value::Array(cascade)) if pv::isset(&mapping, "cascade") => cascade
                .values()
                .filter_map(|v| v.to_php_string().map(|s| pv::text(&s).to_ascii_lowercase()))
                .collect(),
            _ => Vec::new(),
        };
        let all_cascades = ["remove", "persist", "refresh", "detach"];
        let cascades: Vec<String> = if cascades.iter().any(|c| c == "all") {
            all_cascades.iter().map(|c| (*c).to_owned()).collect()
        } else {
            let invalid: Vec<String> = cascades
                .iter()
                .filter(|c| !all_cascades.contains(&c.as_str()))
                .cloned()
                .collect();
            if !invalid.is_empty() {
                return Err(mapping_exception::invalid_cascade_option(
                    &invalid,
                    &self.name,
                    &field_name,
                ));
            }
            cascades
        };
        mapping.insert("cascade", pv::from_strings(&cascades));
        match ty {
            ONE_TO_ONE => {
                if truthy(pv::get(&mapping, "joinColumns")) && !is_owning_side {
                    return Err(
                        mapping_exception::join_column_not_allowed_on_one_to_one_inverse_side(
                            &self.name,
                            &field_name,
                        ),
                    );
                }
                if is_owning_side {
                    AssociationMapping::to_one_owning_side_from_mapping_array_and_name(
                        AssociationKind::OneToOneOwningSide,
                        &mapping,
                        self.naming_strategy.as_ref(),
                        &self.name,
                        !self.table.is_empty(),
                        self.is_inheritance_type_single_table(),
                    )
                } else {
                    AssociationMapping::to_one_inverse_side_from_mapping_array_and_name(
                        &mapping, &self.name,
                    )
                }
            }
            MANY_TO_ONE => AssociationMapping::to_one_owning_side_from_mapping_array_and_name(
                AssociationKind::ManyToOne,
                &mapping,
                self.naming_strategy.as_ref(),
                &self.name,
                !self.table.is_empty(),
                self.is_inheritance_type_single_table(),
            ),
            ONE_TO_MANY => {
                AssociationMapping::one_to_many_from_mapping_array_and_name(&mapping, &self.name)
            }
            MANY_TO_MANY => {
                mapping.remove(&Key::from("joinColumns"));
                if is_owning_side {
                    AssociationMapping::many_to_many_owning_side_from_mapping_array_and_naming_strategy(
                        &mapping,
                        self.naming_strategy.as_ref(),
                    )
                } else {
                    AssociationMapping::from_mapping_array(
                        AssociationClass::Final(AssociationKind::ManyToManyInverseSide),
                        &mapping,
                    )
                }
            }
            _ => Err(mapping_exception::invalid_association_type(
                &self.name,
                &field_name,
                ty,
            )),
        }
    }

    /// `getIdentifierFieldNames()`.
    pub fn get_identifier_field_names(&self) -> &[String] {
        &self.identifier
    }

    /// `getSingleIdentifierFieldName()`.
    pub fn get_single_identifier_field_name(&self) -> Result<String, Thrown> {
        if self.is_identifier_composite {
            return Err(
                mapping_exception::single_id_not_allowed_on_composite_primary_key(&self.name),
            );
        }
        self.identifier
            .first()
            .cloned()
            .ok_or_else(|| mapping_exception::no_id_defined(&self.name))
    }

    /// `getSingleIdentifierColumnName()`.
    pub fn get_single_identifier_column_name(&self) -> Result<String, Thrown> {
        Ok(self.get_column_name(&self.get_single_identifier_field_name()?))
    }

    /// `setIdentifier()`, with the positions of the fields.
    pub fn set_identifier(&mut self, identifier: &[String], positions: &[(String, i64)]) {
        self.identifier = Vec::new();
        self.identifier_positions = OrderedMap::new();
        for field_name in identifier {
            let position = positions
                .iter()
                .find(|(name, _)| name == field_name)
                .map(|(_, p)| *p);
            self.register_identifier_field(field_name, position);
        }
        self.is_identifier_composite = self.identifier.len() > 1;
    }

    /// `getIdentifier()`.
    pub fn get_identifier(&self) -> &[String] {
        &self.identifier
    }

    /// `hasField()`.
    pub fn has_field(&self, field_name: &str) -> bool {
        self.field_mappings.contains_key(field_name)
            || self.embedded_classes.contains_key(field_name)
    }

    /// `getColumnNames()`.
    pub fn get_column_names(&self, field_names: Option<&[String]>) -> Vec<String> {
        match field_names {
            None => self.field_names.keys().map(str::to_owned).collect(),
            Some(field_names) => field_names
                .iter()
                .map(|f| self.get_column_name(f))
                .collect(),
        }
    }

    /// `getIdentifierColumnNames()`.
    pub fn get_identifier_column_names(&self) -> Vec<String> {
        let mut column_names = Vec::new();
        for id_property in &self.identifier {
            if let Some(mapping) = self.field_mappings.get(id_property) {
                column_names.push(mapping.column_name.clone());
                continue;
            }
            if let Some(association) = self.association_mappings.get(id_property) {
                column_names.extend(association.join_columns.iter().map(|c| c.name.clone()));
            }
        }
        column_names
    }

    pub fn set_id_generator_type(&mut self, generator_type: i64) {
        self.generator_type = generator_type;
    }

    pub fn uses_id_generator(&self) -> bool {
        self.generator_type != GENERATOR_TYPE_NONE
    }

    pub fn is_inheritance_type_none(&self) -> bool {
        self.inheritance_type == INHERITANCE_TYPE_NONE
    }

    pub fn is_inheritance_type_joined(&self) -> bool {
        self.inheritance_type == INHERITANCE_TYPE_JOINED
    }

    pub fn is_inheritance_type_single_table(&self) -> bool {
        self.inheritance_type == INHERITANCE_TYPE_SINGLE_TABLE
    }

    pub fn is_id_generator_identity(&self) -> bool {
        self.generator_type == GENERATOR_TYPE_IDENTITY
    }

    pub fn is_id_generator_sequence(&self) -> bool {
        self.generator_type == GENERATOR_TYPE_SEQUENCE
    }

    pub fn is_identifier_natural(&self) -> bool {
        self.generator_type == GENERATOR_TYPE_NONE
    }

    /// `getTypeOfField()`.
    pub fn get_type_of_field(&self, field_name: &str) -> Option<String> {
        self.field_mappings
            .get(field_name)
            .map(|m| m.r#type.clone())
    }

    /// `getTableName()`.
    pub fn get_table_name(&self) -> Result<String, Thrown> {
        text_of(pv::get(&self.table, "name")).ok_or_else(|| {
            Thrown::new(
                "Error",
                format!(
                    "Typed property {CLASS}::$table must not be accessed before initialization"
                ),
                0,
            )
        })
    }

    /// `getSchemaName()`.
    pub fn get_schema_name(&self) -> Option<String> {
        text_of(pv::get(&self.table, "schema"))
    }

    /// `getTemporaryIdTableName()`.
    pub fn get_temporary_id_table_name(&self) -> Result<String, Thrown> {
        Ok(format!("{}_id_tmp", self.get_table_name()?).replace('.', "_"))
    }

    /// `setSubclasses()`.
    pub fn set_subclasses(&mut self, subclasses: &[String]) {
        for subclass in subclasses {
            let subclass = self.fully_qualified_class_name(subclass);
            self.sub_classes.push(subclass);
        }
    }

    /// `setParentClasses()`.
    pub fn set_parent_classes(&mut self, class_names: Vec<String>) {
        if let Some(root) = class_names.last() {
            self.root_entity_name.clone_from(root);
        }
        self.parent_classes = class_names;
    }

    /// `setInheritanceType()`.
    pub fn set_inheritance_type(&mut self, ty: i64) -> Result<(), Thrown> {
        if ![
            INHERITANCE_TYPE_NONE,
            INHERITANCE_TYPE_SINGLE_TABLE,
            INHERITANCE_TYPE_JOINED,
        ]
        .contains(&ty)
        {
            return Err(mapping_exception::invalid_inheritance_type(&self.name, ty));
        }
        self.inheritance_type = ty;
        Ok(())
    }

    /// `setAssociationOverride()`.
    pub fn set_association_override(
        &mut self,
        field_name: &str,
        override_mapping: &PhpArray,
    ) -> Result<(), Thrown> {
        let Some(association) = self.association_mappings.get(field_name) else {
            return Err(mapping_exception::invalid_override_field_name(
                &self.name, field_name,
            ));
        };
        let mut mapping = association.to_array()?;
        if let Some(inherited) = text_of(pv::get(&mapping, "inherited")) {
            return Err(mapping_exception::illegal_override_of_inherited_property(
                &self.name, field_name, &inherited,
            ));
        }
        for key in ["joinColumns", "inversedBy", "joinTable", "fetch", "cascade"] {
            if pv::isset(override_mapping, key) {
                mapping.insert(key, pv::required(override_mapping, key).clone());
            }
        }
        match int_of(pv::get(&mapping, "type")) {
            Some(ONE_TO_ONE | MANY_TO_ONE) => {
                mapping.insert("joinColumnFieldNames", Value::Array(PhpArray::new()));
                mapping.insert("sourceToTargetKeyColumns", Value::Array(PhpArray::new()));
            }
            Some(MANY_TO_MANY) => {
                mapping.insert("relationToSourceKeyColumns", Value::Array(PhpArray::new()));
                mapping.insert("relationToTargetKeyColumns", Value::Array(PhpArray::new()));
            }
            _ => {}
        }
        // `toArray()` gives the join columns and the join table as arrays,
        // which the mapping arrays take.
        let completed = self.validate_and_complete_association_mapping(mapping)?;
        self.association_mappings.insert(field_name, completed);
        Ok(())
    }

    /// `setAttributeOverride()`.
    pub fn set_attribute_override(
        &mut self,
        field_name: &str,
        override_mapping: &PhpArray,
    ) -> Result<(), Thrown> {
        let Some(mapping) = self.field_mappings.get(field_name).cloned() else {
            return Err(mapping_exception::invalid_override_field_name(
                &self.name, field_name,
            ));
        };
        if let Some(inherited) = &mapping.inherited {
            return Err(mapping_exception::illegal_override_of_inherited_property(
                &self.name, field_name, inherited,
            ));
        }
        let mut override_mapping = override_mapping.clone();
        if let Some(id) = mapping.id {
            override_mapping.insert("id", Value::Bool(id));
        }
        if let Some(declared) = &mapping.declared {
            override_mapping.insert("declared", Value::from(declared.as_str()));
        }
        if !pv::isset(&override_mapping, "type") {
            override_mapping.insert("type", Value::from(mapping.r#type.as_str()));
        }
        if !pv::isset(&override_mapping, "fieldName") {
            override_mapping.insert("fieldName", Value::from(mapping.field_name.as_str()));
        }
        if text_of(pv::get(&override_mapping, "type")).as_deref() != Some(mapping.r#type.as_str()) {
            return Err(mapping_exception::invalid_override_field_type(
                &self.name, field_name,
            ));
        }
        self.field_mappings.remove(field_name);
        self.field_names.remove(&mapping.column_name);
        self.column_names.remove(&mapping.field_name);
        let override_mapping = self.validate_and_complete_field_mapping(override_mapping)?;
        self.field_mappings.insert(field_name, override_mapping);
        Ok(())
    }

    /// `isInheritedField()`.
    pub fn is_inherited_field(&self, field_name: &str) -> bool {
        self.field_mappings
            .get(field_name)
            .is_some_and(|m| m.inherited.is_some())
    }

    /// `isRootEntity()`.
    pub fn is_root_entity(&self) -> bool {
        self.name == self.root_entity_name
    }

    /// `isInheritedAssociation()`.
    pub fn is_inherited_association(&self, field_name: &str) -> bool {
        self.association_mappings
            .get(field_name)
            .is_some_and(|m| m.inherited.is_some())
    }

    /// `isInheritedEmbeddedClass()`.
    pub fn is_inherited_embedded_class(&self, field_name: &str) -> bool {
        self.embedded_classes
            .get(field_name)
            .is_some_and(|m| m.inherited.is_some())
    }

    /// `setTableName()`.
    pub fn set_table_name(&mut self, table_name: &str) {
        self.table.insert("name", Value::from(table_name));
    }

    /// `setPrimaryTable()`: sets the primary table definition.
    pub fn set_primary_table(&mut self, table: &PhpArray) {
        if let Some(mut name) = text_of(pv::get(table, "name")).filter(|_| pv::isset(table, "name"))
        {
            if let Some((schema, rest)) = name.clone().split_once('.') {
                self.table.insert("schema", Value::from(schema));
                rest.clone_into(&mut name);
            }
            let quoted = name.starts_with('`');
            let name = if quoted {
                name.trim_matches('`')
            } else {
                &name
            };
            if quoted {
                self.table.insert("quoted", Value::Bool(true));
            }
            self.table.insert("name", Value::from(name));
        }
        for key in [
            "quoted",
            "schema",
            "indexes",
            "uniqueConstraints",
            "options",
        ] {
            if pv::isset(table, key) {
                self.table.insert(key, pv::required(table, key).clone());
            }
        }
    }

    /// `mapField()`: adds a mapped field to the class.
    pub fn map_field(&mut self, mapping: PhpArray) -> Result<(), Thrown> {
        let mapping = self.validate_and_complete_field_mapping(mapping)?;
        self.assert_field_not_mapped(&mapping.field_name)?;
        if mapping.generated.is_some() {
            self.requires_fetch_after_change = true;
        }
        self.field_mappings
            .insert(mapping.field_name.clone(), mapping);
        Ok(())
    }

    /// `addInheritedAssociationMapping()`.
    pub fn add_inherited_association_mapping(
        &mut self,
        mapping: AssociationMapping,
    ) -> Result<(), Thrown> {
        if self.association_mappings.contains_key(&mapping.field_name) {
            return Err(mapping_exception::duplicate_association_mapping(
                &self.name,
                &mapping.field_name,
            ));
        }
        self.association_mappings
            .insert(mapping.field_name.clone(), mapping);
        Ok(())
    }

    /// `addInheritedFieldMapping()`.
    pub fn add_inherited_field_mapping(&mut self, field_mapping: FieldMapping) {
        self.column_names.insert(
            field_mapping.field_name.clone(),
            field_mapping.column_name.clone(),
        );
        self.field_names.insert(
            field_mapping.column_name.clone(),
            field_mapping.field_name.clone(),
        );
        if field_mapping.generated.is_some() {
            self.requires_fetch_after_change = true;
        }
        self.field_mappings
            .insert(field_mapping.field_name.clone(), field_mapping);
    }

    fn map_association(&mut self, mut mapping: PhpArray, ty: i64) -> Result<(), Thrown> {
        mapping.insert("type", Value::Int(ty));
        if matches!(ty, ONE_TO_MANY | MANY_TO_MANY) && pv::isset(&mapping, "orderBy") {
            let field_name = text_of(pv::get(&mapping, "fieldName")).unwrap_or_default();
            let order_by = pv::array(pv::required(&mapping, "orderBy"), CLASS, "orderBy")?;
            mapping.insert(
                "orderBy",
                Value::Array(self.normalize_order_by(&order_by, &field_name)?),
            );
        }
        let mapping = self.validate_and_complete_association_mapping(mapping)?;
        self.store_association_mapping(mapping)
    }

    /// `mapOneToOne()`.
    pub fn map_one_to_one(&mut self, mapping: PhpArray) -> Result<(), Thrown> {
        self.map_association(mapping, ONE_TO_ONE)
    }

    /// `mapOneToMany()`.
    pub fn map_one_to_many(&mut self, mapping: PhpArray) -> Result<(), Thrown> {
        self.map_association(mapping, ONE_TO_MANY)
    }

    /// `mapManyToOne()`.
    pub fn map_many_to_one(&mut self, mapping: PhpArray) -> Result<(), Thrown> {
        self.map_association(mapping, MANY_TO_ONE)
    }

    /// `mapManyToMany()`.
    pub fn map_many_to_many(&mut self, mapping: PhpArray) -> Result<(), Thrown> {
        self.map_association(mapping, MANY_TO_MANY)
    }

    /// `_storeAssociationMapping()`.
    pub fn store_association_mapping(
        &mut self,
        assoc_mapping: AssociationMapping,
    ) -> Result<(), Thrown> {
        self.assert_field_not_mapped(&assoc_mapping.field_name)?;
        self.association_mappings
            .insert(assoc_mapping.field_name.clone(), assoc_mapping);
        Ok(())
    }

    /// `setCustomRepositoryClass()`.
    pub fn set_custom_repository_class(&mut self, repository_class_name: Option<&str>) {
        self.custom_repository_class_name =
            repository_class_name.map(|name| self.fully_qualified_class_name(name));
    }

    /// `hasLifecycleCallbacks()`.
    pub fn has_lifecycle_callbacks(&self, lifecycle_event: &str) -> bool {
        self.lifecycle_callbacks.contains_key(lifecycle_event)
    }

    /// `getLifecycleCallbacks()`.
    pub fn get_lifecycle_callbacks(&self, event: &str) -> Vec<String> {
        self.lifecycle_callbacks
            .get(event)
            .cloned()
            .unwrap_or_default()
    }

    /// `addLifecycleCallback()`.
    pub fn add_lifecycle_callback(&mut self, callback: &str, event: &str) -> Result<(), Thrown> {
        if self.is_embedded_class {
            return Err(
                mapping_exception::illegal_lifecycle_callback_on_embedded_class(
                    callback, &self.name,
                ),
            );
        }
        if self
            .lifecycle_callbacks
            .get(event)
            .is_some_and(|callbacks| callbacks.iter().any(|c| c == callback))
        {
            return Ok(());
        }
        match self.lifecycle_callbacks.get_mut(event) {
            Some(callbacks) => callbacks.push(callback.to_owned()),
            None => self
                .lifecycle_callbacks
                .insert(event, vec![callback.to_owned()]),
        }
        Ok(())
    }

    /// `setLifecycleCallbacks()`.
    pub fn set_lifecycle_callbacks(&mut self, callbacks: OrderedMap<Vec<String>>) {
        self.lifecycle_callbacks = callbacks;
    }

    /// `addEntityListener()`.
    pub fn add_entity_listener(
        &mut self,
        event_name: &str,
        class: &str,
        method: &str,
    ) -> Result<(), Thrown> {
        let class = self.fully_qualified_class_name(class);
        let listener = EntityListener {
            class: class.clone(),
            method: method.to_owned(),
        };
        if !php_reflection::class_exists(&class) {
            return Err(mapping_exception::entity_listener_class_not_found(
                &class, &self.name,
            ));
        }
        if !php_reflection::method_exists(&class, method) {
            return Err(mapping_exception::entity_listener_method_not_found(
                &class, method, &self.name,
            ));
        }
        if self
            .entity_listeners
            .get(event_name)
            .is_some_and(|listeners| listeners.contains(&listener))
        {
            return Err(mapping_exception::duplicate_entity_listener(
                &class, method, &self.name,
            ));
        }
        match self.entity_listeners.get_mut(event_name) {
            Some(listeners) => listeners.push(listener),
            None => self.entity_listeners.insert(event_name, vec![listener]),
        }
        Ok(())
    }

    /// `setDiscriminatorColumn()` with a mapping object.
    pub fn set_discriminator_column_mapping(&mut self, column: DiscriminatorColumnMapping) {
        self.discriminator_column = Some(column);
    }

    /// `setDiscriminatorColumn()` with a mapping array or null.
    pub fn set_discriminator_column(&mut self, column_def: Option<PhpArray>) -> Result<(), Thrown> {
        let Some(mut column_def) = column_def else {
            return Ok(());
        };
        let Some(name) =
            text_of(pv::get(&column_def, "name")).filter(|_| pv::isset(&column_def, "name"))
        else {
            return Err(mapping_exception::name_is_mandatory_for_discriminator_columns(&self.name));
        };
        if self.field_names.contains_key(&name) {
            return Err(mapping_exception::duplicate_column_name(&self.name, &name));
        }
        if !pv::isset(&column_def, "fieldName") {
            column_def.insert("fieldName", Value::from(name.as_str()));
        }
        if !pv::isset(&column_def, "type") {
            column_def.insert("type", Value::from("string"));
        }
        if !pv::isset(&column_def, "options") {
            column_def.insert("options", Value::Array(PhpArray::new()));
        }
        let ty = text_of(pv::get(&column_def, "type")).unwrap_or_default();
        if ["boolean", "array", "object", "datetime", "time", "date"].contains(&ty.as_str()) {
            return Err(mapping_exception::invalid_discriminator_column_type(
                &self.name, &ty,
            ));
        }
        if let Some(enum_type) =
            text_of(pv::get(&column_def, "enumType")).filter(|_| pv::isset(&column_def, "enumType"))
        {
            if !enum_exists(&enum_type) {
                let field_name = text_of(pv::get(&column_def, "fieldName")).unwrap_or_default();
                return Err(mapping_exception::non_enum_type_mapped(
                    &self.name,
                    &field_name,
                    &enum_type,
                ));
            }
            if ty == "enum"
                && let Some(Value::Array(options)) = column_def.get_mut(&Key::from("options"))
                && !pv::isset(options, "values")
            {
                options.insert("values", Value::Array(enum_values(&enum_type)));
            }
        }
        self.discriminator_column =
            Some(DiscriminatorColumnMapping::from_mapping_array(&column_def)?);
        Ok(())
    }

    /// `getDiscriminatorColumn()`.
    pub fn get_discriminator_column(&self) -> Result<&DiscriminatorColumnMapping, Thrown> {
        self.discriminator_column
            .as_ref()
            .ok_or_else(|| logic_exception("The discriminator column was not set.".to_owned()))
    }

    /// `setDiscriminatorMap()`: value => class.
    pub fn set_discriminator_map(&mut self, map: &PhpArray) -> Result<(), Thrown> {
        let mut seen: Vec<String> = Vec::new();
        let mut duplicates: Vec<String> = Vec::new();
        for class in map.values() {
            let class = text_of(Some(class)).unwrap_or_default();
            if seen.contains(&class) {
                if !duplicates.contains(&class) {
                    duplicates.push(class);
                }
            } else {
                seen.push(class);
            }
        }
        if !duplicates.is_empty() {
            let name = self.name.clone();
            deprecations::trigger(DUPLICATE_DISCRIMINATOR_DEPRECATION, || {
                format!(
                    "Mapping a class to multiple discriminator values is deprecated,\nand the discriminator mapping of {name} contains duplicate values\nfor the following discriminator values: {}.",
                    duplicates.join(", ")
                )
            });
        }
        let values = self
            .discriminator_column
            .as_ref()
            .and_then(|column| pv::get(&column.options, "values").cloned())
            .filter(|v| !matches!(v, Value::Null));
        if let Some(Value::Array(values)) = values {
            let allowed: Vec<String> = values.values().filter_map(|v| text_of(Some(v))).collect();
            let diff: Vec<String> = map
                .keys()
                .map(pv::key_name)
                .filter(|key| !allowed.contains(key))
                .collect();
            if !diff.is_empty() {
                let enum_type = self
                    .discriminator_column
                    .as_ref()
                    .and_then(|c| c.enum_type.clone())
                    .unwrap_or_default();
                return Err(mapping_exception::invalid_entries_in_discriminator_map(
                    &diff, &self.name, &enum_type,
                ));
            }
        }
        for (value, class_name) in map.iter() {
            self.add_discriminator_map_class(
                value,
                &text_of(Some(class_name)).unwrap_or_default(),
            )?;
        }
        Ok(())
    }

    /// `addDiscriminatorMapClass()`.
    pub fn add_discriminator_map_class(
        &mut self,
        name: &Key,
        class_name: &str,
    ) -> Result<(), Thrown> {
        let class_name = self.fully_qualified_class_name(class_name);
        let class_name = class_name.trim_start_matches('\\').to_owned();
        self.discriminator_map
            .insert(name.clone(), Value::from(class_name.as_str()));
        if self.name == class_name {
            self.discriminator_value = name.to_value();
            return Ok(());
        }
        let exists = php_reflection::class_info(&class_name)
            .is_some_and(|info| matches!(info.kind(), ClassKind::Class | ClassKind::Interface));
        if !exists {
            return Err(mapping_exception::invalid_class_in_discriminator_map(
                &class_name,
                &self.name,
            ));
        }
        self.add_sub_class(&class_name);
        Ok(())
    }

    /// `addSubClasses()`.
    pub fn add_sub_classes(&mut self, classes: &[String]) {
        for class_name in classes {
            self.add_sub_class(class_name);
        }
    }

    /// `addSubClass()`.
    pub fn add_sub_class(&mut self, class_name: &str) {
        let is_subclass = !class_name.eq_ignore_ascii_case(&self.name)
            && php_reflection::is_a(class_name, &self.name);
        if is_subclass && !self.sub_classes.iter().any(|c| c == class_name) {
            self.sub_classes.push(class_name.to_owned());
        }
    }

    /// `hasAssociation()`.
    pub fn has_association(&self, field_name: &str) -> bool {
        self.association_mappings.contains_key(field_name)
    }

    /// `isSingleValuedAssociation()`.
    pub fn is_single_valued_association(&self, field_name: &str) -> bool {
        self.association_mappings
            .get(field_name)
            .is_some_and(AssociationMapping::is_to_one)
    }

    /// `isCollectionValuedAssociation()`.
    pub fn is_collection_valued_association(&self, field_name: &str) -> bool {
        self.association_mappings
            .get(field_name)
            .is_some_and(|m| !m.is_to_one())
    }

    /// `isAssociationWithSingleJoinColumn()`.
    pub fn is_association_with_single_join_column(&self, field_name: &str) -> bool {
        self.association_mappings
            .get(field_name)
            .is_some_and(|m| m.join_columns.len() == 1)
    }

    /// `getSingleAssociationJoinColumnName()`.
    pub fn get_single_association_join_column_name(
        &self,
        field_name: &str,
    ) -> Result<String, Thrown> {
        match self.association_mappings.get(field_name) {
            Some(assoc) if assoc.join_columns.len() == 1 => Ok(assoc.join_columns[0].name.clone()),
            _ => Err(mapping_exception::no_single_association_join_column_found(
                &self.name, field_name,
            )),
        }
    }

    /// `getSingleAssociationReferencedJoinColumnName()`.
    pub fn get_single_association_referenced_join_column_name(
        &self,
        field_name: &str,
    ) -> Result<String, Thrown> {
        match self.association_mappings.get(field_name) {
            Some(assoc) if assoc.join_columns.len() == 1 => {
                Ok(assoc.join_columns[0].referenced_column_name.clone())
            }
            _ => Err(mapping_exception::no_single_association_join_column_found(
                &self.name, field_name,
            )),
        }
    }

    /// `getFieldForColumn()`.
    pub fn get_field_for_column(&self, column_name: &str) -> Result<String, Thrown> {
        if let Some(field) = self.field_names.get(column_name) {
            return Ok(field.clone());
        }
        for (assoc_name, mapping) in self.association_mappings.iter() {
            if mapping.join_columns.len() == 1 && mapping.join_columns[0].name == column_name {
                return Ok(assoc_name.to_owned());
            }
        }
        Err(mapping_exception::no_field_name_found_for_column(
            &self.name,
            column_name,
        ))
    }

    /// `setIdGenerator()`.
    pub fn set_id_generator(&mut self, generator: IdGenerator) {
        self.id_generator = Some(generator);
    }

    /// `setCustomGeneratorDefinition()`.
    pub fn set_custom_generator_definition(&mut self, definition: PhpArray) {
        self.custom_generator_definition = Some(definition);
    }

    /// `setSequenceGeneratorDefinition()`.
    pub fn set_sequence_generator_definition(
        &mut self,
        mut definition: PhpArray,
    ) -> Result<(), Thrown> {
        let sequence_name = text_of(pv::get(&definition, "sequenceName"))
            .filter(|_| pv::isset(&definition, "sequenceName"));
        let Some(sequence_name) = sequence_name.filter(|name| !name.trim().is_empty()) else {
            return Err(mapping_exception::missing_sequence_name(&self.name));
        };
        if sequence_name.starts_with('`') {
            definition.insert("sequenceName", Value::from(sequence_name.trim_matches('`')));
            definition.insert("quoted", Value::Bool(true));
        }
        for key in ["allocationSize", "initialValue"] {
            let value = pv::get(&definition, key)
                .filter(|_| pv::isset(&definition, key))
                .and_then(Value::to_php_string)
                .map(|s| pv::text(&s))
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "1".to_owned());
            definition.insert(key, Value::from(value));
        }
        self.sequence_generator_definition = Some(definition);
        Ok(())
    }

    /// `setVersionMapping()`: sets the version field mapping used for
    /// versioning; completes the mapping's default.
    pub fn set_version_mapping(&mut self, mapping: &mut PhpArray) -> Result<(), Thrown> {
        self.is_versioned = true;
        self.version_field = text_of(pv::get(mapping, "fieldName"));
        self.requires_fetch_after_change = true;
        if !pv::isset(mapping, "default") {
            let ty = text_of(pv::get(mapping, "type")).unwrap_or_default();
            let default = if ["integer", "bigint", "smallint"].contains(&ty.as_str()) {
                Value::Int(1)
            } else if ty == "datetime" {
                Value::Object(Arc::new(GenericObject::new(
                    "Doctrine\\DBAL\\Schema\\DefaultExpression\\CurrentTimestamp",
                    PhpArray::new(),
                )))
            } else {
                return Err(mapping_exception::unsupported_optimistic_locking_type(
                    &self.name,
                    &self.version_field.clone().unwrap_or_default(),
                    &ty,
                ));
            };
            if let Some(Value::Array(options)) = mapping.get_mut(&Key::from("options")) {
                options.insert("default", default)
            } else {
                let mut options = PhpArray::new();
                options.insert("default", default);
                mapping.insert("options", Value::Array(options));
            }
        }
        Ok(())
    }

    /// `setVersioned()`.
    pub fn set_versioned(&mut self, versioned: bool) {
        self.is_versioned = versioned;
        if versioned {
            self.requires_fetch_after_change = true;
        }
    }

    /// `setVersionField()`.
    pub fn set_version_field(&mut self, version_field: Option<String>) {
        self.version_field = version_field;
    }

    /// `markReadOnly()`.
    pub fn mark_read_only(&mut self) {
        self.is_read_only = true;
    }

    /// `getFieldNames()`.
    pub fn get_field_names(&self) -> Vec<String> {
        self.field_mappings.keys().map(str::to_owned).collect()
    }

    /// `getAssociationNames()`.
    pub fn get_association_names(&self) -> Vec<String> {
        self.association_mappings
            .keys()
            .map(str::to_owned)
            .collect()
    }

    /// `getAssociationTargetClass()`.
    pub fn get_association_target_class(&self, assoc_name: &str) -> Result<String, Thrown> {
        self.association_mappings
            .get(assoc_name)
            .map(|m| m.target_entity.clone())
            .ok_or_else(|| {
                Thrown::new(
                    "InvalidArgumentException",
                    format!("Association name expected, '{assoc_name}' is not an association."),
                    0,
                )
            })
    }

    /// `getName()`.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// `isAssociationInverseSide()`.
    pub fn is_association_inverse_side(&self, assoc_name: &str) -> bool {
        self.association_mappings
            .get(assoc_name)
            .is_some_and(|m| !m.is_owning_side())
    }

    /// `getAssociationMappedByTargetField()`.
    pub fn get_association_mapped_by_target_field(
        &self,
        assoc_name: &str,
    ) -> Result<String, Thrown> {
        let assoc = self.get_association_mapping(assoc_name)?;
        if assoc.is_owning_side() {
            return Err(logic_exception(format!(
                "Context: Calling {CLASS}::getAssociationMappedByTargetField() with \"{assoc_name}\", which is the owning side of an association.\nProblem: The owning side of an association has no \"mappedBy\" field.\nSolution: Call {CLASS}::isAssociationInverseSide() to check first."
            )));
        }
        Ok(assoc.mapped_by.clone().unwrap_or_default())
    }

    /// `fullyQualifiedClassName()`.
    pub fn fully_qualified_class_name(&self, class_name: &str) -> String {
        match &self.namespace {
            Some(namespace)
                if !class_name.contains('\\') && !namespace.is_empty() && namespace != "0" =>
            {
                format!("{namespace}\\{class_name}")
            }
            _ => class_name.to_owned(),
        }
    }

    /// `fullyQualifiedClassName(null)`: deprecated, gives null.
    pub fn fully_qualified_null_class_name(&self) -> Option<String> {
        deprecations::trigger(NULL_CLASS_NAME_DEPRECATION, || {
            format!(
                "Passing null to {CLASS}::fullyQualifiedClassName is deprecated and will not be supported in Doctrine ORM 4.0"
            )
        });
        None
    }

    /// `mapEmbedded()`.
    pub fn map_embedded(&mut self, mut mapping: PhpArray) -> Result<(), Thrown> {
        let field_name = text_of(pv::get(&mapping, "fieldName")).unwrap_or_default();
        self.assert_field_not_mapped(&field_name)?;
        if !pv::isset(&mapping, "class")
            && self.is_typed_property(&field_name)
            && let Some(class) = &self.refl_class
        {
            let property = class.get_property(&field_name)?;
            if let Some(ty) = property.get_type()
                && !ty.get_name().contains(['|', '&'])
            {
                mapping.insert("class", Value::from(ty.get_name()));
            }
        }
        if !truthy(pv::get(&mapping, "class")) {
            return Err(mapping_exception::missing_embedded_class(&field_name));
        }
        let class = self
            .fully_qualified_class_name(&text_of(pv::get(&mapping, "class")).unwrap_or_default());
        let embedded = EmbeddedClassMapping::from_mapping_array(&php_value::php_array![
            "class" => class,
            "columnPrefix" => pv::get(&mapping, "columnPrefix").cloned().unwrap_or_default(),
            "declaredField" => pv::get(&mapping, "declaredField").cloned().unwrap_or_default(),
            "originalField" => pv::get(&mapping, "originalField").cloned().unwrap_or_default(),
        ])?;
        self.embedded_classes.insert(field_name, embedded);
        Ok(())
    }

    /// `inlineEmbeddable()`: inlines the fields of the embeddable.
    pub fn inline_embeddable(
        &mut self,
        property: &str,
        embeddable: &ClassMetadataInfo,
    ) -> Result<(), Thrown> {
        for original_field_mapping in embeddable.field_mappings.values() {
            let mut field_mapping = original_field_mapping.to_array();
            if !pv::isset(&field_mapping, "originalClass") {
                field_mapping.insert("originalClass", Value::from(embeddable.name.as_str()));
            }
            let declared_field = match text_of(pv::get(&field_mapping, "declaredField")) {
                Some(declared) => format!("{property}.{declared}"),
                None => property.to_owned(),
            };
            field_mapping.insert("declaredField", Value::from(declared_field));
            if !pv::isset(&field_mapping, "originalField") {
                field_mapping.insert(
                    "originalField",
                    Value::from(original_field_mapping.field_name.as_str()),
                );
            }
            field_mapping.insert(
                "fieldName",
                Value::from(format!("{property}.{}", original_field_mapping.field_name)),
            );
            let column_prefix = self
                .embedded_classes
                .get(property)
                .map(|e| e.column_prefix.clone())
                .unwrap_or_default();
            let column_name = original_field_mapping.column_name.clone();
            match &column_prefix {
                prefix if prefix.is_truthy() => {
                    if let ColumnPrefix::Prefix(prefix) = prefix {
                        field_mapping
                            .insert("columnName", Value::from(format!("{prefix}{column_name}")));
                    }
                }
                ColumnPrefix::False => {}
                _ => {
                    let class_name = self
                        .refl_class
                        .as_ref()
                        .map_or(self.name.clone(), |c| c.get_name().to_owned());
                    let embeddable_class_name = embeddable
                        .refl_class
                        .as_ref()
                        .map_or(embeddable.name.clone(), |c| c.get_name().to_owned());
                    field_mapping.insert(
                        "columnName",
                        Value::from(self.naming_strategy.embedded_field_to_column_name(
                            property,
                            &column_name,
                            &class_name,
                            &embeddable_class_name,
                        )),
                    );
                }
            }
            self.map_field(field_mapping)?;
        }
        Ok(())
    }

    fn assert_field_not_mapped(&self, field_name: &str) -> Result<(), Thrown> {
        if self.field_mappings.contains_key(field_name)
            || self.association_mappings.contains_key(field_name)
            || self.embedded_classes.contains_key(field_name)
        {
            return Err(mapping_exception::duplicate_field_mapping(
                &self.name, field_name,
            ));
        }
        Ok(())
    }

    /// `getSequenceName()`: the sequence name of the single identifier.
    pub fn get_sequence_name(&self) -> Result<String, Thrown> {
        let sequence_prefix = self.get_sequence_prefix()?;
        let column_name = self.get_single_identifier_column_name()?;
        Ok(format!("{sequence_prefix}_{column_name}_seq"))
    }

    /// `getSequencePrefix()`.
    pub fn get_sequence_prefix(&self) -> Result<String, Thrown> {
        let table_name = self.get_table_name()?;
        Ok(match self.get_schema_name() {
            Some(schema) if !schema.is_empty() && schema != "0" => format!("{schema}.{table_name}"),
            _ => table_name,
        })
    }

    fn register_identifier_field(&mut self, field_name: &str, id_position: Option<i64>) {
        self.identifier.push(field_name.to_owned());
        self.identifier_positions
            .insert(field_name, id_position.unwrap_or(0));
        let positions = self.identifier_positions.clone();
        let position = |field: &String| positions.get(field).copied().unwrap_or(0);
        self.identifier.sort_by_key(position);
    }

    fn normalize_order_by(
        &self,
        order_by: &PhpArray,
        field_name: &str,
    ) -> Result<PhpArray, Thrown> {
        let mut normalized = PhpArray::new();
        for (field, direction) in order_by.iter() {
            if matches!(direction, Value::Object(o) if o.class() == SortDirection::CLASS) {
                normalized.insert(field.clone(), direction.clone());
                continue;
            }
            let direction = text_of(Some(direction)).unwrap_or_default();
            let name = self.name.clone();
            deprecations::trigger(ORDER_BY_DIRECTION_DEPRECATION, || {
                format!(
                    "{name}::{field_name} : Using \"{direction}\" as an order by direction is deprecated in favor of passing a SortDirection instance"
                )
            });
            let case = match direction.to_ascii_uppercase().as_str() {
                "ASC" => SortDirection::Ascending,
                "DESC" => SortDirection::Descending,
                _ => {
                    return Err(mapping_exception::invalid_order_by_direction(
                        &direction,
                        &pv::key_name(field),
                        field_name,
                        &self.name,
                    ));
                }
            };
            normalized.insert(field.clone(), sort_direction(case));
        }
        Ok(normalized)
    }
}

/// A `SortDirection` case as a PHP value.
pub fn sort_direction(direction: SortDirection) -> Value {
    Value::Object(EnumCase::new(SortDirection::CLASS, direction.name(), None).object())
}
