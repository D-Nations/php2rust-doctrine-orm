//! The typed field mappers: a field's DBAL type (and enum type) from the
//! PHP type of its property.
//!
//! Upstream: doctrine/orm src/Mapping/TypedFieldMapper.php src/Mapping/DefaultTypedFieldMapper.php src/Mapping/ChainTypedFieldMapper.php

use std::fmt;
use std::sync::Arc;

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::types::Types;
use php2rust_doctrine_persistence::php_reflection::{self, ClassKind, ReflectionProperty};

use crate::mapping::mapping_exception;
use crate::mapping::property_value as pv;

/// `Doctrine\ORM\Mapping\TypedFieldMapper`.
pub trait TypedFieldMapper: Send + Sync + fmt::Debug {
    /// Validates that the type is valid and completes the mapping type.
    fn validate_and_complete(
        &self,
        mapping: PhpArray,
        field: &ReflectionProperty,
    ) -> Result<PhpArray, Thrown>;
}

/// PHP's built-in type names (`ReflectionNamedType::isBuiltin()`).
const BUILTIN: [&str; 16] = [
    "int", "float", "string", "bool", "array", "iterable", "callable", "object", "mixed", "void",
    "null", "never", "false", "true", "static", "self",
];

pub(crate) fn is_builtin(type_name: &str) -> bool {
    BUILTIN.contains(&type_name.to_ascii_lowercase().as_str())
}

/// `enum_exists($name)`.
pub(crate) fn enum_exists(name: &str) -> bool {
    php_reflection::class_info(name).is_some_and(|info| info.kind() == ClassKind::Enum)
}

/// The backing type of an enum (`ReflectionEnum::getBackingType()`):
/// `None` for a pure enum.
pub(crate) fn enum_backing_type(name: &str) -> Option<&'static str> {
    let info = php_reflection::class_info(name)?;
    let case = info.cases().first()?;
    match case.property(b"value")? {
        Value::Int(_) => Some("int"),
        Value::String(_) => Some("string"),
        _ => None,
    }
}

/// The backing values of an enum's cases
/// (`array_column($enum::cases(), 'value')`).
pub(crate) fn enum_values(name: &str) -> PhpArray {
    php_reflection::class_info(name).map_or_else(PhpArray::new, |info| {
        PhpArray::list(
            info.cases()
                .iter()
                .filter_map(|case| case.property(b"value")),
        )
    })
}

/// `Doctrine\ORM\Mapping\DefaultTypedFieldMapper`.
#[derive(Debug, Clone)]
pub struct DefaultTypedFieldMapper {
    typed_field_mappings: Vec<(String, String)>,
}

impl Default for DefaultTypedFieldMapper {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl DefaultTypedFieldMapper {
    /// `$typedFieldMappings` maps PHP types to DBAL types; they are merged
    /// over the defaults (`array_merge()`).
    pub fn new(typed_field_mappings: Vec<(String, String)>) -> Self {
        let mut mappings: Vec<(String, String)> = [
            ("DateInterval", Types::DATEINTERVAL),
            ("DateTime", Types::DATETIME_MUTABLE),
            ("DateTimeImmutable", Types::DATETIME_IMMUTABLE),
            ("array", Types::JSON),
            ("bool", Types::BOOLEAN),
            ("float", Types::FLOAT),
            ("int", Types::INTEGER),
            ("string", Types::STRING),
            // DBAL 4.3+
            ("BcMath\\Number", Types::NUMBER),
        ]
        .into_iter()
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
        .collect();
        for (php, dbal) in typed_field_mappings {
            match mappings.iter_mut().find(|(known, _)| *known == php) {
                Some(existing) => existing.1 = dbal,
                None => mappings.push((php, dbal)),
            }
        }
        Self {
            typed_field_mappings: mappings,
        }
    }
}

impl TypedFieldMapper for DefaultTypedFieldMapper {
    fn validate_and_complete(
        &self,
        mut mapping: PhpArray,
        field: &ReflectionProperty,
    ) -> Result<PhpArray, Thrown> {
        let Some(ty) = field.get_type() else {
            return Ok(mapping);
        };
        // A union or intersection type is not a ReflectionNamedType.
        if ty.get_name().contains(['|', '&']) {
            return Ok(mapping);
        }
        let mut type_name = ty.get_name().trim_start_matches('?').to_owned();
        if !is_builtin(&type_name)
            && enum_exists(&type_name)
            && (!pv::isset(&mapping, "type")
                || pv::get(&mapping, "type").is_some_and(
                    |t| matches!(t, Value::String(s) if s.as_bytes() == Types::ENUM.as_bytes()),
                ))
        {
            let Some(backing) = enum_backing_type(&type_name) else {
                let field_name = pv::get(&mapping, "fieldName")
                    .and_then(Value::to_php_string)
                    .map(|s| pv::text(&s))
                    .unwrap_or_default();
                return Err(mapping_exception::backed_enum_type_required(
                    field.class(),
                    &field_name,
                    &type_name,
                ));
            };
            mapping.insert("enumType", Value::from(type_name.as_str()));
            backing.clone_into(&mut type_name);
        }
        if pv::isset(&mapping, "type") {
            return Ok(mapping);
        }
        if let Some((_, dbal)) = self
            .typed_field_mappings
            .iter()
            .find(|(php, _)| *php == type_name)
        {
            mapping.insert("type", Value::from(dbal.as_str()));
        }
        Ok(mapping)
    }
}

/// `Doctrine\ORM\Mapping\ChainTypedFieldMapper`.
#[derive(Debug, Clone)]
pub struct ChainTypedFieldMapper {
    typed_field_mappers: Vec<Arc<dyn TypedFieldMapper>>,
}

impl ChainTypedFieldMapper {
    pub fn new(typed_field_mappers: Vec<Arc<dyn TypedFieldMapper>>) -> Self {
        Self {
            typed_field_mappers,
        }
    }
}

impl TypedFieldMapper for ChainTypedFieldMapper {
    fn validate_and_complete(
        &self,
        mut mapping: PhpArray,
        field: &ReflectionProperty,
    ) -> Result<PhpArray, Thrown> {
        for typed_field_mapper in &self.typed_field_mappers {
            mapping = typed_field_mapper.validate_and_complete(mapping, field)?;
        }
        Ok(mapping)
    }
}
