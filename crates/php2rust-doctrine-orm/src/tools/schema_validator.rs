//! Performs strict validation of the mapping schema.
//!
//! `schemaInSyncWithMetadata()` and `getUpdateSchemaList()` are not ported
//! (the schema tool has no schema updates). A property type is a named
//! type unless it is a union or an intersection.
//!
//! Upstream: doctrine/orm src/Tools/SchemaValidator.php

use std::sync::Arc;

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::types;
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;
use php2rust_doctrine_persistence::php_reflection;

use crate::entity_manager::EntityManager;
use crate::mapping::typed_field_mapper::enum_backing_type;
use crate::mapping::{ClassMetadata, ClassMetadataInfo, FieldMapping};
use crate::utility::to_text;

/// It maps built-in Doctrine types to PHP types.
const BUILTIN_TYPES_MAP: [(&str, &[&str]); 12] = [
    ("Doctrine\\DBAL\\Types\\AsciiStringType", &["string"]),
    ("Doctrine\\DBAL\\Types\\BigIntType", &["int", "string"]),
    ("Doctrine\\DBAL\\Types\\BooleanType", &["bool"]),
    ("Doctrine\\DBAL\\Types\\DecimalType", &["string"]),
    ("Doctrine\\DBAL\\Types\\FloatType", &["float"]),
    ("Doctrine\\DBAL\\Types\\GuidType", &["string"]),
    ("Doctrine\\DBAL\\Types\\IntegerType", &["int"]),
    ("Doctrine\\DBAL\\Types\\JsonType", &["array"]),
    ("Doctrine\\DBAL\\Types\\SimpleArrayType", &["array"]),
    ("Doctrine\\DBAL\\Types\\SmallIntType", &["int"]),
    ("Doctrine\\DBAL\\Types\\StringType", &["string"]),
    ("Doctrine\\DBAL\\Types\\TextType", &["string"]),
];

/// `array_diff($a, $b)`: the values of `$a` that are not in `$b`.
fn array_diff(a: &[String], b: &[String]) -> Vec<String> {
    a.iter().filter(|value| !b.contains(value)).cloned().collect()
}

/// `array_values()` of an array of strings.
fn string_values(array: &PhpArray) -> Vec<String> {
    array.values().map(to_text).collect()
}

/// `SchemaValidator`.
#[derive(Debug)]
pub struct SchemaValidator {
    em: Arc<EntityManager>,
    validate_property_types: bool,
}

impl SchemaValidator {
    /// `new SchemaValidator($em, $validatePropertyTypes)`.
    pub fn new(em: &Arc<EntityManager>, validate_property_types: bool) -> Self {
        Self {
            em: em.clone(),
            validate_property_types,
        }
    }

    /// Checks the internal consistency of all mapping files.
    ///
    /// There are several checks that can't be done at runtime or are too
    /// expensive, which can be verified with this command. For example:
    ///
    /// 1. Check if a relation with "mappedBy" is actually connected to that
    ///    specified field.
    /// 2. Check if "mappedBy" and "inversedBy" are consistent to each other.
    /// 3. Check if "referencedColumnName" attributes are really pointing to
    ///    primary key columns.
    pub fn validate_mapping(&self) -> Result<Vec<(String, Vec<String>)>, Thrown> {
        let mut errors = Vec::new();
        let classes = self.em.get_metadata_factory().get_all_metadata()?;

        for class in classes {
            let ce = self.validate_class(&class)?;
            if !ce.is_empty() {
                errors.push((class.read().name.clone(), ce));
            }
        }

        Ok(errors)
    }

    /// Validates a single class of the current.
    #[allow(
        clippy::too_many_lines,
        reason = "validateClass(), in its order"
    )]
    pub fn validate_class(&self, class: &ClassMetadata) -> Result<Vec<String>, Thrown> {
        let class = class.read().clone();
        let mut ce = Vec::new();
        let name = &class.name;

        for (field_name, mapping) in class.field_mappings.iter() {
            if !types::has_type(&mapping.r#type) {
                ce.push(format!(
                    "The field '{name}#{field_name}' uses a non-existent type '{}'.",
                    mapping.r#type
                ));
            }
        }

        if self.validate_property_types {
            ce.extend(validate_properties_types(&class));
        }

        for (field_name, assoc) in class.association_mappings.iter() {
            let target_entity = &assoc.target_entity;
            if !php_reflection::class_exists(target_entity) || self.em.is_transient(target_entity)?
            {
                ce.push(format!(
                    "The target entity '{target_entity}' specified on {name}#{field_name} is unknown or not an entity."
                ));

                return Ok(ce);
            }

            let target_metadata = self.em.get_class_metadata(target_entity)?;
            let target_metadata = target_metadata.read().clone();

            if target_metadata.is_mapped_superclass {
                ce.push(format!(
                    "The target entity '{target_entity}' specified on {name}#{field_name} is a mapped superclass. This is not possible since there is no table that a foreign key could refer to."
                ));

                return Ok(ce);
            }

            if assoc.is_many_to_one()
                && class.identifier.len() == 1
                && class.identifier[0] == field_name
            {
                ce.push(format!(
                    "The association '{name}#{field_name}' is a many-to-one association and is the sole identifier of the entity. This effectively makes the association one-to-one; use a one-to-one association instead or add another identifier field."
                ));
            }

            if assoc.id.is_some() && target_metadata.contains_foreign_identifier {
                ce.push(format!(
                    "Cannot map association '{name}#{field_name} as identifier, because the target entity '{}' also maps an association as identifier.",
                    target_metadata.name
                ));
            }

            if !assoc.is_owning_side() {
                let mapped_by = assoc.mapped_by.clone().unwrap_or_default();
                if target_metadata.has_field(&mapped_by) {
                    ce.push(format!(
                        "The association {name}#{field_name} refers to the owning side field {target_entity}#{mapped_by} which is not defined as association, but as field."
                    ));
                }

                match target_metadata.association_mappings.get(&mapped_by) {
                    None => ce.push(format!(
                        "The association {name}#{field_name} refers to the owning side field {target_entity}#{mapped_by} which does not exist."
                    )),
                    Some(target_assoc) => match target_assoc.inversed_by.as_deref() {
                        None => ce.push(format!(
                            "The field {name}#{field_name} is on the inverse side of a bi-directional relationship, but the specified mappedBy association on the target-entity {target_entity}#{mapped_by} does not contain the required 'inversedBy: \"{field_name}\"' attribute."
                        )),
                        Some(inversed_by) if inversed_by != field_name => ce.push(format!(
                            "The mappings {name}#{field_name} and {target_entity}#{mapped_by} are inconsistent with each other."
                        )),
                        Some(_) => {}
                    },
                }
            }

            if assoc.is_owning_side()
                && let Some(inversed_by) = assoc.inversed_by.as_deref()
            {
                if target_metadata.has_field(inversed_by) {
                    ce.push(format!(
                        "The association {name}#{field_name} refers to the inverse side field {target_entity}#{inversed_by} which is not defined as association."
                    ));
                }

                match target_metadata.association_mappings.get(inversed_by) {
                    None => ce.push(format!(
                        "The association {name}#{field_name} refers to the inverse side field {target_entity}#{inversed_by} which does not exist."
                    )),
                    Some(target_assoc) if target_assoc.is_owning_side() => ce.push(format!(
                        "The field {name}#{field_name} is on the owning side of a bi-directional relationship, but the specified inversedBy association on the target-entity {target_entity}#{inversed_by} does not contain the required 'mappedBy: \"{field_name}\"' attribute."
                    )),
                    Some(target_assoc) if target_assoc.mapped_by.as_deref() != Some(field_name) => {
                        ce.push(format!(
                            "The mappings {name}#{field_name} and {target_entity}#{inversed_by} are inconsistent with each other."
                        ));
                    }
                    Some(_) => {}
                }

                // Verify inverse side/owning side match each other
                if let Some(target_assoc) = target_metadata.association_mappings.get(inversed_by) {
                    let target_name = &target_metadata.name;
                    if assoc.is_one_to_one() && !target_assoc.is_one_to_one() {
                        ce.push(format!(
                            "If association {name}#{field_name} is one-to-one, then the inversed side {target_name}#{inversed_by} has to be one-to-one as well."
                        ));
                    } else if assoc.is_many_to_one() && !target_assoc.is_one_to_many() {
                        ce.push(format!(
                            "If association {name}#{field_name} is many-to-one, then the inversed side {target_name}#{inversed_by} has to be one-to-many."
                        ));
                    } else if assoc.is_many_to_many() && !target_assoc.is_many_to_many() {
                        ce.push(format!(
                            "If association {name}#{field_name} is many-to-many, then the inversed side {target_name}#{inversed_by} has to be many-to-many as well."
                        ));
                    }

                    if !php_reflection::is_a(&target_assoc.target_entity, &assoc.source_entity) {
                        ce.push(format!(
                            "The association {name}#{field_name} refers to the inverse side {target_entity}#{inversed_by} which targets a different entity ({}).",
                            target_assoc.target_entity
                        ));
                    }
                }
            }

            if assoc.is_owning_side() {
                if assoc.is_many_to_many_owning_side()
                    && let Some(join_table) = &assoc.join_table
                {
                    let identifier_columns = class.get_identifier_column_names();
                    for join_column in &join_table.join_columns {
                        if !identifier_columns.contains(&join_column.referenced_column_name) {
                            ce.push(format!(
                                "The referenced column name '{}' has to be a primary key column on the target entity class '{name}'.",
                                join_column.referenced_column_name
                            ));
                            break;
                        }
                    }

                    let identifier_columns = target_metadata.get_identifier_column_names();
                    for inverse_join_column in &join_table.inverse_join_columns {
                        if !identifier_columns.contains(&inverse_join_column.referenced_column_name)
                        {
                            ce.push(format!(
                                "The referenced column name '{}' has to be a primary key column on the target entity class '{}'.",
                                inverse_join_column.referenced_column_name, target_metadata.name
                            ));
                            break;
                        }
                    }

                    let target_identifier_columns = target_metadata.get_identifier_column_names();
                    if target_identifier_columns.len() != join_table.inverse_join_columns.len() {
                        ce.push(format!(
                            "The inverse join columns of the many-to-many table '{}' have to contain to ALL identifier columns of the target entity '{}', however '{}' are missing.",
                            join_table.name,
                            target_metadata.name,
                            array_diff(
                                &target_identifier_columns,
                                &string_values(&assoc.relation_to_target_key_columns)
                            )
                            .join(", ")
                        ));
                    }

                    let identifier_columns = class.get_identifier_column_names();
                    if identifier_columns.len() != join_table.join_columns.len() {
                        ce.push(format!(
                            "The join columns of the many-to-many table '{}' have to contain to ALL identifier columns of the source entity '{name}', however '{}' are missing.",
                            join_table.name,
                            array_diff(
                                &identifier_columns,
                                &string_values(&assoc.relation_to_source_key_columns)
                            )
                            .join(", ")
                        ));
                    }
                } else if assoc.is_to_one_owning_side() {
                    let identifier_columns = target_metadata.get_identifier_column_names();
                    for join_column in &assoc.join_columns {
                        if !identifier_columns.contains(&join_column.referenced_column_name) {
                            ce.push(format!(
                                "The referenced column name '{}' has to be a primary key column on the target entity class '{}'.",
                                join_column.referenced_column_name, target_metadata.name
                            ));
                        }
                    }

                    if identifier_columns.len() != assoc.join_columns.len() {
                        let ids: Vec<String> = assoc
                            .join_columns
                            .iter()
                            .map(|join_column| join_column.name.clone())
                            .collect();

                        ce.push(format!(
                            "The join columns of the association '{}' have to match to ALL identifier columns of the target entity '{}', however '{}' are missing.",
                            assoc.field_name,
                            target_metadata.name,
                            array_diff(&identifier_columns, &ids).join(", ")
                        ));
                    }
                }
            }

            if assoc.is_ordered() {
                let target_name = &target_metadata.name;
                for order_field in assoc.order_by().keys() {
                    let order_field = php_value::Key::to_value(order_field);
                    let order_field = to_text(&order_field);
                    if !target_metadata.has_field(&order_field)
                        && !target_metadata.has_association(&order_field)
                    {
                        ce.push(format!(
                            "The association {name}#{field_name} is ordered by a foreign field {order_field} that is not a field on the target entity {target_name}."
                        ));
                        continue;
                    }

                    if target_metadata.is_collection_valued_association(&order_field) {
                        ce.push(format!(
                            "The association {name}#{field_name} is ordered by a field {order_field} on {target_name} that is a collection-valued association."
                        ));
                        continue;
                    }

                    if target_metadata.is_association_inverse_side(&order_field) {
                        ce.push(format!(
                            "The association {name}#{field_name} is ordered by a field {order_field} on {target_name} that is the inverse side of an association."
                        ));
                    }
                }
            }
        }

        if !class.is_inheritance_type_none()
            && !class.is_root_entity()
            && class
                .refl_class
                .as_ref()
                .is_some_and(|refl_class| !refl_class.is_abstract())
            && !class.is_mapped_superclass
            && !class
                .discriminator_map
                .values()
                .any(|value| matches!(value, Value::String(s) if s.as_bytes() == name.as_bytes()))
        {
            ce.push(format!(
                "Entity class '{name}' is part of inheritance hierarchy, but is not mapped in the root entity '{}' discriminator map. All subclasses must be listed in the discriminator map.",
                class.root_entity_name
            ));
        }

        for sub_class in &class.sub_classes {
            if !php_reflection::class_parents(sub_class)
                .unwrap_or_default()
                .contains(name)
            {
                ce.push(format!(
                    "According to the discriminator map class '{sub_class}' has to be a child of '{name}' but these entities are not related through inheritance."
                ));
            }
        }

        Ok(ce)
    }
}

/// `validatePropertiesTypes()`: the found issues.
fn validate_properties_types(class: &ClassMetadataInfo) -> Vec<String> {
    class
        .field_mappings
        .iter()
        .filter_map(|(_, field_mapping)| validate_property_type(class, field_mapping))
        .collect()
}

#[allow(
    clippy::too_many_lines,
    reason = "the closure of validatePropertiesTypes(), in its order"
)]
fn validate_property_type(class: &ClassMetadataInfo, field_mapping: &FieldMapping) -> Option<String> {
    let name = &class.name;
    let field_name = &field_mapping.field_name;
    let property_type = class
        .property_accessors
        .get(field_name)?
        .get_underlying_reflector()
        .get_type()
        .map(|ty| ty.get_name().to_owned());

    // If the field type is not a built-in type, we cannot check it
    if !types::has_type(&field_mapping.r#type) {
        return None;
    }

    // If the property type is not a named type, we cannot check it
    let property_type = property_type
        .filter(|ty| !ty.contains('|') && !ty.contains('&') && ty != "mixed")?;

    // If the metadata field type is not a mapped built-in type, we cannot check it
    let metadata_field_type = find_built_in_type(&field_mapping.r#type)?;

    // If the property type is the same as the metadata field type, we are ok
    if metadata_field_type.contains(&property_type.as_str()) {
        return None;
    }

    let metadata_field_types = metadata_field_type.join("|");

    if php_reflection::enum_exists(&property_type)
        && let Some(backing_type) = enum_backing_type(&property_type)
    {
        if !metadata_field_type.contains(&backing_type) {
            return Some(format!(
                "The field '{name}#{field_name}' has the property type '{property_type}' with a backing type of '{backing_type}' that differs from the metadata field type '{metadata_field_types}'."
            ));
        }

        let enum_type = field_mapping.enum_type.as_deref()?;
        if property_type == enum_type {
            return None;
        }

        return Some(format!(
            "The field '{name}#{field_name}' has the property type '{property_type}' that differs from the metadata enumType '{enum_type}'."
        ));
    }

    if let Some(enum_type) = field_mapping.enum_type.as_deref()
        && property_type != enum_type
        && php_reflection::interface_exists(&property_type)
        && php_reflection::is_a(enum_type, &property_type)
    {
        let backing_type = enum_backing_type(enum_type).unwrap_or("");

        if metadata_field_type.contains(&backing_type) {
            return None;
        }

        return Some(format!(
            "The field '{name}#{field_name}' has the metadata enumType '{enum_type}' with a backing type of '{backing_type}' that differs from the metadata field type '{metadata_field_types}'."
        ));
    }

    if field_mapping.r#type == "json"
        && ["string", "int", "float", "bool", "true", "false", "null"]
            .contains(&property_type.as_str())
    {
        return None;
    }

    Some(format!(
        "The field '{name}#{field_name}' has the property type '{property_type}' that differs from the metadata field type '{metadata_field_types}' returned by the '{}' DBAL type.",
        field_mapping.r#type
    ))
}

/// `findBuiltInType()`: the exact DBAL type must be used (no subclasses),
/// since consumers of doctrine/orm may have their own customization around
/// field types.
fn find_built_in_type(type_name: &str) -> Option<&'static [&'static str]> {
    let ty = types::get_type(type_name).ok()?;
    let class = ty.class();
    BUILTIN_TYPES_MAP
        .iter()
        .find(|(type_class, _)| *type_class == class)
        .map(|(_, php_types)| *php_types)
}
