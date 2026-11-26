//! `EmbeddedClassMapping` and `DiscriminatorColumnMapping`.
//!
//! Upstream: doctrine/orm src/Mapping/EmbeddedClassMapping.php src/Mapping/DiscriminatorColumnMapping.php

use php_value::{PhpArray, Thrown, Value};

use crate::mapping::field_mapping::MappingProperties;
use crate::mapping::property_value::{self as pv, Sleep, from_opt_int, from_opt_string};

/// `columnPrefix`: `string|false|null`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ColumnPrefix {
    #[default]
    Null,
    False,
    Prefix(String),
}

impl ColumnPrefix {
    pub fn to_value(&self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::False => Value::Bool(false),
            Self::Prefix(prefix) => Value::from(prefix.as_str()),
        }
    }

    fn from_value(value: &Value, class: &str, property: &str) -> Result<Self, Thrown> {
        match value {
            Value::Null => Ok(Self::Null),
            Value::Bool(false) => Ok(Self::False),
            Value::String(s) => Ok(Self::Prefix(pv::text(s))),
            other => Err(pv::cannot_assign(
                other,
                class,
                property,
                "string|false|null",
            )),
        }
    }

    /// PHP's truthiness of the value.
    pub fn is_truthy(&self) -> bool {
        matches!(self, Self::Prefix(p) if !p.is_empty() && p != "0")
    }
}

/// `Doctrine\ORM\Mapping\EmbeddedClassMapping`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedClassMapping {
    pub column_prefix: ColumnPrefix,
    pub declared_field: Option<String>,
    pub original_field: Option<String>,
    /// The class the embedded is inherited from.
    pub inherited: Option<String>,
    /// The class that declares the embedded.
    pub declared: Option<String>,
    pub class: String,
}

impl EmbeddedClassMapping {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Mapping\\EmbeddedClassMapping";

    pub const PROPERTIES: [&'static str; 6] = [
        "columnPrefix",
        "declaredField",
        "originalField",
        "inherited",
        "declared",
        "class",
    ];

    pub fn new(class: impl Into<String>) -> Self {
        Self {
            column_prefix: ColumnPrefix::Null,
            declared_field: None,
            original_field: None,
            inherited: None,
            declared: None,
            class: class.into(),
        }
    }

    /// `EmbeddedClassMapping::fromMappingArray()`.
    pub fn from_mapping_array(mapping_array: &PhpArray) -> Result<Self, Thrown> {
        let mut mapping = Self::new(pv::string(
            pv::required(mapping_array, "class"),
            Self::CLASS,
            "class",
        )?);
        for (key, value) in mapping_array.iter() {
            let key = pv::key_name(key);
            if key == "class" {
                continue;
            }
            mapping.set_property(&key, value.clone())?;
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

    /// `__sleep()`.
    pub fn sleep_names(&self) -> Vec<&'static str> {
        let mut serialized = vec!["class"];
        if self.column_prefix.is_truthy() {
            serialized.push("columnPrefix");
        }
        for name in ["declaredField", "originalField", "inherited", "declared"] {
            if !matches!(self.property(name), Some(Value::Null) | None) {
                serialized.push(name);
            }
        }
        serialized
    }

    /// `unserialize()` of a serialized `EmbeddedClassMapping`.
    pub fn unserialize(data: &[u8]) -> Option<Self> {
        let (class, properties) = pv::unserialize_object(data, &[Self::CLASS])?;
        if class != Self::CLASS {
            return None;
        }
        let mut mapping =
            Self::new(pv::string(pv::required(&properties, "class"), Self::CLASS, "class").ok()?);
        for (key, value) in properties.iter() {
            mapping
                .set_property(&pv::key_name(key), value.clone())
                .ok()?;
        }
        Some(mapping)
    }
}

impl Sleep for EmbeddedClassMapping {
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

impl MappingProperties for EmbeddedClassMapping {
    const CLASS: &'static str = EmbeddedClassMapping::CLASS;

    fn property(&self, name: &str) -> Option<Value> {
        Some(match name {
            "columnPrefix" => self.column_prefix.to_value(),
            "declaredField" => from_opt_string(self.declared_field.as_ref()),
            "originalField" => from_opt_string(self.original_field.as_ref()),
            "inherited" => from_opt_string(self.inherited.as_ref()),
            "declared" => from_opt_string(self.declared.as_ref()),
            "class" => Value::from(self.class.as_str()),
            _ => return None,
        })
    }

    fn set_property(&mut self, name: &str, value: Value) -> Result<bool, Thrown> {
        let c = Self::CLASS;
        match name {
            "columnPrefix" => self.column_prefix = ColumnPrefix::from_value(&value, c, name)?,
            "declaredField" => self.declared_field = pv::opt_string(&value, c, name)?,
            "originalField" => self.original_field = pv::opt_string(&value, c, name)?,
            "inherited" => self.inherited = pv::opt_string(&value, c, name)?,
            "declared" => self.declared = pv::opt_string(&value, c, name)?,
            "class" => self.class = pv::string(&value, c, name)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// `Doctrine\ORM\Mapping\DiscriminatorColumnMapping`.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscriminatorColumnMapping {
    /// The database length of the column. Optional. Default value taken
    /// from the type.
    pub length: Option<i64>,
    pub column_definition: Option<String>,
    pub enum_type: Option<String>,
    pub options: PhpArray,
    pub r#type: String,
    pub field_name: String,
    pub name: String,
}

impl DiscriminatorColumnMapping {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Mapping\\DiscriminatorColumnMapping";

    pub const PROPERTIES: [&'static str; 7] = [
        "length",
        "columnDefinition",
        "enumType",
        "options",
        "type",
        "fieldName",
        "name",
    ];

    pub fn new(
        r#type: impl Into<String>,
        field_name: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            length: None,
            column_definition: None,
            enum_type: None,
            options: PhpArray::new(),
            r#type: r#type.into(),
            field_name: field_name.into(),
            name: name.into(),
        }
    }

    /// `DiscriminatorColumnMapping::fromMappingArray()`: a null value
    /// keeps the default; an unknown key is an `Exception`.
    pub fn from_mapping_array(mapping_array: &PhpArray) -> Result<Self, Thrown> {
        let c = Self::CLASS;
        let mut mapping = Self::new(
            pv::string(pv::required(mapping_array, "type"), c, "type")?,
            pv::string(pv::required(mapping_array, "fieldName"), c, "fieldName")?,
            pv::string(pv::required(mapping_array, "name"), c, "name")?,
        );
        for (key, value) in mapping_array.iter() {
            let key = pv::key_name(key);
            if ["type", "fieldName", "name"].contains(&key.as_str()) {
                continue;
            }
            if Self::PROPERTIES.contains(&key.as_str()) {
                let value = if matches!(value, Value::Null) {
                    mapping.property(&key).unwrap_or_default()
                } else {
                    value.clone()
                };
                mapping.set_property(&key, value)?;
            } else {
                return Err(Thrown::new(
                    "Exception",
                    format!("Unknown property {key} on class {c}"),
                    0,
                ));
            }
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

    /// `__sleep()`.
    pub fn sleep_names(&self) -> Vec<&'static str> {
        let mut serialized = vec!["type", "fieldName", "name"];
        for name in ["length", "columnDefinition", "enumType", "options"] {
            if !matches!(self.property(name), Some(Value::Null) | None) {
                serialized.push(name);
            }
        }
        serialized
    }

    /// `unserialize()` of a serialized `DiscriminatorColumnMapping`.
    pub fn unserialize(data: &[u8]) -> Option<Self> {
        let (class, properties) = pv::unserialize_object(data, &[Self::CLASS])?;
        if class != Self::CLASS {
            return None;
        }
        let c = Self::CLASS;
        let mut mapping = Self::new(
            pv::string(pv::required(&properties, "type"), c, "type").ok()?,
            pv::string(pv::required(&properties, "fieldName"), c, "fieldName").ok()?,
            pv::string(pv::required(&properties, "name"), c, "name").ok()?,
        );
        for (key, value) in properties.iter() {
            mapping
                .set_property(&pv::key_name(key), value.clone())
                .ok()?;
        }
        Some(mapping)
    }
}

impl Sleep for DiscriminatorColumnMapping {
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

impl MappingProperties for DiscriminatorColumnMapping {
    const CLASS: &'static str = DiscriminatorColumnMapping::CLASS;

    fn property(&self, name: &str) -> Option<Value> {
        Some(match name {
            "length" => from_opt_int(self.length),
            "columnDefinition" => from_opt_string(self.column_definition.as_ref()),
            "enumType" => from_opt_string(self.enum_type.as_ref()),
            "options" => Value::Array(self.options.clone()),
            "type" => Value::from(self.r#type.as_str()),
            "fieldName" => Value::from(self.field_name.as_str()),
            "name" => Value::from(self.name.as_str()),
            _ => return None,
        })
    }

    fn set_property(&mut self, name: &str, value: Value) -> Result<bool, Thrown> {
        let c = Self::CLASS;
        match name {
            "length" => self.length = pv::opt_int(&value, c, name)?,
            "columnDefinition" => self.column_definition = pv::opt_string(&value, c, name)?,
            "enumType" => self.enum_type = pv::opt_string(&value, c, name)?,
            "options" => self.options = pv::array(&value, c, name)?,
            "type" => self.r#type = pv::string(&value, c, name)?,
            "fieldName" => self.field_name = pv::string(&value, c, name)?,
            "name" => self.name = pv::string(&value, c, name)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}
