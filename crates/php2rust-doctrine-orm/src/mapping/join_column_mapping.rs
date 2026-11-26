//! `JoinColumnMapping` and `JoinTableMapping`: the join columns of a
//! to-one owning side and the join table of a many-to-many owning side.
//!
//! Upstream: doctrine/orm src/Mapping/JoinColumnMapping.php src/Mapping/JoinTableMapping.php

use std::sync::Arc;

use php_value::object::GenericObject;
use php_value::{PhpArray, Thrown, Value};

use crate::mapping::field_mapping::{MappingProperties, trigger_array_access};
use crate::mapping::property_value::{
    self as pv, Sleep, from_opt_array, from_opt_bool, from_opt_string,
};

/// `Doctrine\ORM\Mapping\JoinColumnMapping`.
#[derive(Debug, Clone, PartialEq)]
pub struct JoinColumnMapping {
    pub deferrable: Option<bool>,
    pub unique: Option<bool>,
    pub quoted: Option<bool>,
    pub field_name: Option<String>,
    pub on_delete: Option<String>,
    pub column_definition: Option<String>,
    pub nullable: Option<bool>,
    pub foreign_key_name: Option<String>,
    pub options: Option<PhpArray>,
    pub name: String,
    pub referenced_column_name: String,
}

impl JoinColumnMapping {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Mapping\\JoinColumnMapping";

    pub const PROPERTIES: [&'static str; 11] = [
        "deferrable",
        "unique",
        "quoted",
        "fieldName",
        "onDelete",
        "columnDefinition",
        "nullable",
        "foreignKeyName",
        "options",
        "name",
        "referencedColumnName",
    ];

    pub fn new(name: impl Into<String>, referenced_column_name: impl Into<String>) -> Self {
        Self {
            deferrable: None,
            unique: None,
            quoted: None,
            field_name: None,
            on_delete: None,
            column_definition: None,
            nullable: None,
            foreign_key_name: None,
            options: None,
            name: name.into(),
            referenced_column_name: referenced_column_name.into(),
        }
    }

    /// `JoinColumnMapping::fromMappingArray()`: null values are skipped.
    pub fn from_mapping_array(mapping_array: &PhpArray) -> Result<Self, Thrown> {
        let c = Self::CLASS;
        let mut mapping = Self::new(
            pv::string(pv::required(mapping_array, "name"), c, "name")?,
            pv::string(
                pv::required(mapping_array, "referencedColumnName"),
                c,
                "referencedColumnName",
            )?,
        );
        for (key, value) in mapping_array.iter() {
            let key = pv::key_name(key);
            if Self::PROPERTIES.contains(&key.as_str()) && !matches!(value, Value::Null) {
                mapping.set_property(&key, value.clone())?;
            }
        }
        Ok(mapping)
    }

    /// From a value that is a mapping array or a (serialized)
    /// `JoinColumnMapping` object.
    pub(crate) fn from_value(value: &Value) -> Result<Self, Thrown> {
        match value {
            Value::Object(object) => Self::from_properties(&object.properties()),
            Value::Array(array) => Self::from_mapping_array(array),
            other => Err(pv::cannot_assign(other, Self::CLASS, "name", "string")),
        }
    }

    /// The object from its (serialized) properties.
    pub fn from_properties(properties: &PhpArray) -> Result<Self, Thrown> {
        let c = Self::CLASS;
        let mut mapping = Self::new(
            pv::string(pv::required(properties, "name"), c, "name")?,
            pv::string(
                pv::required(properties, "referencedColumnName"),
                c,
                "referencedColumnName",
            )?,
        );
        for (key, value) in properties.iter() {
            mapping.set_property(&pv::key_name(key), value.clone())?;
        }
        Ok(mapping)
    }

    /// `(array) $mapping`.
    pub fn to_array(&self) -> PhpArray {
        let mut array = PhpArray::new();
        for name in Self::PROPERTIES {
            array.insert(name, self.property(name).unwrap_or_default());
        }
        array
    }

    /// The object as a PHP value (its properties as an object).
    pub(crate) fn to_object(&self) -> Value {
        Value::Object(Arc::new(GenericObject::new(Self::CLASS, self.sleep())))
    }

    /// `__sleep()`.
    pub fn sleep_names(&self) -> Vec<&'static str> {
        let mut serialized = Vec::new();
        for name in [
            "columnDefinition",
            "fieldName",
            "foreignKeyName",
            "name",
            "onDelete",
            "options",
            "referencedColumnName",
        ] {
            if !matches!(self.property(name), Some(Value::Null) | None) {
                serialized.push(name);
            }
        }
        for (name, value) in [
            ("deferrable", self.deferrable),
            ("unique", self.unique),
            ("quoted", self.quoted),
            ("nullable", self.nullable),
        ] {
            if value.is_some() {
                serialized.push(name);
            }
        }
        serialized
    }

    /// `unserialize()` of a serialized `JoinColumnMapping`.
    pub fn unserialize(data: &[u8]) -> Option<Self> {
        let (class, properties) = pv::unserialize_object(data, &[Self::CLASS])?;
        (class == Self::CLASS)
            .then(|| Self::from_properties(&properties).ok())
            .flatten()
    }
}

impl Sleep for JoinColumnMapping {
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

impl MappingProperties for JoinColumnMapping {
    const CLASS: &'static str = JoinColumnMapping::CLASS;

    fn property(&self, name: &str) -> Option<Value> {
        Some(match name {
            "deferrable" => from_opt_bool(self.deferrable),
            "unique" => from_opt_bool(self.unique),
            "quoted" => from_opt_bool(self.quoted),
            "fieldName" => from_opt_string(self.field_name.as_ref()),
            "onDelete" => from_opt_string(self.on_delete.as_ref()),
            "columnDefinition" => from_opt_string(self.column_definition.as_ref()),
            "nullable" => from_opt_bool(self.nullable),
            "foreignKeyName" => from_opt_string(self.foreign_key_name.as_ref()),
            "options" => from_opt_array(self.options.as_ref()),
            "name" => Value::from(self.name.as_str()),
            "referencedColumnName" => Value::from(self.referenced_column_name.as_str()),
            _ => return None,
        })
    }

    fn set_property(&mut self, name: &str, value: Value) -> Result<bool, Thrown> {
        let c = Self::CLASS;
        match name {
            "deferrable" => self.deferrable = pv::opt_bool(&value, c, name)?,
            "unique" => self.unique = pv::opt_bool(&value, c, name)?,
            "quoted" => self.quoted = pv::opt_bool(&value, c, name)?,
            "fieldName" => self.field_name = pv::opt_string(&value, c, name)?,
            "onDelete" => self.on_delete = pv::opt_string(&value, c, name)?,
            "columnDefinition" => self.column_definition = pv::opt_string(&value, c, name)?,
            "nullable" => self.nullable = pv::opt_bool(&value, c, name)?,
            "foreignKeyName" => self.foreign_key_name = pv::opt_string(&value, c, name)?,
            "options" => self.options = pv::opt_array(&value, c, name)?,
            "name" => self.name = pv::string(&value, c, name)?,
            "referencedColumnName" => self.referenced_column_name = pv::string(&value, c, name)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// `Doctrine\ORM\Mapping\JoinTableMapping`.
#[derive(Debug, Clone, PartialEq)]
pub struct JoinTableMapping {
    pub quoted: Option<bool>,
    pub foreign_key_name: Option<String>,
    pub inverse_foreign_key_name: Option<String>,
    pub join_columns: Vec<JoinColumnMapping>,
    pub inverse_join_columns: Vec<JoinColumnMapping>,
    pub options: PhpArray,
    pub schema: Option<String>,
    pub name: String,
}

fn join_columns_value(columns: &[JoinColumnMapping]) -> Value {
    Value::Array(PhpArray::list(
        columns.iter().map(JoinColumnMapping::to_object),
    ))
}

fn join_columns_from(
    value: &Value,
    class: &str,
    property: &str,
) -> Result<Vec<JoinColumnMapping>, Thrown> {
    pv::array(value, class, property)?
        .values()
        .map(JoinColumnMapping::from_value)
        .collect()
}

impl JoinTableMapping {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Mapping\\JoinTableMapping";

    pub const PROPERTIES: [&'static str; 8] = [
        "quoted",
        "foreignKeyName",
        "inverseForeignKeyName",
        "joinColumns",
        "inverseJoinColumns",
        "options",
        "schema",
        "name",
    ];

    pub fn new(name: impl Into<String>) -> Self {
        Self {
            quoted: None,
            foreign_key_name: None,
            inverse_foreign_key_name: None,
            join_columns: Vec::new(),
            inverse_join_columns: Vec::new(),
            options: PhpArray::new(),
            schema: None,
            name: name.into(),
        }
    }

    /// `JoinTableMapping::fromMappingArray()`.
    pub fn from_mapping_array(mapping_array: &PhpArray) -> Result<Self, Thrown> {
        let c = Self::CLASS;
        let mut mapping = Self::new(pv::string(pv::required(mapping_array, "name"), c, "name")?);
        for key in ["quoted", "schema", "options"] {
            if pv::isset(mapping_array, key) {
                mapping.set_property(key, pv::required(mapping_array, key).clone())?;
            }
        }
        for key in ["foreignKeyName", "inverseForeignKeyName"] {
            if pv::isset(mapping_array, key) {
                mapping.set_property(key, pv::required(mapping_array, key).clone())?;
            }
        }
        for (key, target) in [
            ("joinColumns", &mut mapping.join_columns),
            ("inverseJoinColumns", &mut mapping.inverse_join_columns),
        ] {
            if pv::isset(mapping_array, key)
                && let Value::Array(columns) = pv::required(mapping_array, key)
            {
                for column in columns.values() {
                    target.push(JoinColumnMapping::from_value(column)?);
                }
            }
        }
        Ok(mapping)
    }

    /// The object from its (serialized) properties.
    pub fn from_properties(properties: &PhpArray) -> Result<Self, Thrown> {
        let mut mapping = Self::new(pv::string(
            pv::required(properties, "name"),
            Self::CLASS,
            "name",
        )?);
        for (key, value) in properties.iter() {
            mapping.set_property(&pv::key_name(key), value.clone())?;
        }
        Ok(mapping)
    }

    pub(crate) fn from_value(value: &Value) -> Result<Self, Thrown> {
        match value {
            Value::Object(object) => Self::from_properties(&object.properties()),
            Value::Array(array) => Self::from_mapping_array(array),
            other => Err(pv::cannot_assign(other, Self::CLASS, "name", "string")),
        }
    }

    /// `toArray()`: the join columns as arrays, without `nullable`.
    pub fn to_array(&self) -> PhpArray {
        let mut array = PhpArray::new();
        for name in Self::PROPERTIES {
            let value = match name {
                "joinColumns" | "inverseJoinColumns" => {
                    let columns = if name == "joinColumns" {
                        &self.join_columns
                    } else {
                        &self.inverse_join_columns
                    };
                    Value::Array(PhpArray::list(columns.iter().map(|column| {
                        let mut column = column.to_array();
                        column.remove(&php_value::Key::from("nullable"));
                        Value::Array(column)
                    })))
                }
                _ => self.property(name).unwrap_or_default(),
            };
            array.insert(name, value);
        }
        array
    }

    pub(crate) fn to_object(&self) -> Value {
        Value::Object(Arc::new(GenericObject::new(Self::CLASS, self.sleep())))
    }

    /// `offsetSet()`: join columns are given as mapping arrays.
    pub fn offset_set(&mut self, offset: &str, value: Value) -> Result<(), Thrown> {
        trigger_array_access(Self::CLASS);
        self.set_property(offset, value).map(|_| ())
    }

    /// `__sleep()`.
    pub fn sleep_names(&self) -> Vec<&'static str> {
        let mut serialized = Vec::new();
        for name in [
            "joinColumns",
            "inverseJoinColumns",
            "name",
            "schema",
            "options",
            "foreignKeyName",
            "inverseForeignKeyName",
        ] {
            if !matches!(self.property(name), Some(Value::Null) | None) {
                serialized.push(name);
            }
        }
        if self.quoted == Some(true) {
            serialized.push("quoted");
        }
        serialized
    }

    /// `unserialize()` of a serialized `JoinTableMapping`.
    pub fn unserialize(data: &[u8]) -> Option<Self> {
        let (class, properties) =
            pv::unserialize_object(data, &[Self::CLASS, JoinColumnMapping::CLASS])?;
        (class == Self::CLASS)
            .then(|| Self::from_properties(&properties).ok())
            .flatten()
    }
}

impl Sleep for JoinTableMapping {
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

impl MappingProperties for JoinTableMapping {
    const CLASS: &'static str = JoinTableMapping::CLASS;

    fn property(&self, name: &str) -> Option<Value> {
        Some(match name {
            "quoted" => from_opt_bool(self.quoted),
            "foreignKeyName" => from_opt_string(self.foreign_key_name.as_ref()),
            "inverseForeignKeyName" => from_opt_string(self.inverse_foreign_key_name.as_ref()),
            "joinColumns" => join_columns_value(&self.join_columns),
            "inverseJoinColumns" => join_columns_value(&self.inverse_join_columns),
            "options" => Value::Array(self.options.clone()),
            "schema" => from_opt_string(self.schema.as_ref()),
            "name" => Value::from(self.name.as_str()),
            _ => return None,
        })
    }

    fn set_property(&mut self, name: &str, value: Value) -> Result<bool, Thrown> {
        let c = Self::CLASS;
        match name {
            "quoted" => self.quoted = pv::opt_bool(&value, c, name)?,
            "foreignKeyName" => self.foreign_key_name = pv::opt_string(&value, c, name)?,
            "inverseForeignKeyName" => {
                self.inverse_foreign_key_name = pv::opt_string(&value, c, name)?;
            }
            "joinColumns" => self.join_columns = join_columns_from(&value, c, name)?,
            "inverseJoinColumns" => self.inverse_join_columns = join_columns_from(&value, c, name)?,
            "options" => self.options = pv::array(&value, c, name)?,
            "schema" => self.schema = pv::opt_string(&value, c, name)?,
            "name" => self.name = pv::string(&value, c, name)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}
