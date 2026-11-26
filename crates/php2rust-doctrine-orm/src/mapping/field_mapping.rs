//! `FieldMapping`: the mapping of one field (column) of an entity, with
//! `ArrayAccessImplementation`'s deprecated array access.
//!
//! Upstream: doctrine/orm src/Mapping/FieldMapping.php src/Mapping/ArrayAccessImplementation.php

use php_value::{PhpArray, Thrown, Value};

use crate::deprecations;
use crate::mapping::property_value::{
    self as pv, Sleep, from_opt_array, from_opt_bool, from_opt_int, from_opt_string,
};

/// The deprecation of the array access on the mapping value objects.
pub const ARRAY_ACCESS_DEPRECATION: &str = "https://github.com/doctrine/orm/pull/11211";

/// `ArrayAccessImplementation`'s deprecation.
pub(crate) fn trigger_array_access(class: &str) {
    deprecations::trigger(ARRAY_ACCESS_DEPRECATION, || {
        format!(
            "Using ArrayAccess on {class} is deprecated and will not be possible in Doctrine ORM 4.0. Use the corresponding property instead."
        )
    });
}

/// `new InvalidArgumentException('Undefined property: ' . $offset)`.
pub(crate) fn undefined_property(offset: &str) -> Thrown {
    Thrown::new(
        "InvalidArgumentException",
        format!("Undefined property: {offset}"),
        0,
    )
}

/// A value object with `ArrayAccessImplementation`: its properties by
/// name.
pub trait MappingProperties {
    const CLASS: &'static str;
    /// The property's value, `None` when the class has no such property.
    fn property(&self, name: &str) -> Option<Value>;
    /// Assigns a property; `Ok(false)` when the class has no such
    /// property (PHP then creates a dynamic property, which this port does
    /// not keep).
    fn set_property(&mut self, name: &str, value: Value) -> Result<bool, Thrown>;

    /// `offsetExists()`: `isset($this->$offset)`.
    fn offset_exists(&self, offset: &str) -> bool {
        trigger_array_access(Self::CLASS);
        self.property(offset)
            .is_some_and(|value| !matches!(value, Value::Null))
    }

    /// `offsetGet()`.
    fn offset_get(&self, offset: &str) -> Result<Value, Thrown> {
        trigger_array_access(Self::CLASS);
        self.property(offset)
            .ok_or_else(|| undefined_property(offset))
    }

    /// `offsetSet()`.
    fn offset_set(&mut self, offset: &str, value: Value) -> Result<(), Thrown> {
        trigger_array_access(Self::CLASS);
        self.set_property(offset, value).map(|_| ())
    }

    /// `offsetUnset()`: the property becomes null.
    fn offset_unset(&mut self, offset: &str) -> Result<(), Thrown> {
        trigger_array_access(Self::CLASS);
        self.set_property(offset, Value::Null).map(|_| ())
    }
}

/// `Doctrine\ORM\Mapping\FieldMapping`.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldMapping {
    /// The database length of the column. Optional. Default value taken
    /// from the type.
    pub length: Option<i64>,
    /// Marks the field as the primary key of the entity. Multiple fields
    /// of an entity can have the id attribute, forming a composite key.
    pub id: Option<bool>,
    pub id_position: Option<i64>,
    pub nullable: Option<bool>,
    pub not_insertable: Option<bool>,
    pub not_updatable: Option<bool>,
    pub column_definition: Option<String>,
    /// `ClassMetadata::GENERATED_*`.
    pub generated: Option<i64>,
    pub enum_type: Option<String>,
    /// The precision of a decimal column.
    pub precision: Option<i64>,
    /// The scale of a decimal column.
    pub scale: Option<i64>,
    /// Whether a unique constraint should be generated for the column.
    pub unique: Option<bool>,
    /// Whether an index should be generated for the column.
    pub index: Option<bool>,
    /// The topmost entity class this field is inherited from.
    pub inherited: Option<String>,
    pub original_class: Option<String>,
    pub original_field: Option<String>,
    pub quoted: Option<bool>,
    /// The topmost non-transient class that declares this field.
    pub declared: Option<String>,
    pub declared_field: Option<String>,
    pub options: Option<PhpArray>,
    pub version: Option<bool>,
    /// Deprecated: use options with the 'default' key instead. A string,
    /// an int or null.
    pub default: Value,
    /// The type name of the mapped field.
    pub r#type: String,
    /// The name of the field in the entity.
    pub field_name: String,
    /// The column name.
    pub column_name: String,
}

impl FieldMapping {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Mapping\\FieldMapping";

    /// The property names, in declaration order.
    pub const PROPERTIES: [&'static str; 25] = [
        "length",
        "id",
        "idPosition",
        "nullable",
        "notInsertable",
        "notUpdatable",
        "columnDefinition",
        "generated",
        "enumType",
        "precision",
        "scale",
        "unique",
        "index",
        "inherited",
        "originalClass",
        "originalField",
        "quoted",
        "declared",
        "declaredField",
        "options",
        "version",
        "default",
        "type",
        "fieldName",
        "columnName",
    ];

    pub fn new(
        r#type: impl Into<String>,
        field_name: impl Into<String>,
        column_name: impl Into<String>,
    ) -> Self {
        Self {
            length: None,
            id: None,
            id_position: None,
            nullable: None,
            not_insertable: None,
            not_updatable: None,
            column_definition: None,
            generated: None,
            enum_type: None,
            precision: None,
            scale: None,
            unique: None,
            index: None,
            inherited: None,
            original_class: None,
            original_field: None,
            quoted: None,
            declared: None,
            declared_field: None,
            options: None,
            version: None,
            default: Value::Null,
            r#type: r#type.into(),
            field_name: field_name.into(),
            column_name: column_name.into(),
        }
    }

    /// `FieldMapping::fromMappingArray()`.
    pub fn from_mapping_array(mapping_array: &PhpArray) -> Result<Self, Thrown> {
        let class = Self::CLASS;
        let mut mapping = Self::new(
            pv::string(pv::required(mapping_array, "type"), class, "type")?,
            pv::string(pv::required(mapping_array, "fieldName"), class, "fieldName")?,
            pv::string(
                pv::required(mapping_array, "columnName"),
                class,
                "columnName",
            )?,
        );
        for (key, value) in mapping_array.iter() {
            let key = pv::key_name(key);
            if ["type", "fieldName", "columnName"].contains(&key.as_str()) {
                continue;
            }
            if Self::PROPERTIES.contains(&key.as_str()) {
                mapping.set_property(&key, value.clone())?;
            }
        }
        Ok(mapping)
    }

    /// `(array) $mapping`: every property, in declaration order.
    pub fn to_array(&self) -> PhpArray {
        let mut array = PhpArray::new();
        for name in Self::PROPERTIES {
            array.insert(name, self.property(name).unwrap_or_default());
        }
        array
    }

    /// `__sleep()`.
    pub fn sleep_names(&self) -> Vec<&'static str> {
        let mut serialized = vec!["type", "fieldName", "columnName"];
        for (name, set) in [
            ("nullable", self.nullable),
            ("notInsertable", self.not_insertable),
            ("notUpdatable", self.not_updatable),
            ("id", self.id),
            ("unique", self.unique),
            ("version", self.version),
            ("quoted", self.quoted),
            ("index", self.index),
        ] {
            if set == Some(true) {
                serialized.push(name);
            }
        }
        for name in [
            "idPosition",
            "length",
            "columnDefinition",
            "generated",
            "enumType",
            "precision",
            "scale",
            "inherited",
            "originalClass",
            "originalField",
            "declared",
            "declaredField",
            "options",
            "default",
        ] {
            if !matches!(self.property(name), Some(Value::Null) | None) {
                serialized.push(name);
            }
        }
        serialized
    }

    /// `unserialize(serialize($mapping))`'s second half: the object from
    /// its serialized properties.
    pub fn from_properties(properties: &PhpArray) -> Result<Self, Thrown> {
        let class = Self::CLASS;
        let mut mapping = Self::new(
            pv::string(pv::required(properties, "type"), class, "type")?,
            pv::string(pv::required(properties, "fieldName"), class, "fieldName")?,
            pv::string(pv::required(properties, "columnName"), class, "columnName")?,
        );
        for (key, value) in properties.iter() {
            mapping.set_property(&pv::key_name(key), value.clone())?;
        }
        Ok(mapping)
    }

    /// `unserialize()` of a serialized `FieldMapping`.
    pub fn unserialize(data: &[u8]) -> Option<Self> {
        let (class, properties) = pv::unserialize_object(data, &[Self::CLASS])?;
        if class != Self::CLASS {
            return None;
        }
        Self::from_properties(&properties).ok()
    }
}

impl Sleep for FieldMapping {
    fn class_name(&self) -> String {
        Self::CLASS.to_owned()
    }

    fn sleep(&self) -> PhpArray {
        let mut array = PhpArray::new();
        for name in self.sleep_names() {
            array.insert(name, self.property(name).unwrap_or_default());
        }
        array
    }
}

impl MappingProperties for FieldMapping {
    const CLASS: &'static str = FieldMapping::CLASS;

    fn property(&self, name: &str) -> Option<Value> {
        Some(match name {
            "length" => from_opt_int(self.length),
            "id" => from_opt_bool(self.id),
            "idPosition" => from_opt_int(self.id_position),
            "nullable" => from_opt_bool(self.nullable),
            "notInsertable" => from_opt_bool(self.not_insertable),
            "notUpdatable" => from_opt_bool(self.not_updatable),
            "columnDefinition" => from_opt_string(self.column_definition.as_ref()),
            "generated" => from_opt_int(self.generated),
            "enumType" => from_opt_string(self.enum_type.as_ref()),
            "precision" => from_opt_int(self.precision),
            "scale" => from_opt_int(self.scale),
            "unique" => from_opt_bool(self.unique),
            "index" => from_opt_bool(self.index),
            "inherited" => from_opt_string(self.inherited.as_ref()),
            "originalClass" => from_opt_string(self.original_class.as_ref()),
            "originalField" => from_opt_string(self.original_field.as_ref()),
            "quoted" => from_opt_bool(self.quoted),
            "declared" => from_opt_string(self.declared.as_ref()),
            "declaredField" => from_opt_string(self.declared_field.as_ref()),
            "options" => from_opt_array(self.options.as_ref()),
            "version" => from_opt_bool(self.version),
            "default" => self.default.clone(),
            "type" => Value::from(self.r#type.as_str()),
            "fieldName" => Value::from(self.field_name.as_str()),
            "columnName" => Value::from(self.column_name.as_str()),
            _ => return None,
        })
    }

    fn set_property(&mut self, name: &str, value: Value) -> Result<bool, Thrown> {
        let c = Self::CLASS;
        match name {
            "length" => self.length = pv::opt_int(&value, c, name)?,
            "id" => self.id = pv::opt_bool(&value, c, name)?,
            "idPosition" => self.id_position = pv::opt_int(&value, c, name)?,
            "nullable" => self.nullable = pv::opt_bool(&value, c, name)?,
            "notInsertable" => self.not_insertable = pv::opt_bool(&value, c, name)?,
            "notUpdatable" => self.not_updatable = pv::opt_bool(&value, c, name)?,
            "columnDefinition" => self.column_definition = pv::opt_string(&value, c, name)?,
            "generated" => self.generated = pv::opt_int(&value, c, name)?,
            "enumType" => self.enum_type = pv::opt_string(&value, c, name)?,
            "precision" => self.precision = pv::opt_int(&value, c, name)?,
            "scale" => self.scale = pv::opt_int(&value, c, name)?,
            "unique" => self.unique = pv::opt_bool(&value, c, name)?,
            "index" => self.index = pv::opt_bool(&value, c, name)?,
            "inherited" => self.inherited = pv::opt_string(&value, c, name)?,
            "originalClass" => self.original_class = pv::opt_string(&value, c, name)?,
            "originalField" => self.original_field = pv::opt_string(&value, c, name)?,
            "quoted" => self.quoted = pv::opt_bool(&value, c, name)?,
            "declared" => self.declared = pv::opt_string(&value, c, name)?,
            "declaredField" => self.declared_field = pv::opt_string(&value, c, name)?,
            "options" => self.options = pv::opt_array(&value, c, name)?,
            "version" => self.version = pv::opt_bool(&value, c, name)?,
            "default" => {
                if !matches!(value, Value::Null | Value::Int(_) | Value::String(_)) {
                    return Err(pv::cannot_assign(&value, c, name, "string|int|null"));
                }
                self.default = value;
            }
            "type" => self.r#type = pv::string(&value, c, name)?,
            "fieldName" => self.field_name = pv::string(&value, c, name)?,
            "columnName" => self.column_name = pv::string(&value, c, name)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}
