//! `Doctrine\ORM\Mapping\Driver\AttributeDriver`: the mapping read from
//! the mapping attributes of the classes (see
//! [`crate::mapping::attributes`]).
//!
//! Upstream: doctrine/orm src/Mapping/Driver/AttributeDriver.php src/Mapping/Driver/ReflectionBasedDriver.php src/Mapping/Builder/EntityListenerBuilder.php

use std::sync::Arc;

use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_persistence::mapping::ClassMetadata as PersistenceClassMetadata;
use php2rust_doctrine_persistence::mapping::driver::{
    ClassLocator, ColocatedMappingDriver, MappingDriver,
};
use php2rust_doctrine_persistence::php_reflection::{
    self, ClassInfo, ReflectionClass, ReflectionProperty,
};

use crate::events;
use crate::mapping::attributes::{
    self as attr, AttributeClass, JoinColumnProperties, MappingAttribute,
};
use crate::mapping::class_metadata::{self as cm, ClassMetadata, ClassMetadataInfo};
use crate::mapping::driver::attribute_reader::AttributeReader;
use crate::mapping::mapping_exception;
use crate::mapping::property_value as pv;

/// `Doctrine\ORM\Mapping\Driver\AttributeDriver`.
pub const CLASS: &str = "Doctrine\\ORM\\Mapping\\Driver\\AttributeDriver";

/// The paths (or the class locator) the driver lists its classes from.
pub enum Paths {
    Paths(Vec<String>),
    Locator(Arc<dyn ClassLocator>),
}

impl From<Vec<String>> for Paths {
    fn from(paths: Vec<String>) -> Self {
        Self::Paths(paths)
    }
}

impl From<Vec<&str>> for Paths {
    fn from(paths: Vec<&str>) -> Self {
        Self::Paths(paths.into_iter().map(str::to_owned).collect())
    }
}

/// `Doctrine\ORM\Mapping\Driver\AttributeDriver`.
pub struct AttributeDriver {
    colocated: ColocatedMappingDriver,
    reader: AttributeReader,
}

impl std::fmt::Debug for AttributeDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttributeDriver")
            .field("paths", &self.colocated.get_paths())
            .finish_non_exhaustive()
    }
}

/// `constant('Doctrine\ORM\Mapping\ClassMetadata::' . $name)`.
fn class_metadata_constant(name: &str) -> Result<i64, Thrown> {
    cm::constant(name).ok_or_else(|| {
        Thrown::new(
            "Error",
            format!("Undefined constant {}::{name}", cm::CLASS),
            0,
        )
    })
}

impl AttributeDriver {
    /// `new AttributeDriver($paths, $reportFieldsWhereDeclared)`.
    pub fn new(
        paths: impl Into<Paths>,
        report_fields_where_declared: bool,
    ) -> Result<Self, Thrown> {
        if !report_fields_where_declared {
            return Err(Thrown::new(
                "InvalidArgumentException",
                format!(
                    "The $reportFieldsWhereDeclared argument is no longer supported, make sure to omit it when calling {CLASS}::__construct."
                ),
                0,
            ));
        }
        let colocated = ColocatedMappingDriver::new(CLASS);
        match paths.into() {
            Paths::Locator(locator) => colocated.set_class_locator(locator),
            Paths::Paths(paths) => colocated.add_paths(&paths),
        }
        Ok(Self {
            colocated,
            reader: AttributeReader::new(),
        })
    }

    /// The `ColocatedMappingDriver` methods (paths, exclude paths, file
    /// extension).
    pub fn colocated(&self) -> &ColocatedMappingDriver {
        &self.colocated
    }

    /// `isTransient()`.
    pub fn is_transient_class(&self, class_name: &str) -> Result<bool, Thrown> {
        let class = ReflectionClass::new(class_name)?;
        let attributes = self.reader.get_class_attributes(&class);
        Ok(!attributes.iter().any(|(_, instance)| {
            instance.first().is_some_and(|a| {
                matches!(
                    a,
                    MappingAttribute::Entity(_) | MappingAttribute::MappedSuperclass(_)
                )
            })
        }))
    }

    /// `loadMetadataForClass()` on the ORM's class metadata.
    #[allow(
        clippy::too_many_lines,
        reason = "one function upstream; kept in its order"
    )]
    pub fn load(&self, class_name: &str, metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
        let reflection_class = ReflectionClass::new(&metadata.name)?;
        let class_attributes = self.reader.get_class_attributes(&reflection_class);

        if let Some(entity) = class_attributes.get::<attr::Entity>() {
            if let Some(repository_class) = &entity.repository_class {
                metadata.set_custom_repository_class(Some(repository_class));
            }
            if entity.read_only {
                metadata.mark_read_only();
            }
        } else if let Some(mapped_superclass) = class_attributes.get::<attr::MappedSuperclass>() {
            metadata.set_custom_repository_class(mapped_superclass.repository_class.as_deref());
            metadata.is_mapped_superclass = true;
        } else if class_attributes.contains(attr::Embeddable::CLASS) {
            metadata.is_embedded_class = true;
        } else {
            return Err(
                mapping_exception::class_is_not_a_valid_entity_or_mapped_super_class(class_name),
            );
        }

        let mut primary_table = PhpArray::new();
        if let Some(table) = class_attributes.get::<attr::Table>() {
            primary_table.insert("name", pv::from_opt_string(table.name.as_ref()));
            primary_table.insert("schema", pv::from_opt_string(table.schema.as_ref()));
            if !table.options.is_empty() {
                primary_table.insert("options", Value::Array(table.options.clone()));
            }
        }

        if class_attributes.contains(attr::Index::CLASS) {
            if metadata.is_embedded_class {
                return Err(mapping_exception::invalid_attribute_on_embeddable(
                    &metadata.name,
                    attr::Index::CLASS,
                ));
            }
            let mut indexes = PhpArray::new();
            for (idx, index_attribute) in class_attributes
                .collection::<attr::Index>()
                .into_iter()
                .enumerate()
            {
                let mut index = PhpArray::new();
                if let Some(columns) = index_attribute.columns.as_ref().filter(|c| !c.is_empty()) {
                    index.insert("columns", pv::from_strings(columns));
                }
                if let Some(fields) = index_attribute.fields.as_ref().filter(|f| !f.is_empty()) {
                    index.insert("fields", pv::from_strings(fields));
                }
                if pv::isset(&index, "columns") == pv::isset(&index, "fields") {
                    return Err(mapping_exception::invalid_index_configuration(
                        class_name,
                        &index_attribute
                            .name
                            .clone()
                            .unwrap_or_else(|| idx.to_string()),
                    ));
                }
                if let Some(flags) = index_attribute.flags.as_ref().filter(|f| !f.is_empty()) {
                    index.insert("flags", pv::from_strings(flags));
                }
                if let Some(options) = index_attribute.options.as_ref().filter(|o| !o.is_empty()) {
                    index.insert("options", Value::Array(options.clone()));
                }
                match index_attribute
                    .name
                    .as_deref()
                    .filter(|n| !n.is_empty() && *n != "0")
                {
                    Some(name) => indexes.insert(name, Value::Array(index)),
                    None => {
                        // Only fails past PHP_INT_MAX entries.
                        let _ = indexes.push(Value::Array(index));
                    }
                }
            }
            primary_table.insert("indexes", Value::Array(indexes));
        }

        if class_attributes.contains(attr::UniqueConstraint::CLASS) {
            if metadata.is_embedded_class {
                return Err(mapping_exception::invalid_attribute_on_embeddable(
                    &metadata.name,
                    attr::UniqueConstraint::CLASS,
                ));
            }
            let mut constraints = PhpArray::new();
            for (idx, constraint_attribute) in class_attributes
                .collection::<attr::UniqueConstraint>()
                .into_iter()
                .enumerate()
            {
                let mut constraint = PhpArray::new();
                if let Some(columns) = constraint_attribute
                    .columns
                    .as_ref()
                    .filter(|c| !c.is_empty())
                {
                    constraint.insert("columns", pv::from_strings(columns));
                }
                if let Some(fields) = constraint_attribute
                    .fields
                    .as_ref()
                    .filter(|f| !f.is_empty())
                {
                    constraint.insert("fields", pv::from_strings(fields));
                }
                if pv::isset(&constraint, "columns") == pv::isset(&constraint, "fields") {
                    return Err(mapping_exception::invalid_unique_constraint_configuration(
                        class_name,
                        &constraint_attribute
                            .name
                            .clone()
                            .unwrap_or_else(|| idx.to_string()),
                    ));
                }
                if let Some(options) = constraint_attribute
                    .options
                    .as_ref()
                    .filter(|o| !o.is_empty())
                {
                    constraint.insert("options", Value::Array(options.clone()));
                }
                match constraint_attribute
                    .name
                    .as_deref()
                    .filter(|n| !n.is_empty() && *n != "0")
                {
                    Some(name) => constraints.insert(name, Value::Array(constraint)),
                    None => {
                        // Only fails past PHP_INT_MAX entries.
                        let _ = constraints.push(Value::Array(constraint));
                    }
                }
            }
            primary_table.insert("uniqueConstraints", Value::Array(constraints));
        }

        metadata.set_primary_table(&primary_table);

        if let Some(cache) = class_attributes.get::<attr::Cache>() {
            if metadata.is_embedded_class {
                return Err(mapping_exception::invalid_attribute_on_embeddable(
                    &metadata.name,
                    attr::Cache::CLASS,
                ));
            }
            let mut cache_map = PhpArray::new();
            cache_map.insert("region", pv::from_opt_string(cache.region.as_ref()));
            cache_map.insert(
                "usage",
                Value::Int(class_metadata_constant(&format!(
                    "CACHE_USAGE_{}",
                    cache.usage
                ))?),
            );
            metadata.enable_cache(cache_map);
        }

        if let Some(inheritance_type) = class_attributes.get::<attr::InheritanceType>() {
            if metadata.is_embedded_class {
                return Err(mapping_exception::invalid_attribute_on_embeddable(
                    &metadata.name,
                    attr::InheritanceType::CLASS,
                ));
            }
            metadata.set_inheritance_type(class_metadata_constant(&format!(
                "INHERITANCE_TYPE_{}",
                inheritance_type.value
            ))?)?;

            if metadata.inheritance_type != cm::INHERITANCE_TYPE_NONE {
                if let Some(discr) = class_attributes.get::<attr::DiscriminatorColumn>() {
                    let mut column_def = PhpArray::new();
                    column_def.insert("name", pv::from_opt_string(discr.name.as_ref()));
                    column_def.insert(
                        "type",
                        Value::from(discr.r#type.as_deref().unwrap_or("string")),
                    );
                    column_def.insert("length", Value::Int(discr.length.unwrap_or(255)));
                    column_def.insert(
                        "columnDefinition",
                        pv::from_opt_string(discr.column_definition.as_ref()),
                    );
                    column_def.insert("enumType", pv::from_opt_string(discr.enum_type.as_ref()));
                    if !discr.options.is_empty() {
                        column_def.insert("options", Value::Array(discr.options.clone()));
                    }
                    metadata.set_discriminator_column(Some(column_def))?;
                } else {
                    let mut column_def = PhpArray::new();
                    column_def.insert("name", Value::from("dtype"));
                    column_def.insert("type", Value::from("string"));
                    column_def.insert("length", Value::Int(255));
                    metadata.set_discriminator_column(Some(column_def))?;
                }

                if let Some(discr_map) = class_attributes.get::<attr::DiscriminatorMap>() {
                    metadata.set_discriminator_map(&discr_map.value)?;
                }
            }
        }

        if let Some(change_tracking) = class_attributes.get::<attr::ChangeTrackingPolicy>() {
            if metadata.is_embedded_class {
                return Err(mapping_exception::invalid_attribute_on_embeddable(
                    &metadata.name,
                    attr::ChangeTrackingPolicy::CLASS,
                ));
            }
            metadata.set_change_tracking_policy(class_metadata_constant(&format!(
                "CHANGETRACKING_{}",
                change_tracking.value
            ))?);
        }

        for property in properties(&reflection_class) {
            if is_repeated_property_declaration(&property, metadata) {
                continue;
            }
            self.map_property(class_name, &property, metadata)?;
        }

        if let Some(overrides) = class_attributes.get::<attr::AssociationOverrides>() {
            if metadata.is_embedded_class {
                return Err(mapping_exception::invalid_attribute_on_embeddable(
                    &metadata.name,
                    "Doctrine\\ORM\\Mapping\\AssociationOverride",
                ));
            }
            for association_override in &overrides.overrides {
                let mut override_mapping = PhpArray::new();
                if let Some(join_columns) = association_override
                    .join_columns
                    .as_ref()
                    .filter(|c| !c.is_empty())
                {
                    override_mapping.insert(
                        "joinColumns",
                        Value::Array(PhpArray::list(
                            join_columns
                                .iter()
                                .map(|c| Value::Array(join_column_to_array(c))),
                        )),
                    );
                }
                if let Some(join_columns) = association_override
                    .inverse_join_columns
                    .as_ref()
                    .filter(|c| !c.is_empty())
                {
                    override_mapping.insert(
                        "inverseJoinColumns",
                        Value::Array(PhpArray::list(
                            join_columns
                                .iter()
                                .map(|c| Value::Array(join_column_to_array(c))),
                        )),
                    );
                }
                if let Some(join_table_attribute) = &association_override.join_table {
                    let mut join_table = PhpArray::new();
                    join_table.insert(
                        "name",
                        pv::from_opt_string(join_table_attribute.name.as_ref()),
                    );
                    join_table.insert(
                        "schema",
                        pv::from_opt_string(join_table_attribute.schema.as_ref()),
                    );
                    for key in ["joinColumns", "inverseJoinColumns"] {
                        let columns = pv::get(&override_mapping, key)
                            .cloned()
                            .unwrap_or_else(|| Value::Array(PhpArray::new()));
                        join_table.insert(key, columns);
                        override_mapping.remove(&Key::from(key));
                    }
                    override_mapping.insert("joinTable", Value::Array(join_table));
                }
                if let Some(inversed_by) = association_override
                    .inversed_by
                    .as_deref()
                    .filter(|s| !s.is_empty() && *s != "0")
                {
                    override_mapping.insert("inversedBy", Value::from(inversed_by));
                }
                if let Some(fetch) = association_override
                    .fetch
                    .as_deref()
                    .filter(|s| !s.is_empty() && *s != "0")
                {
                    override_mapping.insert(
                        "fetch",
                        Value::Int(class_metadata_constant(&format!("FETCH_{fetch}"))?),
                    );
                }
                metadata.set_association_override(&association_override.name, &override_mapping)?;
            }
        }

        if let Some(overrides) = class_attributes.get::<attr::AttributeOverrides>() {
            if metadata.is_embedded_class {
                return Err(mapping_exception::invalid_attribute_on_embeddable(
                    &metadata.name,
                    attr::AttributeOverrides::CLASS,
                ));
            }
            for attribute_override in &overrides.overrides {
                let mapping =
                    column_to_array(&attribute_override.name, &attribute_override.column)?;
                metadata.set_attribute_override(&attribute_override.name, &mapping)?;
            }
        }

        if let Some(listeners) = class_attributes.get::<attr::EntityListeners>() {
            if metadata.is_embedded_class {
                return Err(mapping_exception::invalid_attribute_on_embeddable(
                    &metadata.name,
                    attr::EntityListeners::CLASS,
                ));
            }
            for item in &listeners.value {
                let listener_class_name = metadata.fully_qualified_class_name(item);
                if !php_reflection::class_exists(&listener_class_name) {
                    return Err(mapping_exception::entity_listener_class_not_found(
                        &listener_class_name,
                        class_name,
                    ));
                }
                let mut has_mapping = false;
                let listener_class = ReflectionClass::new(&listener_class_name)?;
                for (declaring, method) in public_methods(&listener_class) {
                    let callbacks = self.get_method_callbacks(&declaring, &method);
                    has_mapping = has_mapping || !callbacks.is_empty();
                    for (method, event) in callbacks {
                        metadata.add_entity_listener(event, listener_class.get_name(), &method)?;
                    }
                }
                if !has_mapping {
                    bind_entity_listener(metadata, &listener_class_name)?;
                }
            }
        }

        if class_attributes.contains(attr::HasLifecycleCallbacks::CLASS) {
            if metadata.is_embedded_class {
                return Err(mapping_exception::invalid_attribute_on_embeddable(
                    &metadata.name,
                    attr::HasLifecycleCallbacks::CLASS,
                ));
            }
            for (declaring, method) in public_methods(&reflection_class) {
                for (method, event) in self.get_method_callbacks(&declaring, &method) {
                    metadata.add_lifecycle_callback(&method, event)?;
                }
            }
        }

        Ok(())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the property branch of loadMetadataForClass(), in its order"
    )]
    fn map_property(
        &self,
        class_name: &str,
        property: &ReflectionProperty,
        metadata: &mut ClassMetadataInfo,
    ) -> Result<(), Thrown> {
        let reader = &self.reader;
        let mut mapping = PhpArray::new();
        mapping.insert("fieldName", Value::from(property.get_name()));

        if let Some(cache) = reader.get_property_attribute::<attr::Cache>(property)? {
            let mut cache_map = PhpArray::new();
            cache_map.insert(
                "usage",
                Value::Int(class_metadata_constant(&format!(
                    "CACHE_USAGE_{}",
                    cache.usage
                ))?),
            );
            cache_map.insert("region", pv::from_opt_string(cache.region.as_ref()));
            let defaults = metadata.get_association_cache_defaults(property.get_name(), cache_map);
            mapping.insert("cache", Value::Array(defaults));
        }

        let join_columns: Vec<Value> = reader
            .get_property_attribute_collection::<attr::JoinColumn>(property)?
            .iter()
            .map(|c| Value::Array(join_column_to_array(c)))
            .collect();

        let column = reader.get_property_attribute::<attr::Column>(property)?;
        let one_to_one = reader.get_property_attribute::<attr::OneToOne>(property)?;
        let one_to_many = reader.get_property_attribute::<attr::OneToMany>(property)?;
        let many_to_one = reader.get_property_attribute::<attr::ManyToOne>(property)?;
        let many_to_many = reader.get_property_attribute::<attr::ManyToMany>(property)?;
        let embedded = reader.get_property_attribute::<attr::Embedded>(property)?;

        let id = |mapping: &mut PhpArray| -> Result<(), Thrown> {
            if let Some(id) = reader.get_property_attribute::<attr::Id>(property)? {
                mapping.insert("id", Value::Bool(true));
                mapping.insert("idPosition", Value::Int(id.position));
            }
            Ok(())
        };
        let embeddable_check = |metadata: &ClassMetadataInfo, class: &str| {
            if metadata.is_embedded_class {
                Err(mapping_exception::invalid_attribute_on_embeddable(
                    &metadata.name,
                    class,
                ))
            } else {
                Ok(())
            }
        };

        if let Some(column) = column {
            let mut mapping = column_to_array(property.get_name(), &column)?;
            id(&mut mapping)?;
            if let Some(generated) =
                reader.get_property_attribute::<attr::GeneratedValue>(property)?
            {
                metadata.set_id_generator_type(class_metadata_constant(&format!(
                    "GENERATOR_TYPE_{}",
                    generated.strategy
                ))?);
            }
            if reader
                .get_property_attribute::<attr::Version>(property)?
                .is_some()
            {
                metadata.set_version_mapping(&mut mapping)?;
            }
            metadata.map_field(mapping)?;

            let sequence = reader.get_property_attribute::<attr::SequenceGenerator>(property)?;
            let custom = reader.get_property_attribute::<attr::CustomIdGenerator>(property)?;
            if let Some(sequence) = sequence {
                let mut definition = PhpArray::new();
                definition.insert(
                    "sequenceName",
                    pv::from_opt_string(sequence.sequence_name.as_ref()),
                );
                definition.insert("allocationSize", Value::Int(sequence.allocation_size));
                definition.insert("initialValue", Value::Int(sequence.initial_value));
                metadata.set_sequence_generator_definition(definition)?;
            } else if let Some(custom) = custom {
                let mut definition = PhpArray::new();
                definition.insert("class", pv::from_opt_string(custom.class.as_ref()));
                metadata.set_custom_generator_definition(definition);
            }
        } else if let Some(one_to_one) = one_to_one {
            embeddable_check(metadata, attr::OneToOne::CLASS)?;
            id(&mut mapping)?;
            mapping.insert(
                "targetEntity",
                pv::from_opt_string(one_to_one.target_entity.as_ref()),
            );
            mapping.insert("joinColumns", Value::Array(PhpArray::list(join_columns)));
            mapping.insert(
                "mappedBy",
                pv::from_opt_string(one_to_one.mapped_by.as_ref()),
            );
            mapping.insert(
                "inversedBy",
                pv::from_opt_string(one_to_one.inversed_by.as_ref()),
            );
            mapping.insert("cascade", opt_strings(one_to_one.cascade.as_deref()));
            mapping.insert("orphanRemoval", Value::Bool(one_to_one.orphan_removal));
            mapping.insert(
                "fetch",
                Value::Int(get_fetch_mode(class_name, &one_to_one.fetch)?),
            );
            metadata.map_one_to_one(mapping)?;
        } else if let Some(one_to_many) = one_to_many {
            embeddable_check(metadata, attr::OneToMany::CLASS)?;
            mapping.insert(
                "mappedBy",
                pv::from_opt_string(one_to_many.mapped_by.as_ref()),
            );
            mapping.insert(
                "targetEntity",
                pv::from_opt_string(one_to_many.target_entity.as_ref()),
            );
            mapping.insert("cascade", opt_strings(one_to_many.cascade.as_deref()));
            mapping.insert(
                "indexBy",
                pv::from_opt_string(one_to_many.index_by.as_ref()),
            );
            mapping.insert("orphanRemoval", Value::Bool(one_to_many.orphan_removal));
            mapping.insert(
                "fetch",
                Value::Int(get_fetch_mode(class_name, &one_to_many.fetch)?),
            );
            if let Some(order_by) = reader.get_property_attribute::<attr::OrderBy>(property)? {
                mapping.insert("orderBy", Value::Array(order_by.value));
            }
            metadata.map_one_to_many(mapping)?;
        } else if let Some(many_to_one) = many_to_one {
            embeddable_check(metadata, attr::ManyToOne::CLASS)?;
            id(&mut mapping)?;
            mapping.insert("joinColumns", Value::Array(PhpArray::list(join_columns)));
            mapping.insert("cascade", opt_strings(many_to_one.cascade.as_deref()));
            mapping.insert(
                "inversedBy",
                pv::from_opt_string(many_to_one.inversed_by.as_ref()),
            );
            mapping.insert(
                "targetEntity",
                pv::from_opt_string(many_to_one.target_entity.as_ref()),
            );
            mapping.insert(
                "fetch",
                Value::Int(get_fetch_mode(class_name, &many_to_one.fetch)?),
            );
            metadata.map_many_to_one(mapping)?;
        } else if let Some(many_to_many) = many_to_many {
            embeddable_check(metadata, attr::ManyToMany::CLASS)?;
            let mut join_table = PhpArray::new();
            let mut table_join_columns: Vec<Value> = Vec::new();
            let mut table_inverse_join_columns: Vec<Value> = Vec::new();
            if let Some(join_table_attribute) =
                reader.get_property_attribute::<attr::JoinTable>(property)?
            {
                join_table.insert(
                    "name",
                    pv::from_opt_string(join_table_attribute.name.as_ref()),
                );
                join_table.insert(
                    "schema",
                    pv::from_opt_string(join_table_attribute.schema.as_ref()),
                );
                if !join_table_attribute.options.is_empty() {
                    join_table.insert(
                        "options",
                        Value::Array(join_table_attribute.options.clone()),
                    );
                }
                if let Some(name) = &join_table_attribute.foreign_key_name {
                    join_table.insert("foreignKeyName", Value::from(name.as_str()));
                }
                if let Some(name) = &join_table_attribute.inverse_foreign_key_name {
                    join_table.insert("inverseForeignKeyName", Value::from(name.as_str()));
                }
                table_join_columns.extend(
                    join_table_attribute
                        .join_columns
                        .iter()
                        .map(|c| Value::Array(join_column_to_array(c))),
                );
                table_inverse_join_columns.extend(
                    join_table_attribute
                        .inverse_join_columns
                        .iter()
                        .map(|c| Value::Array(join_column_to_array(c))),
                );
            }
            table_join_columns.extend(join_columns);
            table_inverse_join_columns.extend(
                reader
                    .get_property_attribute_collection::<attr::InverseJoinColumn>(property)?
                    .iter()
                    .map(|c| Value::Array(join_column_to_array(c))),
            );
            if !table_join_columns.is_empty() {
                join_table.insert(
                    "joinColumns",
                    Value::Array(PhpArray::list(table_join_columns)),
                );
            }
            if !table_inverse_join_columns.is_empty() {
                join_table.insert(
                    "inverseJoinColumns",
                    Value::Array(PhpArray::list(table_inverse_join_columns)),
                );
            }
            mapping.insert("joinTable", Value::Array(join_table));
            mapping.insert(
                "targetEntity",
                Value::from(many_to_many.target_entity.as_str()),
            );
            mapping.insert(
                "mappedBy",
                pv::from_opt_string(many_to_many.mapped_by.as_ref()),
            );
            mapping.insert(
                "inversedBy",
                pv::from_opt_string(many_to_many.inversed_by.as_ref()),
            );
            mapping.insert("cascade", opt_strings(many_to_many.cascade.as_deref()));
            mapping.insert(
                "indexBy",
                pv::from_opt_string(many_to_many.index_by.as_ref()),
            );
            mapping.insert("orphanRemoval", Value::Bool(many_to_many.orphan_removal));
            mapping.insert(
                "fetch",
                Value::Int(get_fetch_mode(class_name, &many_to_many.fetch)?),
            );
            if let Some(order_by) = reader.get_property_attribute::<attr::OrderBy>(property)? {
                mapping.insert("orderBy", Value::Array(order_by.value));
            }
            metadata.map_many_to_many(mapping)?;
        } else if let Some(embedded) = embedded {
            mapping.insert("class", pv::from_opt_string(embedded.class.as_ref()));
            mapping.insert("columnPrefix", embedded.column_prefix);
            metadata.map_embedded(mapping)?;
        }
        Ok(())
    }

    /// `getMethodCallbacks()`: `[method, event]` pairs.
    fn get_method_callbacks(&self, declaring: &str, method: &str) -> Vec<(String, &'static str)> {
        let mut callbacks = Vec::new();
        for (_, instance) in self.reader.get_method_attributes(declaring, method).iter() {
            let Some(attribute) = instance.first() else {
                continue;
            };
            let event = match attribute {
                MappingAttribute::PrePersist(_) => events::PRE_PERSIST,
                MappingAttribute::PostPersist(_) => events::POST_PERSIST,
                MappingAttribute::PreUpdate(_) => events::PRE_UPDATE,
                MappingAttribute::PostUpdate(_) => events::POST_UPDATE,
                MappingAttribute::PreRemove(_) => events::PRE_REMOVE,
                MappingAttribute::PostRemove(_) => events::POST_REMOVE,
                MappingAttribute::PostLoad(_) => events::POST_LOAD,
                MappingAttribute::PreFlush(_) => events::PRE_FLUSH,
                _ => continue,
            };
            callbacks.push((method.to_owned(), event));
        }
        callbacks
    }
}

impl MappingDriver for AttributeDriver {
    fn load_metadata_for_class(
        &self,
        class_name: &str,
        metadata: &dyn PersistenceClassMetadata,
    ) -> Result<(), Thrown> {
        let Some(metadata) = metadata.downcast_ref::<ClassMetadata>() else {
            return Err(Thrown::new(
                "TypeError",
                format!(
                    "{CLASS}::loadMetadataForClass(): Argument #2 ($metadata) must be of type {}",
                    cm::CLASS
                ),
                0,
            ));
        };
        let mut info = metadata.write();
        self.load(class_name, &mut info)
    }

    fn get_all_class_names(&self) -> Result<Vec<String>, Thrown> {
        self.colocated
            .get_all_class_names(|class| self.is_transient_class(class))
    }

    fn is_transient(&self, class_name: &str) -> Result<bool, Thrown> {
        self.is_transient_class(class_name)
    }
}

/// `ReflectionClass::getProperties()`: the class's own properties, then
/// the non-private ones its parents declare, each with its declaring
/// class.
pub(crate) fn properties(class: &ReflectionClass) -> Vec<ReflectionProperty> {
    let mut properties: Vec<ReflectionProperty> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut current: Option<Arc<ClassInfo>> = php_reflection::class_info(class.get_name());
    let mut first = true;
    let mut depth = 0;
    while let Some(info) = current {
        for property in info.properties() {
            if seen.iter().any(|name| name == property.name()) {
                continue;
            }
            let Ok(reflection) = ReflectionProperty::new(info.name(), property.name()) else {
                continue;
            };
            if !first && reflection.is_private() {
                continue;
            }
            seen.push(property.name().to_owned());
            properties.push(reflection);
        }
        first = false;
        depth += 1;
        current = info
            .parent()
            .filter(|_| depth < 64)
            .and_then(php_reflection::class_info);
    }
    properties
}

/// `ReflectionClass::getMethods(ReflectionMethod::IS_PUBLIC)`: the
/// public methods with their declaring class.
fn public_methods(class: &ReflectionClass) -> Vec<(String, String)> {
    let mut methods: Vec<(String, String)> = Vec::new();
    let mut current: Option<Arc<ClassInfo>> = php_reflection::class_info(class.get_name());
    let mut depth = 0;
    while let Some(info) = current {
        for method in info.methods() {
            if !method.is_public()
                || methods
                    .iter()
                    .any(|(_, known)| known.eq_ignore_ascii_case(method.name()))
            {
                continue;
            }
            methods.push((info.name().to_owned(), method.name().to_owned()));
        }
        depth += 1;
        current = info
            .parent()
            .filter(|_| depth < 64)
            .and_then(php_reflection::class_info);
    }
    methods
}

/// `ReflectionBasedDriver::isRepeatedPropertyDeclaration()`.
fn is_repeated_property_declaration(
    property: &ReflectionProperty,
    metadata: &ClassMetadataInfo,
) -> bool {
    let declaring_class = property.class();
    let name = property.get_name();
    if metadata
        .field_mappings
        .get(name)
        .and_then(|m| m.declared.as_deref())
        == Some(declaring_class)
    {
        return true;
    }
    if metadata
        .association_mappings
        .get(name)
        .and_then(|m| m.declared.as_deref())
        == Some(declaring_class)
    {
        return true;
    }
    metadata
        .embedded_classes
        .get(name)
        .and_then(|m| m.declared.as_deref())
        == Some(declaring_class)
}

/// `EntityListenerBuilder::bindEntityListener()`.
pub fn bind_entity_listener(
    metadata: &mut ClassMetadataInfo,
    class_name: &str,
) -> Result<(), Thrown> {
    const EVENTS: [&str; 8] = [
        events::PRE_REMOVE,
        events::POST_REMOVE,
        events::PRE_PERSIST,
        events::POST_PERSIST,
        events::PRE_UPDATE,
        events::POST_UPDATE,
        events::POST_LOAD,
        events::PRE_FLUSH,
    ];
    let class = metadata.fully_qualified_class_name(class_name);
    if !php_reflection::class_exists(&class) {
        return Err(mapping_exception::entity_listener_class_not_found(
            &class, class_name,
        ));
    }
    let Ok(reflection) = ReflectionClass::new(&class) else {
        return Ok(());
    };
    // get_class_methods() from outside the class: the public methods.
    for (_, method) in public_methods(&reflection) {
        if !EVENTS.contains(&method.as_str()) {
            continue;
        }
        metadata.add_entity_listener(&method, &class, &method)?;
    }
    Ok(())
}

fn get_fetch_mode(class_name: &str, fetch_mode: &str) -> Result<i64, Thrown> {
    cm::constant(&format!("FETCH_{fetch_mode}"))
        .ok_or_else(|| mapping_exception::invalid_fetch_mode(class_name, fetch_mode))
}

fn get_generated_mode(generated_mode: &str) -> Result<i64, Thrown> {
    cm::constant(&format!("GENERATED_{generated_mode}"))
        .ok_or_else(|| mapping_exception::invalid_generated_mode(generated_mode))
}

fn opt_strings(strings: Option<&[String]>) -> Value {
    strings.map_or(Value::Null, pv::from_strings)
}

/// `joinColumnToArray()`.
fn join_column_to_array(join_column: &JoinColumnProperties) -> PhpArray {
    let mut mapping = PhpArray::new();
    mapping.insert("name", pv::from_opt_string(join_column.name.as_ref()));
    mapping.insert("deferrable", Value::Bool(join_column.deferrable));
    mapping.insert("unique", Value::Bool(join_column.unique));
    mapping.insert(
        "nullable",
        join_column.nullable.map_or(Value::Null, Value::Bool),
    );
    mapping.insert("onDelete", join_column.on_delete.clone());
    mapping.insert(
        "columnDefinition",
        pv::from_opt_string(join_column.column_definition.as_ref()),
    );
    mapping.insert(
        "referencedColumnName",
        pv::from_opt_string(join_column.referenced_column_name.as_ref()),
    );
    mapping.insert(
        "foreignKeyName",
        pv::from_opt_string(join_column.foreign_key_name.as_ref()),
    );
    if !join_column.options.is_empty() {
        mapping.insert("options", Value::Array(join_column.options.clone()));
    }
    mapping
}

/// `columnToArray()`.
fn column_to_array(field_name: &str, column: &attr::Column) -> Result<PhpArray, Thrown> {
    let opt_int = |v: Option<i64>| v.map_or(Value::Null, Value::Int);
    let mut mapping = PhpArray::new();
    mapping.insert("fieldName", Value::from(field_name));
    mapping.insert("type", pv::from_opt_string(column.r#type.as_ref()));
    mapping.insert("scale", opt_int(column.scale));
    mapping.insert("length", opt_int(column.length));
    mapping.insert("unique", Value::Bool(column.unique));
    mapping.insert("nullable", Value::Bool(column.nullable));
    mapping.insert("index", Value::Bool(column.index));
    mapping.insert("precision", opt_int(column.precision));
    if !column.options.is_empty() {
        mapping.insert("options", Value::Array(column.options.clone()));
    }
    if let Some(name) = &column.name {
        mapping.insert("columnName", Value::from(name.as_str()));
    }
    if let Some(definition) = &column.column_definition {
        mapping.insert("columnDefinition", Value::from(definition.as_str()));
    }
    if !column.updatable {
        mapping.insert("notUpdatable", Value::Bool(true));
    }
    if !column.insertable {
        mapping.insert("notInsertable", Value::Bool(true));
    }
    if let Some(generated) = &column.generated {
        mapping.insert("generated", Value::Int(get_generated_mode(generated)?));
    }
    if let Some(enum_type) = column
        .enum_type
        .as_deref()
        .filter(|s| !s.is_empty() && *s != "0")
    {
        mapping.insert("enumType", Value::from(enum_type));
    }
    Ok(mapping)
}
