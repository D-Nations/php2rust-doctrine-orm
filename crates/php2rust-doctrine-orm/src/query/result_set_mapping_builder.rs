//! A `ResultSetMappingBuilder` uses the `EntityManager` to automatically
//! populate entity fields.
//!
//! PHP's class extends `ResultSetMapping`; here the builder holds one and
//! dereferences to it (`into_result_set_mapping()` gives it).
//!
//! Upstream: doctrine/orm src/Query/ResultSetMappingBuilder.php

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::types;

use crate::entity_manager::EntityManager;
use crate::mapping::ordered_map::OrderedMap;
use crate::mapping::quote_strategy::get_sql_result_casing;
use crate::utility::get_type_of_column;

use super::ResultSetMapping;

fn invalid_argument(message: String) -> Thrown {
    Thrown::new("InvalidArgumentException", message, 0)
}

/// `Doctrine\ORM\Query\ResultSetMappingBuilder`.
#[derive(Debug, Clone)]
pub struct ResultSetMappingBuilder {
    rsm: ResultSetMapping,
    em: Arc<EntityManager>,
    default_rename_mode: i64,
    sql_counter: i64,
}

impl Deref for ResultSetMappingBuilder {
    type Target = ResultSetMapping;

    fn deref(&self) -> &ResultSetMapping {
        &self.rsm
    }
}

impl DerefMut for ResultSetMappingBuilder {
    fn deref_mut(&mut self) -> &mut ResultSetMapping {
        &mut self.rsm
    }
}

impl ResultSetMappingBuilder {
    /// Picking this rename mode will register entity columns as is, as
    /// they are in the database. This can cause clashes when multiple
    /// entities are fetched that have columns with the same name.
    pub const COLUMN_RENAMING_NONE: i64 = 1;

    /// Picking custom renaming allows the user to define the renaming of
    /// specific columns with a rename array that contains column names as
    /// keys and result alias as values.
    pub const COLUMN_RENAMING_CUSTOM: i64 = 2;

    /// Incremental renaming uses a result set mapping internal counter to
    /// add a number to each column result, leading to uniqueness. This
    /// only works if you use `generateSelectClause()` to generate the
    /// SELECT clause for you.
    pub const COLUMN_RENAMING_INCREMENT: i64 = 3;

    pub fn new(em: &Arc<EntityManager>, default_rename_mode: Option<i64>) -> Self {
        Self {
            rsm: ResultSetMapping::new(),
            em: Arc::clone(em),
            default_rename_mode: default_rename_mode.unwrap_or(Self::COLUMN_RENAMING_NONE),
            sql_counter: 0,
        }
    }

    /// The result set mapping built.
    pub fn into_result_set_mapping(self) -> ResultSetMapping {
        self.rsm
    }

    /// Adds a root entity and all of its fields to the result set.
    pub fn add_root_entity_from_class_metadata(
        &mut self,
        class: &str,
        alias: &str,
        renamed_columns: &OrderedMap<String>,
        rename_mode: Option<i64>,
    ) -> Result<(), Thrown> {
        let rename_mode = rename_mode
            .filter(|m| *m != 0)
            .unwrap_or(self.default_rename_mode);
        let column_alias_map = self.get_column_alias_map(class, rename_mode, renamed_columns)?;

        self.rsm.add_entity_result(class, alias, None);
        self.add_all_class_fields(class, alias, &column_alias_map)
    }

    /// Adds a joined entity and all of its fields to the result set.
    pub fn add_joined_entity_from_class_metadata(
        &mut self,
        class: &str,
        alias: &str,
        parent_alias: &str,
        relation: &str,
        renamed_columns: &OrderedMap<String>,
        rename_mode: Option<i64>,
    ) -> Result<(), Thrown> {
        let rename_mode = rename_mode
            .filter(|m| *m != 0)
            .unwrap_or(self.default_rename_mode);
        let column_alias_map = self.get_column_alias_map(class, rename_mode, renamed_columns)?;

        self.rsm
            .add_joined_entity_result(class, alias, parent_alias, relation, false);
        self.add_all_class_fields(class, alias, &column_alias_map)
    }

    /// Adds all fields of the given class to the result set mapping
    /// (columns and meta fields).
    fn add_all_class_fields(
        &mut self,
        class: &str,
        alias: &str,
        column_alias_map: &OrderedMap<String>,
    ) -> Result<(), Thrown> {
        let class_metadata = self.em.get_class_metadata(class)?;
        let platform = self.em.get_platform();
        let info = class_metadata.read();

        if !Self::is_inheritance_supported(&info) {
            return Err(invalid_argument(
                "ResultSetMapping builder does not currently support your inheritance scheme."
                    .to_owned(),
            ));
        }

        for column_name in info.get_column_names(None) {
            let property_name = info.get_field_name(&column_name);
            let column_alias = get_sql_result_casing(
                platform,
                column_alias_map
                    .get(&column_name)
                    .map_or("", String::as_str),
            );

            if self.rsm.field_mappings.contains_key(&column_alias) {
                return Err(invalid_argument(format!(
                    "The column '{column_name}' conflicts with another column in the mapper."
                )));
            }

            self.rsm
                .add_field_result(alias, &column_alias, &property_name, None);

            let enum_type = info
                .field_mappings
                .get(&property_name)
                .and_then(|m| m.enum_type.clone());
            if let Some(enum_type) = enum_type.filter(|e| !e.is_empty()) {
                self.rsm.add_enum_result(&column_alias, &enum_type);
            }
        }

        for association_mapping in info.association_mappings.values() {
            if !association_mapping.is_to_one_owning_side() {
                continue;
            }
            let target_class = self
                .em
                .get_class_metadata(&association_mapping.target_entity)?;
            let is_identifier = association_mapping.id == Some(true);

            for join_column in &association_mapping.join_columns {
                let column_name = &join_column.name;
                let column_alias = get_sql_result_casing(
                    platform,
                    column_alias_map.get(column_name).map_or("", String::as_str),
                );
                let column_type = get_type_of_column(
                    &join_column.referenced_column_name,
                    &target_class.read(),
                    &self.em,
                )?;

                if self.rsm.meta_mappings.contains_key(&column_alias) {
                    return Err(invalid_argument(format!(
                        "The column '{column_alias}' conflicts with another column in the mapper."
                    )));
                }

                self.rsm.add_meta_result(
                    alias,
                    &column_alias,
                    column_name,
                    is_identifier,
                    Some(&column_type),
                );
            }
        }
        Ok(())
    }

    fn is_inheritance_supported(class_metadata: &crate::mapping::ClassMetadataInfo) -> bool {
        if class_metadata.is_inheritance_type_single_table()
            && class_metadata.discriminator_map.values().any(
                |v| matches!(v, Value::String(s) if s.as_bytes() == class_metadata.name.as_bytes()),
            )
        {
            return true;
        }

        !(class_metadata.is_inheritance_type_single_table()
            || class_metadata.is_inheritance_type_joined())
    }

    /// Gets column alias for a given column.
    fn get_column_alias(
        &mut self,
        column_name: &str,
        mode: i64,
        custom_rename_columns: &OrderedMap<String>,
    ) -> Result<String, Thrown> {
        match mode {
            Self::COLUMN_RENAMING_INCREMENT => {
                let alias = format!("{column_name}{}", self.sql_counter);
                self.sql_counter += 1;
                Ok(alias)
            }
            Self::COLUMN_RENAMING_CUSTOM => Ok(custom_rename_columns
                .get(column_name)
                .cloned()
                .unwrap_or_else(|| column_name.to_owned())),
            Self::COLUMN_RENAMING_NONE => Ok(column_name.to_owned()),
            _ => Err(invalid_argument(format!(
                "{mode} is not a valid value for $mode"
            ))),
        }
    }

    /// Retrieves a class columns and join columns aliases that are used in
    /// the SELECT clause.
    ///
    /// This depends on the renaming mode selected by the user.
    fn get_column_alias_map(
        &mut self,
        class_name: &str,
        mode: i64,
        custom_rename_columns: &OrderedMap<String>,
    ) -> Result<OrderedMap<String>, Thrown> {
        let mode = if custom_rename_columns.is_empty() {
            mode
        } else {
            // for BC with 2.2-2.3 API
            Self::COLUMN_RENAMING_CUSTOM
        };

        let mut column_alias = OrderedMap::new();
        let class = self.em.get_class_metadata(class_name)?;
        let (column_names, join_columns) = {
            let info = class.read();
            let join_columns: Vec<String> = info
                .association_mappings
                .values()
                .filter(|a| a.is_to_one_owning_side())
                .flat_map(|a| a.join_columns.iter().map(|j| j.name.clone()))
                .collect();
            (info.get_column_names(None), join_columns)
        };

        for column_name in column_names.into_iter().chain(join_columns) {
            let alias = self.get_column_alias(&column_name, mode, custom_rename_columns)?;
            column_alias.insert(column_name, alias);
        }
        Ok(column_alias)
    }

    /// Generates the Select clause from this `ResultSetMappingBuilder`.
    ///
    /// Works only for all the entity results. The select parts for scalar
    /// expressions have to be written manually.
    pub fn generate_select_clause(&self, table_aliases: &PhpArray) -> Result<String, Thrown> {
        let mut sql = String::new();
        let platform = self.em.get_platform();

        for (column_name, dql_alias) in self.rsm.column_owner_map.iter() {
            let table_alias = match table_aliases.get(&dql_alias.as_str().into()) {
                Some(alias) if !alias.is_null() => crate::utility::to_text(alias),
                _ => dql_alias.clone(),
            };

            if !sql.is_empty() {
                sql.push_str(", ");
            }

            if let Some(field_name) = self.rsm.field_mappings.get(column_name) {
                let declaring_class = self
                    .rsm
                    .declaring_classes
                    .get(column_name)
                    .cloned()
                    .unwrap_or_default();
                let class = self.em.get_class_metadata(&declaring_class)?;
                let info = class.read();
                let class_field_mapping = info.get_field_mapping(field_name)?;
                let column_sql = format!("{table_alias}.{}", class_field_mapping.column_name);

                let ty = types::get_type(&class_field_mapping.r#type)
                    .map_err(|e| Thrown::new(e.class(), e.message(), 0))?;
                let column_sql = ty
                    .convert_to_php_value_sql(column_sql.as_bytes(), platform)
                    .to_string_lossy()
                    .into_owned();

                sql.push_str(&column_sql);
            } else if let Some(meta) = self.rsm.meta_mappings.get(column_name) {
                sql.push_str(&table_alias);
                sql.push('.');
                sql.push_str(meta);
            } else if let Some(discriminator) = self.rsm.discriminator_columns.get(dql_alias) {
                sql.push_str(&table_alias);
                sql.push('.');
                sql.push_str(discriminator);
            }

            sql.push_str(" AS ");
            sql.push_str(column_name);
        }
        Ok(sql)
    }

    /// `__toString()`.
    pub fn to_string(&self) -> Result<String, Thrown> {
        self.generate_select_clause(&PhpArray::new())
    }
}
