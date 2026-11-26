//! The naming strategies: how table, column and join names are derived
//! from class and property names.
//!
//! Upstream: doctrine/orm src/Mapping/NamingStrategy.php src/Mapping/DefaultNamingStrategy.php src/Mapping/UnderscoreNamingStrategy.php

use std::fmt;
use std::sync::atomic::{AtomicI64, Ordering};

/// PHP's `CASE_LOWER`.
pub const CASE_LOWER: i64 = 0;
/// PHP's `CASE_UPPER`.
pub const CASE_UPPER: i64 = 1;

/// `Doctrine\ORM\Mapping\NamingStrategy`: a set of rules for determining
/// the physical column and table names.
pub trait NamingStrategy: Send + Sync + fmt::Debug {
    /// Returns a table name for an entity class.
    fn class_to_table_name(&self, class_name: &str) -> String;

    /// Returns a column name for a property.
    fn property_to_column_name(&self, property_name: &str, class_name: &str) -> String;

    /// Returns a column name for an embedded property.
    fn embedded_field_to_column_name(
        &self,
        property_name: &str,
        embedded_column_name: &str,
        class_name: &str,
        embedded_class_name: &str,
    ) -> String;

    /// Returns the default reference column name.
    fn reference_column_name(&self) -> String;

    /// Returns a join column name for a property.
    fn join_column_name(&self, property_name: &str, class_name: &str) -> String;

    /// Returns a join table name.
    fn join_table_name(
        &self,
        source_entity: &str,
        target_entity: &str,
        property_name: &str,
    ) -> String;

    /// Returns the foreign key column name for the given parameters.
    fn join_key_column_name(
        &self,
        entity_name: &str,
        referenced_column_name: Option<&str>,
    ) -> String;
}

/// The short name of a class (after the last backslash).
fn short_name(class_name: &str) -> &str {
    class_name
        .rfind('\\')
        .map_or(class_name, |at| &class_name[at + 1..])
}

/// `$referencedColumnName ?: $default`.
fn or_default(referenced_column_name: Option<&str>, default: String) -> String {
    match referenced_column_name {
        Some(name) if !name.is_empty() && name != "0" => name.to_owned(),
        _ => default,
    }
}

/// `Doctrine\ORM\Mapping\DefaultNamingStrategy`: the default naming
/// strategy.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultNamingStrategy;

impl DefaultNamingStrategy {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Mapping\\DefaultNamingStrategy";

    pub fn new() -> Self {
        Self
    }
}

impl NamingStrategy for DefaultNamingStrategy {
    fn class_to_table_name(&self, class_name: &str) -> String {
        short_name(class_name).to_owned()
    }

    fn property_to_column_name(&self, property_name: &str, _class_name: &str) -> String {
        property_name.to_owned()
    }

    fn embedded_field_to_column_name(
        &self,
        property_name: &str,
        embedded_column_name: &str,
        _class_name: &str,
        _embedded_class_name: &str,
    ) -> String {
        format!("{property_name}_{embedded_column_name}")
    }

    fn reference_column_name(&self) -> String {
        "id".to_owned()
    }

    fn join_column_name(&self, property_name: &str, _class_name: &str) -> String {
        format!("{property_name}_{}", self.reference_column_name())
    }

    fn join_table_name(
        &self,
        source_entity: &str,
        target_entity: &str,
        _property_name: &str,
    ) -> String {
        format!(
            "{}_{}",
            self.class_to_table_name(source_entity),
            self.class_to_table_name(target_entity)
        )
        .to_ascii_lowercase()
    }

    fn join_key_column_name(
        &self,
        entity_name: &str,
        referenced_column_name: Option<&str>,
    ) -> String {
        format!(
            "{}_{}",
            self.class_to_table_name(entity_name),
            or_default(referenced_column_name, self.reference_column_name())
        )
        .to_ascii_lowercase()
    }
}

/// `Doctrine\ORM\Mapping\UnderscoreNamingStrategy`: underscore naming,
/// lower case by default.
#[derive(Debug)]
pub struct UnderscoreNamingStrategy {
    case: AtomicI64,
}

impl Default for UnderscoreNamingStrategy {
    fn default() -> Self {
        Self::new(CASE_LOWER)
    }
}

impl UnderscoreNamingStrategy {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Mapping\\UnderscoreNamingStrategy";

    /// `CASE_LOWER` or `CASE_UPPER`.
    pub fn new(case: i64) -> Self {
        Self {
            case: AtomicI64::new(case),
        }
    }

    pub fn get_case(&self) -> i64 {
        self.case.load(Ordering::Relaxed)
    }

    /// Sets string case `CASE_LOWER` | `CASE_UPPER`. Alphanumeric
    /// characters in the string will be converted to the specified case.
    pub fn set_case(&self, case: i64) {
        self.case.store(case, Ordering::Relaxed);
    }

    /// `preg_replace('/(?<=[a-z0-9])([A-Z])/', '_$1', $string)`, then the
    /// case.
    fn underscore(&self, string: &str) -> String {
        let bytes = string.as_bytes();
        let mut out = Vec::with_capacity(bytes.len() + 4);
        for (at, &byte) in bytes.iter().enumerate() {
            if byte.is_ascii_uppercase()
                && at > 0
                && (bytes[at - 1].is_ascii_lowercase() || bytes[at - 1].is_ascii_digit())
            {
                out.push(b'_');
            }
            out.push(byte);
        }
        let string = String::from_utf8_lossy(&out).into_owned();
        if self.get_case() == CASE_UPPER {
            string.to_ascii_uppercase()
        } else {
            string.to_ascii_lowercase()
        }
    }
}

impl NamingStrategy for UnderscoreNamingStrategy {
    fn class_to_table_name(&self, class_name: &str) -> String {
        self.underscore(short_name(class_name))
    }

    fn property_to_column_name(&self, property_name: &str, _class_name: &str) -> String {
        self.underscore(property_name)
    }

    fn embedded_field_to_column_name(
        &self,
        property_name: &str,
        embedded_column_name: &str,
        _class_name: &str,
        _embedded_class_name: &str,
    ) -> String {
        format!("{}_{embedded_column_name}", self.underscore(property_name))
    }

    fn reference_column_name(&self) -> String {
        if self.get_case() == CASE_UPPER {
            "ID".to_owned()
        } else {
            "id".to_owned()
        }
    }

    fn join_column_name(&self, property_name: &str, _class_name: &str) -> String {
        format!(
            "{}_{}",
            self.underscore(property_name),
            self.reference_column_name()
        )
    }

    fn join_table_name(
        &self,
        source_entity: &str,
        target_entity: &str,
        _property_name: &str,
    ) -> String {
        format!(
            "{}_{}",
            self.class_to_table_name(source_entity),
            self.class_to_table_name(target_entity)
        )
    }

    fn join_key_column_name(
        &self,
        entity_name: &str,
        referenced_column_name: Option<&str>,
    ) -> String {
        format!(
            "{}_{}",
            self.class_to_table_name(entity_name),
            or_default(referenced_column_name, self.reference_column_name())
        )
    }
}
