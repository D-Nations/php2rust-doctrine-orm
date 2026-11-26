//! The quote strategies: the SQL names of a class's table, columns,
//! sequences and join tables, quoted for the platform where the mapping
//! asks for it.
//!
//! Upstream: doctrine/orm src/Mapping/QuoteStrategy.php src/Mapping/DefaultQuoteStrategy.php src/Mapping/AnsiQuoteStrategy.php src/Internal/SQLResultCasing.php

use std::fmt;

use php_value::{PhpArray, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_persistence::php_reflection;

use crate::mapping::association_mapping::AssociationMapping;
use crate::mapping::class_metadata::ClassMetadataInfo;
use crate::mapping::join_column_mapping::JoinColumnMapping;
use crate::mapping::property_value as pv;

/// `Doctrine\ORM\Mapping\QuoteStrategy`.
pub trait QuoteStrategy: Send + Sync + fmt::Debug {
    /// Gets the (possibly quoted) column name for safe use in an SQL
    /// statement.
    fn get_column_name(
        &self,
        field_name: &str,
        class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> String;

    /// Gets the (possibly quoted) primary table name for safe use in an
    /// SQL statement.
    fn get_table_name(&self, class: &ClassMetadataInfo, platform: &dyn AbstractPlatform)
    -> String;

    /// Gets the (possibly quoted) sequence name for safe use in an SQL
    /// statement.
    fn get_sequence_name(
        &self,
        definition: &PhpArray,
        class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> String;

    /// Gets the (possibly quoted) name of the join table (of a
    /// many-to-many owning side).
    fn get_join_table_name(
        &self,
        association: &AssociationMapping,
        class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> String;

    /// Gets the (possibly quoted) join column name.
    fn get_join_column_name(
        &self,
        join_column: &JoinColumnMapping,
        class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> String;

    /// Gets the (possibly quoted) join column name of the referenced
    /// column.
    fn get_referenced_join_column_name(
        &self,
        join_column: &JoinColumnMapping,
        class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> String;

    /// Gets the (possibly quoted) identifier column names for safe use in
    /// an SQL statement.
    fn get_identifier_column_names(
        &self,
        class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> Vec<String>;

    /// Gets the column alias.
    fn get_column_alias(
        &self,
        column_name: &str,
        counter: i64,
        platform: &dyn AbstractPlatform,
        class: Option<&ClassMetadataInfo>,
    ) -> String;
}

const DB2_PLATFORM: &str = "Doctrine\\DBAL\\Platforms\\DB2Platform";
const ORACLE_PLATFORM: &str = "Doctrine\\DBAL\\Platforms\\OraclePlatform";
const POSTGRESQL_PLATFORM: &str = "Doctrine\\DBAL\\Platforms\\PostgreSQLPlatform";

/// `$platform instanceof $class`.
pub(crate) fn platform_is_a(platform: &dyn AbstractPlatform, class: &str) -> bool {
    platform.class().eq_ignore_ascii_case(class) || php_reflection::is_a(platform.class(), class)
}

/// `SQLResultCasing::getSQLResultCasing()`.
fn get_sql_result_casing(platform: &dyn AbstractPlatform, column: &str) -> String {
    if platform_is_a(platform, DB2_PLATFORM) || platform_is_a(platform, ORACLE_PLATFORM) {
        return column.to_ascii_uppercase();
    }
    if platform_is_a(platform, POSTGRESQL_PLATFORM) {
        return column.to_ascii_lowercase();
    }
    column.to_owned()
}

fn table_entry(class: &ClassMetadataInfo, key: &str) -> Option<String> {
    match pv::get(&class.table, key)? {
        Value::Null => None,
        value => value.to_php_string().map(|s| pv::text(&s)),
    }
}

fn column_name(class: &ClassMetadataInfo, field_name: &str) -> String {
    class
        .field_mappings
        .get(field_name)
        .map(|m| m.column_name.clone())
        .unwrap_or_default()
}

/// `Doctrine\ORM\Mapping\DefaultQuoteStrategy`: quotes the names the
/// mapping marks as quoted.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultQuoteStrategy;

impl DefaultQuoteStrategy {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Mapping\\DefaultQuoteStrategy";

    pub fn new() -> Self {
        Self
    }
}

impl QuoteStrategy for DefaultQuoteStrategy {
    fn get_column_name(
        &self,
        field_name: &str,
        class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> String {
        let column = column_name(class, field_name);
        if class
            .field_mappings
            .get(field_name)
            .is_some_and(|m| m.quoted.is_some())
        {
            platform.quote_single_identifier(&column)
        } else {
            column
        }
    }

    fn get_table_name(
        &self,
        class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> String {
        let table_name = table_entry(class, "name").unwrap_or_default();
        let quoted = pv::isset(&class.table, "quoted");
        if let Some(schema) = table_entry(class, "schema").filter(|s| !s.is_empty() && s != "0") {
            return if quoted {
                format!(
                    "{}.{}",
                    platform.quote_single_identifier(&schema),
                    platform.quote_single_identifier(&table_name)
                )
            } else {
                format!("{schema}.{table_name}")
            };
        }
        if quoted {
            platform.quote_single_identifier(&table_name)
        } else {
            table_name
        }
    }

    fn get_sequence_name(
        &self,
        definition: &PhpArray,
        _class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> String {
        let name = pv::get(definition, "sequenceName")
            .and_then(Value::to_php_string)
            .map(|s| pv::text(&s))
            .unwrap_or_default();
        if pv::isset(definition, "quoted") {
            name.split('.')
                .map(|part| platform.quote_single_identifier(part))
                .collect::<Vec<_>>()
                .join(".")
        } else {
            name
        }
    }

    fn get_join_column_name(
        &self,
        join_column: &JoinColumnMapping,
        _class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> String {
        if join_column.quoted.is_some() {
            platform.quote_single_identifier(&join_column.name)
        } else {
            join_column.name.clone()
        }
    }

    fn get_referenced_join_column_name(
        &self,
        join_column: &JoinColumnMapping,
        _class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> String {
        if join_column.quoted.is_some() {
            platform.quote_single_identifier(&join_column.referenced_column_name)
        } else {
            join_column.referenced_column_name.clone()
        }
    }

    fn get_join_table_name(
        &self,
        association: &AssociationMapping,
        _class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> String {
        let Some(join_table) = &association.join_table else {
            return String::new();
        };
        let schema = join_table
            .schema
            .as_ref()
            .map_or_else(String::new, |schema| format!("{schema}."));
        let table_name = if join_table.quoted.is_some() {
            platform.quote_single_identifier(&join_table.name)
        } else {
            join_table.name.clone()
        };
        schema + &table_name
    }

    fn get_identifier_column_names(
        &self,
        class: &ClassMetadataInfo,
        platform: &dyn AbstractPlatform,
    ) -> Vec<String> {
        let mut quoted_column_names = Vec::new();
        for field_name in &class.identifier {
            if class.field_mappings.contains_key(field_name) {
                quoted_column_names.push(self.get_column_name(field_name, class, platform));
                continue;
            }
            // Association defined as Id field
            let Some(assoc) = class.association_mappings.get(field_name) else {
                continue;
            };
            quoted_column_names.extend(assoc.join_columns.iter().map(|join_column| {
                if join_column.quoted.is_some() {
                    platform.quote_single_identifier(&join_column.name)
                } else {
                    join_column.name.clone()
                }
            }));
        }
        quoted_column_names
    }

    fn get_column_alias(
        &self,
        column_name: &str,
        counter: i64,
        platform: &dyn AbstractPlatform,
        _class: Option<&ClassMetadataInfo>,
    ) -> String {
        // 1 ) Concatenate column name and counter
        // 2 ) Trim the column alias to the maximum identifier length of the platform.
        //     If the alias is to long, characters are cut off from the beginning.
        // 3 ) Strip non alphanumeric characters
        // 4 ) Prefix with "_" if the result its numeric
        let column_name = format!("{column_name}_{counter}");
        let max = usize::try_from(platform.get_max_identifier_length()).unwrap_or(0);
        let bytes = column_name.as_bytes();
        // substr($s, -$max): the whole string when $max is 0 or longer.
        let start = if max == 0 { 0 } else { bytes.len().saturating_sub(max) };
        let column_name: String = bytes[start..]
            .iter()
            .filter(|b| b.is_ascii_alphanumeric() || **b == b'_')
            .map(|b| char::from(*b))
            .collect();
        let column_name = if is_numeric(&column_name) {
            format!("_{column_name}")
        } else {
            column_name
        };
        get_sql_result_casing(platform, &column_name)
    }
}

/// `is_numeric()` of the alphanumeric and `_` characters the alias keeps.
fn is_numeric(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// `Doctrine\ORM\Mapping\AnsiQuoteStrategy`: ANSI SQL names, never quoted.
#[derive(Debug, Clone, Copy, Default)]
pub struct AnsiQuoteStrategy;

impl AnsiQuoteStrategy {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Mapping\\AnsiQuoteStrategy";

    pub fn new() -> Self {
        Self
    }
}

impl QuoteStrategy for AnsiQuoteStrategy {
    fn get_column_name(
        &self,
        field_name: &str,
        class: &ClassMetadataInfo,
        _platform: &dyn AbstractPlatform,
    ) -> String {
        column_name(class, field_name)
    }

    fn get_table_name(
        &self,
        class: &ClassMetadataInfo,
        _platform: &dyn AbstractPlatform,
    ) -> String {
        table_entry(class, "name").unwrap_or_default()
    }

    fn get_sequence_name(
        &self,
        definition: &PhpArray,
        _class: &ClassMetadataInfo,
        _platform: &dyn AbstractPlatform,
    ) -> String {
        pv::get(definition, "sequenceName")
            .and_then(Value::to_php_string)
            .map(|s| pv::text(&s))
            .unwrap_or_default()
    }

    fn get_join_column_name(
        &self,
        join_column: &JoinColumnMapping,
        _class: &ClassMetadataInfo,
        _platform: &dyn AbstractPlatform,
    ) -> String {
        join_column.name.clone()
    }

    fn get_referenced_join_column_name(
        &self,
        join_column: &JoinColumnMapping,
        _class: &ClassMetadataInfo,
        _platform: &dyn AbstractPlatform,
    ) -> String {
        join_column.referenced_column_name.clone()
    }

    fn get_join_table_name(
        &self,
        association: &AssociationMapping,
        _class: &ClassMetadataInfo,
        _platform: &dyn AbstractPlatform,
    ) -> String {
        association
            .join_table
            .as_ref()
            .map(|t| t.name.clone())
            .unwrap_or_default()
    }

    fn get_identifier_column_names(
        &self,
        class: &ClassMetadataInfo,
        _platform: &dyn AbstractPlatform,
    ) -> Vec<String> {
        class.identifier.clone()
    }

    fn get_column_alias(
        &self,
        column_name: &str,
        counter: i64,
        platform: &dyn AbstractPlatform,
        _class: Option<&ClassMetadataInfo>,
    ) -> String {
        get_sql_result_casing(platform, &format!("{column_name}_{counter}"))
    }
}
