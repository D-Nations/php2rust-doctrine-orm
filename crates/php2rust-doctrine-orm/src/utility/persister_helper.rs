//! The persister helper functions: the types of fields and columns, and
//! the parameter values and types of criteria.
//!
//! Upstream: doctrine/orm src/Utility/PersisterHelper.php

use php_value::{Thrown, Value};
use php2rust_doctrine_dbal::{ArrayParameterType, ParameterType, WrapperParameterType, types};

use crate::entity_manager::EntityManager;
use crate::mapping::ClassMetadataInfo;

use super::identifier_flattener::backed_value;
use super::to_text;

/// A parameter type: a DBAL type name, or a binding type.
pub type ParamType = WrapperParameterType;

const DATE_TIME_TYPES: [&str; 8] = [
    "date",
    "date_immutable",
    "datetime",
    "datetime_immutable",
    "datetimetz",
    "datetimetz_immutable",
    "time",
    "time_immutable",
];

/// `getTypeOfField()`: the types of the field (one for each join column
/// of an association).
pub fn get_type_of_field(
    field_name: &str,
    class: &ClassMetadataInfo,
    em: &EntityManager,
) -> Result<Vec<String>, Thrown> {
    if let Some(mapping) = class.field_mappings.get(field_name) {
        return Ok(vec![mapping.r#type.clone()]);
    }
    let Some(assoc) = class.association_mappings.get(field_name) else {
        return Ok(Vec::new());
    };
    if !assoc.is_owning_side() {
        let target = em.get_class_metadata(&assoc.target_entity)?;
        let target = target.read();
        return get_type_of_field(assoc.mapped_by.as_deref().unwrap_or_default(), &target, em);
    }
    let join_columns = if assoc.is_many_to_many_owning_side() {
        assoc
            .join_table
            .as_ref()
            .map(|t| t.join_columns.clone())
            .unwrap_or_default()
    } else {
        assoc.join_columns.clone()
    };
    let target = em.get_class_metadata(&assoc.target_entity)?;
    let target = target.read();
    let mut types = Vec::new();
    for join_column in &join_columns {
        types.push(get_type_of_column(
            &join_column.referenced_column_name,
            &target,
            em,
        )?);
    }
    Ok(types)
}

/// `getTypeOfColumn()`.
pub fn get_type_of_column(
    column_name: &str,
    class: &ClassMetadataInfo,
    em: &EntityManager,
) -> Result<String, Thrown> {
    if let Some(field_name) = class.field_names.get(column_name)
        && let Some(mapping) = class.field_mappings.get(field_name)
    {
        return Ok(mapping.r#type.clone());
    }
    // iterate over to-one association mappings
    for assoc in class.association_mappings.values() {
        if !assoc.is_to_one_owning_side() {
            continue;
        }
        for join_column in &assoc.join_columns {
            if join_column.name == column_name {
                let target = em.get_class_metadata(&assoc.target_entity)?;
                let target = target.read();
                return get_type_of_column(&join_column.referenced_column_name, &target, em);
            }
        }
    }
    // iterate over to-many association mappings
    for assoc in class.association_mappings.values() {
        if !assoc.is_many_to_many_owning_side() {
            continue;
        }
        let Some(join_table) = &assoc.join_table else {
            continue;
        };
        for join_column in &join_table.join_columns {
            if join_column.name == column_name {
                let target = em.get_class_metadata(&assoc.target_entity)?;
                let target = target.read();
                return get_type_of_column(&join_column.referenced_column_name, &target, em);
            }
        }
    }
    Err(Thrown::new(
        "RuntimeException",
        format!(
            "Could not resolve type of column \"{column_name}\" of class \"{}\"",
            class.name
        ),
        0,
    ))
}

/// `inferParameterTypes()`.
pub fn infer_parameter_types(
    field: &str,
    value: &Value,
    class: &ClassMetadataInfo,
    em: &EntityManager,
) -> Result<Vec<ParamType>, Thrown> {
    let mut types: Vec<ParamType> = Vec::new();
    if let Some(mapping) = class.field_mappings.get(field) {
        types.push(mapping.r#type.as_str().into());
    } else if let Some(assoc) = class.association_mappings.get(field) {
        let assoc = em.get_metadata_factory().get_owning_side(assoc)?;
        let target = em.get_class_metadata(&assoc.target_entity)?;
        let target = target.read();
        let columns = if assoc.is_many_to_many_owning_side() {
            &assoc.relation_to_target_key_columns
        } else {
            &assoc.source_to_target_key_columns
        };
        for column in columns.values() {
            types.push(
                get_type_of_column(&to_text(column), &target, em)?
                    .as_str()
                    .into(),
            );
        }
    } else {
        types.push(ParameterType::String.into());
    }
    // normalizeDateTimeTypesForValue()
    if matches!(value, Value::String(_)) {
        for ty in &mut types {
            if let WrapperParameterType::Name(name) = ty
                && DATE_TIME_TYPES.contains(&name.to_string_lossy().as_ref())
            {
                *ty = ParameterType::String.into();
            }
        }
    }
    if matches!(value, Value::Array(_)) {
        return types.into_iter().map(array_binding_type).collect();
    }
    Ok(types)
}

fn array_binding_type(ty: ParamType) -> Result<ParamType, Thrown> {
    let binding = match ty {
        WrapperParameterType::Parameter(p) => p,
        WrapperParameterType::Name(name) => types::get_type(&name.to_string_lossy())
            .map_err(|e| Thrown::new(e.class(), e.message(), 0))?
            .get_binding_type(),
        WrapperParameterType::Type(t) => t.get_binding_type(),
        other @ WrapperParameterType::Array(_) => return Ok(other),
    };
    Ok(match binding {
        ParameterType::Integer => ArrayParameterType::Integer,
        ParameterType::Ascii => ArrayParameterType::Ascii,
        ParameterType::Binary => ArrayParameterType::Binary,
        _ => ArrayParameterType::String,
    }
    .into())
}

/// `convertToParameterValue()`.
pub fn convert_to_parameter_value(value: &Value, em: &EntityManager) -> Result<Vec<Value>, Thrown> {
    if let Value::Array(items) = value {
        let mut new_value = Vec::new();
        for item in items.values() {
            new_value.extend(convert_to_parameter_value(item, em)?);
        }
        return Ok(vec![Value::Array(php_value::PhpArray::list(new_value))]);
    }
    convert_individual_value(value, em)
}

fn convert_individual_value(value: &Value, em: &EntityManager) -> Result<Vec<Value>, Thrown> {
    let Value::Object(object) = value else {
        return Ok(vec![value.clone()]);
    };
    if super::identifier_flattener::is_backed_enum(object.as_ref()) {
        return Ok(vec![backed_value(value.clone())]);
    }
    if em.is_transient(object.class())? {
        return Ok(vec![value.clone()]);
    }
    let class = em.get_class_metadata(object.class())?;
    let (composite, values) = {
        let info = class.read();
        (
            info.is_identifier_composite,
            info.get_identifier_values(object)?,
        )
    };
    if composite {
        let mut new_value = Vec::new();
        for inner in values.values() {
            new_value.extend(convert_to_parameter_value(inner, em)?);
        }
        return Ok(new_value);
    }
    Ok(vec![
        em.get_unit_of_work().get_single_identifier_value(object)?,
    ])
}
