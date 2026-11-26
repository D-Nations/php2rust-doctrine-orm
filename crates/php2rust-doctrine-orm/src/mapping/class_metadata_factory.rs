//! `Doctrine\ORM\Mapping\ClassMetadataFactory`: the ORM's metadata factory
//! on persistence's `AbstractClassMetadataFactory`.
//!
//! PHP's factory reads the mapping driver, the naming strategy, the typed
//! field mapper, the quote strategy and the identity generation
//! preferences from the entity manager's configuration, the platform
//! from its connection, and dispatches `loadClassMetadata` and
//! `onClassMetadataNotFound` through its event manager. The entity manager
//! is not ported: [`FactoryConfiguration`] holds those parts, and its
//! hooks stand in for the two events' listeners.
//!
//! Upstream: doctrine/orm src/Mapping/ClassMetadataFactory.php

use std::collections::HashSet;
use std::fmt::{self, Write as _};
use std::sync::{Arc, Mutex, PoisonError, RwLock};

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::platforms::{AbstractPlatform, SQLitePlatform};
use php2rust_doctrine_persistence::mapping::driver::MappingDriver;
use php2rust_doctrine_persistence::mapping::{
    AbstractClassMetadataFactory, ClassMetadataFactoryState,
};
use php2rust_doctrine_persistence::mapping::{
    ClassMetadata as PersistenceClassMetadata, ReflectionService,
};
use php2rust_doctrine_persistence::php_reflection::{self, ReflectionClass};

use crate::id::IdGenerator;
use crate::mapping::association_mapping::AssociationMapping;
use crate::mapping::class_metadata::{self as cm, ClassMetadata, ClassMetadataInfo};
use crate::mapping::mapping_exception;
use crate::mapping::naming_strategy::{DefaultNamingStrategy, NamingStrategy};
use crate::mapping::property_value as pv;
use crate::mapping::quote_strategy::{DefaultQuoteStrategy, QuoteStrategy, platform_is_a};
use crate::mapping::typed_field_mapper::{DefaultTypedFieldMapper, TypedFieldMapper};

/// A `loadClassMetadata` listener: gets the loaded metadata.
pub type LoadClassMetadataHook =
    Arc<dyn Fn(&Arc<ClassMetadata>) -> Result<(), Thrown> + Send + Sync>;

/// An `onClassMetadataNotFound` listener: may give the metadata of the
/// class (`OnClassMetadataNotFoundEventArgs::setFoundMetadata()`).
pub type ClassMetadataNotFoundHook =
    Arc<dyn Fn(&str) -> Result<Option<Arc<ClassMetadata>>, Thrown> + Send + Sync>;

/// A platform the factory can share.
pub type SharedPlatform = Arc<dyn AbstractPlatform + Send + Sync>;

/// The parts of the entity manager the factory reads.
#[derive(Clone)]
pub struct FactoryConfiguration {
    /// `Configuration::getMetadataDriverImpl()`.
    pub metadata_driver: Option<Arc<dyn MappingDriver>>,
    /// `Configuration::getNamingStrategy()`.
    pub naming_strategy: Arc<dyn NamingStrategy>,
    /// `Configuration::getTypedFieldMapper()`.
    pub typed_field_mapper: Arc<dyn TypedFieldMapper>,
    /// `Configuration::getQuoteStrategy()`.
    pub quote_strategy: Arc<dyn QuoteStrategy>,
    /// `Configuration::getIdentityGenerationPreferences()`: platform
    /// class => generator type.
    pub identity_generation_preferences: Vec<(String, i64)>,
    /// `Connection::getDatabasePlatform()`.
    pub platform: SharedPlatform,
    /// The `loadClassMetadata` listeners.
    pub load_class_metadata: Vec<LoadClassMetadataHook>,
    /// The `onClassMetadataNotFound` listeners.
    pub on_class_metadata_not_found: Vec<ClassMetadataNotFoundHook>,
}

impl fmt::Debug for FactoryConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FactoryConfiguration")
            .field("naming_strategy", &self.naming_strategy)
            .field("quote_strategy", &self.quote_strategy)
            .field("platform", &self.platform.class())
            .finish_non_exhaustive()
    }
}

impl FactoryConfiguration {
    /// A new `Configuration` with `$driver`, on the `SQLite` platform (the
    /// test entity manager's).
    pub fn new(metadata_driver: Arc<dyn MappingDriver>) -> Self {
        Self {
            metadata_driver: Some(metadata_driver),
            naming_strategy: Arc::new(DefaultNamingStrategy::new()),
            typed_field_mapper: Arc::new(DefaultTypedFieldMapper::default()),
            quote_strategy: Arc::new(DefaultQuoteStrategy::new()),
            identity_generation_preferences: Vec::new(),
            platform: Arc::new(SQLitePlatform::new()),
            load_class_metadata: Vec::new(),
            on_class_metadata_not_found: Vec::new(),
        }
    }
}

/// `Doctrine\ORM\Mapping\ClassMetadataFactory`.
pub struct ClassMetadataFactory {
    state: ClassMetadataFactoryState<ClassMetadata>,
    configuration: RwLock<Option<FactoryConfiguration>>,
    driver: RwLock<Option<Arc<dyn MappingDriver>>>,
    embeddables_active_nesting: Mutex<HashSet<String>>,
}

impl fmt::Debug for ClassMetadataFactory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClassMetadataFactory")
            .finish_non_exhaustive()
    }
}

impl Default for ClassMetadataFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// `Platforms\OraclePlatform`, whose default strategy is a sequence.
const ORACLE_PLATFORM: &str = "Doctrine\\DBAL\\Platforms\\OraclePlatform";

fn not_set() -> Thrown {
    Thrown::new(
        "Error",
        "Call to a member function getConfiguration() on null".to_owned(),
        0,
    )
}

impl ClassMetadataFactory {
    pub fn new() -> Self {
        Self {
            state: ClassMetadataFactoryState::new(),
            configuration: RwLock::new(None),
            driver: RwLock::new(None),
            embeddables_active_nesting: Mutex::new(HashSet::new()),
        }
    }

    /// `setEntityManager()`.
    pub fn set_entity_manager(&self, configuration: FactoryConfiguration) {
        *self
            .configuration
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(configuration);
    }

    fn configuration(&self) -> Result<FactoryConfiguration, Thrown> {
        self.configuration
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .ok_or_else(not_set)
    }

    /// `getOwningSide()`.
    pub fn get_owning_side(
        &self,
        maybe_owning_side: &AssociationMapping,
    ) -> Result<AssociationMapping, Thrown> {
        if maybe_owning_side.is_owning_side() {
            return Ok(maybe_owning_side.clone());
        }
        let target = self.get_metadata_for(&maybe_owning_side.target_entity)?;
        let mapped_by = maybe_owning_side.mapped_by.clone().unwrap_or_default();
        let owning_side = target.read().association_mappings.get(&mapped_by).cloned();
        owning_side.ok_or_else(|| {
            Thrown::new("Warning", format!("Undefined array key \"{mapped_by}\""), 0)
        })
    }

    fn target_platform(&self) -> Result<SharedPlatform, Thrown> {
        Ok(self.configuration()?.platform)
    }

    #[allow(clippy::too_many_lines, reason = "doLoadMetadata(), in its order")]
    fn load(
        &self,
        class: &Arc<ClassMetadata>,
        parent: Option<&ClassMetadataInfo>,
        root_entity_found: bool,
        non_superclass_parents: &[String],
    ) -> Result<(), Thrown> {
        if let Some(parent) = parent {
            let mut info = class.write();
            info.set_inheritance_type(parent.inheritance_type)?;
            if let Some(column) = parent.discriminator_column.clone() {
                info.set_discriminator_column_mapping(column);
            }
            info.set_id_generator_type(parent.generator_type);
            add_inherited_fields(&mut info, parent);
            add_inherited_relations(&mut info, parent)?;
            add_inherited_embedded_classes(&mut info, parent);
            let positions: Vec<(String, i64)> = parent
                .identifier_positions
                .iter()
                .map(|(k, v)| (k.to_owned(), *v))
                .collect();
            info.set_identifier(&parent.identifier, &positions);
            info.set_versioned(parent.is_versioned);
            info.set_version_field(parent.version_field.clone());
            info.set_discriminator_map(&parent.discriminator_map)?;
            info.add_sub_classes(&parent.sub_classes);
            info.set_lifecycle_callbacks(parent.lifecycle_callbacks.clone());
            info.set_change_tracking_policy(parent.change_tracking_policy);
            if let Some(definition) = parent
                .custom_generator_definition
                .clone()
                .filter(|d| !d.is_empty())
            {
                info.set_custom_generator_definition(definition);
            }
            if parent.is_mapped_superclass {
                info.set_custom_repository_class(parent.custom_repository_class_name.as_deref());
            }
        }

        // Invoke driver
        let name = class.read().name.clone();
        let driver = self.get_driver()?;
        if let Err(error) =
            driver.load_metadata_for_class(&name, &**class as &dyn PersistenceClassMetadata)
        {
            if php_reflection::thrown_is_a(&error, "ReflectionException") {
                // PHP chains the ReflectionException as the previous
                // exception; Thrown carries no previous one.
                return Err(mapping_exception::reflection_failure(&name));
            }
            return Err(error);
        }

        // If this class has a parent the id generator strategy is
        // inherited. However this is only true if the hierarchy of parents
        // contains the root entity, if it consists of mapped superclasses
        // these don't necessarily include the id field.
        match parent {
            Some(parent) if root_entity_found => {
                inherit_id_generator_mapping(&mut class.write(), parent)?;
            }
            _ => self.complete_id_generator_mapping(&mut class.write())?,
        }

        let (is_mapped_superclass, embedded) = {
            let info = class.read();
            (
                info.is_mapped_superclass,
                info.embedded_classes
                    .iter()
                    .map(|(k, v)| (k.to_owned(), v.clone()))
                    .collect::<Vec<_>>(),
            )
        };
        if !is_mapped_superclass {
            {
                let info = class.read();
                if root_entity_found && info.is_inheritance_type_none() && !info.is_embedded_class {
                    return Err(mapping_exception::missing_inheritance_type_declaration(
                        non_superclass_parents.last().map_or("", String::as_str),
                        &info.name,
                    ));
                }
            }
            for (property, embeddable_class) in embedded {
                if embeddable_class.inherited.is_some() {
                    continue;
                }
                if self.nesting().contains(&embeddable_class.class) {
                    return Err(mapping_exception::infinite_embeddable_nesting(
                        &name, &property,
                    ));
                }
                self.nesting().insert(name.clone());
                let embeddable_metadata = self.get_metadata_for(&embeddable_class.class)?;
                let embeddable = embeddable_metadata.read().clone();
                if embeddable.is_embedded_class {
                    self.add_nested_embedded_classes(&embeddable, class, &property)?;
                }
                if !embeddable.get_identifier().is_empty() {
                    inherit_id_generator_mapping(&mut class.write(), &embeddable)?;
                }
                class.write().inline_embeddable(&property, &embeddable)?;
                self.nesting().remove(&name);
            }
        }

        if let Some(parent) = parent {
            let mut info = class.write();
            if parent.is_inheritance_type_single_table() {
                info.set_primary_table(&parent.table);
            }
            add_inherited_indexes(&mut info, parent);
            if parent.cache.as_ref().is_some_and(|c| !c.is_empty()) {
                info.cache.clone_from(&parent.cache);
            }
            if parent.contains_foreign_identifier {
                info.contains_foreign_identifier = true;
            }
            if parent.contains_enum_identifier {
                info.contains_enum_identifier = true;
            }
            if !parent.entity_listeners.is_empty() && info.entity_listeners.is_empty() {
                info.entity_listeners = parent.entity_listeners.clone();
            }
        }

        class
            .write()
            .set_parent_classes(non_superclass_parents.to_vec());

        let needs_default_map = {
            let info = class.read();
            info.is_root_entity()
                && !info.is_inheritance_type_none()
                && info.discriminator_map.is_empty()
        };
        if needs_default_map {
            self.add_default_discriminator_map(class)?;
        }

        // During the following event, there may also be updates to the
        // discriminator map as per GH-1257/GH-8402. So, we must not
        // discover the missing subclasses before that.
        for hook in &self.configuration()?.load_class_metadata {
            hook(class)?;
        }

        self.find_abstract_entity_classes_not_listed_in_discriminator_map(class)?;
        self.validate_runtime_metadata(&class.read(), parent)
    }

    fn nesting(&self) -> std::sync::MutexGuard<'_, HashSet<String>> {
        self.embeddables_active_nesting
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// `validateRuntimeMetadata()`.
    pub fn validate_runtime_metadata(
        &self,
        class: &ClassMetadataInfo,
        parent: Option<&ClassMetadataInfo>,
    ) -> Result<(), Thrown> {
        let Some(refl_class) = &class.refl_class else {
            // only validate if there is a reflection class instance
            return Ok(());
        };
        class.validate_identifier()?;
        class.validate_associations()?;
        class.validate_lifecycle_callbacks(&*self.get_reflection_service())?;

        // verify inheritance
        if !class.is_mapped_superclass && !class.is_inheritance_type_none() {
            if parent.is_none() {
                if class.discriminator_map.is_empty() {
                    return Err(mapping_exception::missing_discriminator_map(&class.name));
                }
                if class.discriminator_column.is_none() {
                    return Err(mapping_exception::missing_discriminator_column(&class.name));
                }
                for sub_class in &class.sub_classes {
                    if ReflectionClass::new(sub_class)?.get_name() != sub_class {
                        return Err(mapping_exception::invalid_class_in_discriminator_map(
                            sub_class,
                            &class.name,
                        ));
                    }
                }
            } else if !refl_class.is_abstract()
                && !class
                    .discriminator_map
                    .values()
                    .any(|v| v.identical(&Value::from(class.name.as_str())))
            {
                return Err(
                    mapping_exception::mapped_class_not_part_of_discriminator_map(
                        &class.name,
                        &class.root_entity_name,
                    ),
                );
            }
        } else if class.is_mapped_superclass
            && class.name == class.root_entity_name
            && (!class.discriminator_map.is_empty() || class.discriminator_column.is_some())
        {
            // second condition is necessary for mapped superclasses in the
            // middle of an inheritance hierarchy
            return Err(mapping_exception::no_inheritance_on_mapped_super_class(
                &class.name,
            ));
        }
        Ok(())
    }

    /// `addDefaultDiscriminatorMap()`.
    fn add_default_discriminator_map(&self, class: &Arc<ClassMetadata>) -> Result<(), Thrown> {
        let all_classes = self.get_driver()?.get_all_class_names()?;
        let fqcn = class.read().name.clone();
        let mut map: Vec<(String, String)> = vec![(get_short_name(&fqcn), fqcn.clone())];
        let mut duplicates = Vec::new();
        for sub_class_candidate in all_classes {
            if php_reflection::is_a(&sub_class_candidate, &fqcn)
                && !sub_class_candidate.eq_ignore_ascii_case(&fqcn)
            {
                let short_name = get_short_name(&sub_class_candidate);
                match map.iter_mut().find(|(known, _)| *known == short_name) {
                    Some(entry) => {
                        duplicates.push(short_name);
                        entry.1 = sub_class_candidate;
                    }
                    None => map.push((short_name, sub_class_candidate)),
                }
            }
        }
        if !duplicates.is_empty() {
            return Err(mapping_exception::duplicate_discriminator_entry(
                &fqcn,
                &duplicates,
                &map,
            ));
        }
        let mut array = PhpArray::new();
        for (short_name, class_name) in map {
            array.insert(short_name.as_str(), Value::from(class_name.as_str()));
        }
        class.write().set_discriminator_map(&array)
    }

    /// `findAbstractEntityClassesNotListedInDiscriminatorMap()`.
    fn find_abstract_entity_classes_not_listed_in_discriminator_map(
        &self,
        root_entity_class: &Arc<ClassMetadata>,
    ) -> Result<(), Thrown> {
        let (declared, mut processed) = {
            let root = root_entity_class.read();
            // Only root classes in inheritance hierarchies need contain a
            // discriminator map, so skip for other classes.
            if !root.is_root_entity() || root.is_inheritance_type_none() {
                return Ok(());
            }
            let mut processed: Vec<String> = vec![root.name.clone()];
            processed.extend(root.sub_classes.iter().cloned());
            let declared: Vec<String> = root
                .discriminator_map
                .values()
                .filter_map(Value::to_php_string)
                .map(|s| pv::text(&s))
                .collect();
            (declared, processed)
        };
        for declared_class_name in declared {
            // This fetches non-transient parent classes only
            for parent_class in self.get_parent_classes(&declared_class_name)? {
                if processed.contains(&parent_class) {
                    continue;
                }
                processed.push(parent_class.clone());
                // All non-abstract entity classes must be listed in the
                // discriminator map, and this will be validated/enforced at
                // runtime (possibly at a later time, when the subclass is
                // loaded, but anyways). Also, subclasses is about entity
                // classes only. That means we can ignore non-abstract
                // classes here. The (expensive) driver check for mapped
                // superclasses need only be run for abstract candidate
                // classes.
                if !ReflectionClass::new(&parent_class)?.is_abstract()
                    || self.peek_if_is_mapped_superclass(&parent_class)?
                {
                    continue;
                }
                // We have found a non-transient, non-mapped-superclass = an
                // entity class (possibly abstract, but that does not matter)
                root_entity_class.write().add_sub_class(&parent_class);
            }
        }
        Ok(())
    }

    fn peek_if_is_mapped_superclass(&self, class_name: &str) -> Result<bool, Thrown> {
        let refl_service = self.get_reflection_service();
        let class = self.new_class_metadata_instance(class_name)?;
        self.initialize_reflection(&class, &*refl_service)?;
        self.get_driver()?
            .load_metadata_for_class(class_name, &*class as &dyn PersistenceClassMetadata)?;
        let is_mapped_superclass = class.read().is_mapped_superclass;
        Ok(is_mapped_superclass)
    }

    /// `addNestedEmbeddedClasses()`.
    fn add_nested_embedded_classes(
        &self,
        sub_class: &ClassMetadataInfo,
        parent_class: &Arc<ClassMetadata>,
        prefix: &str,
    ) -> Result<(), Thrown> {
        for (property, embeddable_class) in sub_class.embedded_classes.iter() {
            if embeddable_class.inherited.is_some() {
                continue;
            }
            let embeddable_metadata = self.get_metadata_for(&embeddable_class.class)?;
            let embeddable_name = embeddable_metadata.read().name.clone();
            let mut mapping = PhpArray::new();
            mapping.insert("fieldName", Value::from(format!("{prefix}.{property}")));
            mapping.insert("class", Value::from(embeddable_name.as_str()));
            mapping.insert("columnPrefix", embeddable_class.column_prefix.to_value());
            let declared_field = match embeddable_class.declared_field.as_deref() {
                Some(field) if !field.is_empty() && field != "0" => format!("{prefix}.{field}"),
                _ => prefix.to_owned(),
            };
            mapping.insert("declaredField", Value::from(declared_field));
            let original_field = match embeddable_class.original_field.as_deref() {
                Some(field) if !field.is_empty() && field != "0" => field.to_owned(),
                _ => property.to_owned(),
            };
            mapping.insert("originalField", Value::from(original_field));
            parent_class.write().map_embedded(mapping)?;
        }
        Ok(())
    }

    /// `completeIdGeneratorMapping()`.
    fn complete_id_generator_mapping(&self, class: &mut ClassMetadataInfo) -> Result<(), Thrown> {
        let id_gen_type = class.generator_type;
        if id_gen_type == cm::GENERATOR_TYPE_AUTO {
            let strategy = self.determine_id_generator_strategy(&*self.target_platform()?)?;
            class.set_id_generator_type(strategy);
        }

        // Create & assign an appropriate ID generator instance
        match class.generator_type {
            cm::GENERATOR_TYPE_IDENTITY => {
                let field_name = if class.identifier.is_empty() {
                    None
                } else {
                    Some(class.get_single_identifier_field_name()?)
                };
                let big_int = field_name
                    .and_then(|field| class.field_mappings.get(&field))
                    .is_some_and(|mapping| mapping.r#type == "bigint");
                class.set_id_generator(if big_int {
                    IdGenerator::BigIntegerIdentity
                } else {
                    IdGenerator::Identity
                });
            }
            cm::GENERATOR_TYPE_SEQUENCE => {
                // If there is no sequence definition yet, create a default
                // definition
                let definition = if let Some(definition) = class
                    .sequence_generator_definition
                    .clone()
                    .filter(|d| !d.is_empty())
                {
                    definition
                } else {
                    let field_name = class.get_single_identifier_field_name()?;
                    let sequence_name = class.get_sequence_name()?;
                    let quoted = class
                        .field_mappings
                        .get(&field_name)
                        .is_some_and(|m| m.quoted.is_some())
                        || pv::isset(&class.table, "quoted");
                    let mut definition = PhpArray::new();
                    definition.insert(
                        "sequenceName",
                        Value::from(self.truncate_sequence_name(&sequence_name)?),
                    );
                    definition.insert("allocationSize", Value::Int(1));
                    definition.insert("initialValue", Value::Int(1));
                    if quoted {
                        definition.insert("quoted", Value::Bool(true));
                    }
                    class.set_sequence_generator_definition(definition.clone())?;
                    definition
                };
                let configuration = self.configuration()?;
                let sequence_name = configuration.quote_strategy.get_sequence_name(
                    &definition,
                    class,
                    &*configuration.platform,
                );
                let allocation_size =
                    pv::get(&definition, "allocationSize").map_or(0, php_value::ops::to_int);
                class.set_id_generator(IdGenerator::Sequence {
                    sequence_name,
                    allocation_size,
                });
            }
            cm::GENERATOR_TYPE_NONE => class.set_id_generator(IdGenerator::Assigned),
            cm::GENERATOR_TYPE_CUSTOM => {
                let Some(definition) = class.custom_generator_definition.clone() else {
                    return Err(
                        mapping_exception::invalid_custom_generator_on_class_not_configured(),
                    );
                };
                let generator_class = pv::get(&definition, "class")
                    .and_then(Value::to_php_string)
                    .map(|s| pv::text(&s))
                    .unwrap_or_default();
                if !php_reflection::class_exists(&generator_class) {
                    return Err(
                        mapping_exception::invalid_custom_generator_on_missing_class(
                            &var_export_definition(&definition),
                        ),
                    );
                }
                class.set_id_generator(IdGenerator::Custom {
                    class: php_reflection::class_info(&generator_class)
                        .map_or(generator_class, |info| info.name().to_owned()),
                });
            }
            other => return Err(mapping_exception::unknown_generator_type(other)),
        }
        Ok(())
    }

    /// `determineIdGeneratorStrategy()`.
    fn determine_id_generator_strategy(
        &self,
        platform: &dyn AbstractPlatform,
    ) -> Result<i64, Thrown> {
        for (platform_family, strategy) in &self.configuration()?.identity_generation_preferences {
            if platform_is_a(platform, platform_family) {
                return Ok(*strategy);
            }
        }
        // DBAL 4: getIdentitySequenceName() is gone, so PostgreSQL keeps
        // IDENTITY.
        if platform_is_a(platform, ORACLE_PLATFORM) {
            return Ok(cm::GENERATOR_TYPE_SEQUENCE);
        }
        Ok(cm::GENERATOR_TYPE_IDENTITY)
    }

    /// `truncateSequenceName()`.
    fn truncate_sequence_name(&self, schema_element_name: &str) -> Result<String, Thrown> {
        let platform = self.target_platform()?;
        if !platform_is_a(&*platform, ORACLE_PLATFORM) {
            return Ok(schema_element_name.to_owned());
        }
        let max = usize::try_from(platform.get_max_identifier_length()).unwrap_or(0);
        Ok(schema_element_name
            .get(..max)
            .unwrap_or(schema_element_name)
            .to_owned())
    }
}

/// `var_export($definition, true)` of a custom generator definition.
fn var_export_definition(definition: &PhpArray) -> String {
    let mut out = String::from("array (\n");
    for (key, value) in definition.iter() {
        let key = match key {
            php_value::Key::Int(i) => i.to_string(),
            php_value::Key::Str(_) => format!(
                "'{}'",
                pv::key_name(key).replace('\\', "\\\\").replace('\'', "\\'")
            ),
        };
        let value = match value {
            Value::Null => "NULL".to_owned(),
            Value::Bool(b) => if *b { "true" } else { "false" }.to_owned(),
            Value::Int(i) => i.to_string(),
            other => other.to_php_string().map_or_else(String::new, |s| {
                format!(
                    "'{}'",
                    pv::text(&s).replace('\\', "\\\\").replace('\'', "\\'")
                )
            }),
        };
        let _ = writeln!(out, "  {key} => {value},");
    }
    out.push(')');
    out
}

/// `getShortName()`.
fn get_short_name(class_name: &str) -> String {
    class_name
        .rsplit('\\')
        .next()
        .unwrap_or(class_name)
        .to_ascii_lowercase()
}

/// `addMappingInheritanceInformation()`.
fn inheritance_information(
    inherited: &mut Option<String>,
    declared: &mut Option<String>,
    parent_class: &ClassMetadataInfo,
) {
    if inherited.is_none() && !parent_class.is_mapped_superclass {
        *inherited = Some(parent_class.name.clone());
    }
    if declared.is_none() {
        *declared = Some(parent_class.name.clone());
    }
}

fn add_inherited_fields(sub_class: &mut ClassMetadataInfo, parent_class: &ClassMetadataInfo) {
    for mapping in parent_class.field_mappings.values() {
        let mut sub_class_mapping = mapping.clone();
        inheritance_information(
            &mut sub_class_mapping.inherited,
            &mut sub_class_mapping.declared,
            parent_class,
        );
        sub_class.add_inherited_field_mapping(sub_class_mapping);
    }
    for (name, field) in parent_class.property_accessors.iter() {
        sub_class.property_accessors.insert(name, Arc::clone(field));
    }
}

fn add_inherited_relations(
    sub_class: &mut ClassMetadataInfo,
    parent_class: &ClassMetadataInfo,
) -> Result<(), Thrown> {
    for mapping in parent_class.association_mappings.values() {
        let mut sub_class_mapping = mapping.clone();
        inheritance_information(
            &mut sub_class_mapping.inherited,
            &mut sub_class_mapping.declared,
            parent_class,
        );
        // When the class inheriting the relation ($subClass) is the first
        // entity class since the relation has been defined in a mapped
        // superclass (or in a chain of mapped superclasses) above, then
        // declare this current entity class as the source of the
        // relationship.
        if sub_class_mapping.inherited.is_none() {
            sub_class_mapping.source_entity.clone_from(&sub_class.name);
        }
        sub_class.add_inherited_association_mapping(sub_class_mapping)?;
    }
    Ok(())
}

fn add_inherited_embedded_classes(
    sub_class: &mut ClassMetadataInfo,
    parent_class: &ClassMetadataInfo,
) {
    for (field, embedded_class) in parent_class.embedded_classes.iter() {
        let mut sub_class_mapping = embedded_class.clone();
        inheritance_information(
            &mut sub_class_mapping.inherited,
            &mut sub_class_mapping.declared,
            parent_class,
        );
        sub_class.embedded_classes.insert(field, sub_class_mapping);
    }
}

fn add_inherited_indexes(sub_class: &mut ClassMetadataInfo, parent_class: &ClassMetadataInfo) {
    if !parent_class.is_mapped_superclass {
        return;
    }
    for index_type in ["uniqueConstraints", "indexes"] {
        let Some(Value::Array(parent_indexes)) = pv::get(&parent_class.table, index_type) else {
            continue;
        };
        let mut indexes = match pv::get(&sub_class.table, index_type) {
            Some(Value::Array(indexes)) => indexes.clone(),
            _ => PhpArray::new(),
        };
        for (index_name, index) in parent_indexes.iter() {
            if indexes.get(index_name).is_some() {
                continue; // Let the inheriting table override indices
            }
            indexes.insert(index_name.clone(), index.clone());
        }
        sub_class.table.insert(index_type, Value::Array(indexes));
    }
}

/// `inheritIdGeneratorMapping()`.
fn inherit_id_generator_mapping(
    class: &mut ClassMetadataInfo,
    parent: &ClassMetadataInfo,
) -> Result<(), Thrown> {
    if parent.is_id_generator_sequence()
        && let Some(definition) = parent.sequence_generator_definition.clone()
    {
        class.set_sequence_generator_definition(definition)?;
    }
    if parent.generator_type != 0 {
        class.set_id_generator_type(parent.generator_type);
    }
    if let Some(generator) = &parent.id_generator {
        class.set_id_generator(generator.clone());
    }
    Ok(())
}

impl AbstractClassMetadataFactory for ClassMetadataFactory {
    type Metadata = ClassMetadata;

    fn factory_state(&self) -> &ClassMetadataFactoryState<ClassMetadata> {
        &self.state
    }

    fn initialize(&self) -> Result<(), Thrown> {
        let configuration = self.configuration()?;
        *self.driver.write().unwrap_or_else(PoisonError::into_inner) =
            configuration.metadata_driver;
        self.state.set_initialized(true);
        Ok(())
    }

    fn get_driver(&self) -> Result<Arc<dyn MappingDriver>, Thrown> {
        self.driver
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .ok_or_else(|| {
                Thrown::new(
                    "AssertionError",
                    "assert($this->driver !== null)".to_owned(),
                    0,
                )
            })
    }

    fn wakeup_reflection(
        &self,
        class: &Arc<ClassMetadata>,
        refl_service: &dyn ReflectionService,
    ) -> Result<(), Thrown> {
        // No property of this port is virtual (PHP 8.4 property hooks).
        class.write().wakeup_reflection(refl_service)
    }

    fn initialize_reflection(
        &self,
        class: &Arc<ClassMetadata>,
        refl_service: &dyn ReflectionService,
    ) -> Result<(), Thrown> {
        class.write().initialize_reflection(refl_service)
    }

    fn is_entity(&self, class: &ClassMetadata) -> bool {
        !class.read().is_mapped_superclass
    }

    fn do_load_metadata(
        &self,
        class: &Arc<ClassMetadata>,
        parent: Option<&Arc<ClassMetadata>>,
        root_entity_found: bool,
        non_superclass_parents: &[String],
    ) -> Result<(), Thrown> {
        let parent = parent.map(|parent| parent.read().clone());
        self.load(
            class,
            parent.as_ref(),
            root_entity_found,
            non_superclass_parents,
        )
    }

    fn new_class_metadata_instance(&self, class_name: &str) -> Result<Arc<ClassMetadata>, Thrown> {
        let configuration = self.configuration()?;
        Ok(Arc::new(ClassMetadata::new(
            class_name,
            Some(configuration.naming_strategy),
            Some(configuration.typed_field_mapper),
        )))
    }

    fn on_not_found_metadata(
        &self,
        class_name: &str,
    ) -> Result<Option<Arc<ClassMetadata>>, Thrown> {
        let mut found = None;
        for hook in &self.configuration()?.on_class_metadata_not_found {
            if let Some(metadata) = hook(class_name)? {
                found = Some(metadata);
            }
        }
        Ok(found)
    }
}
