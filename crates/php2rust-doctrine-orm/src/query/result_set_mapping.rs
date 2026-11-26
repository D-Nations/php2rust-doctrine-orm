//! A `ResultSetMapping` describes how a result set of an SQL query maps to
//! a Doctrine result.
//!
//! The public array properties are ordered maps keyed by alias or column
//! name, in PHP's insertion order; the new-object, nested-entity and
//! metadata-parameter maps, which only the DQL walker fills, stay
//! `PhpArray`s.
//!
//! Upstream: doctrine/orm src/Query/ResultSetMapping.php

use php_value::{Key, PhpArray};

use crate::mapping::ordered_map::OrderedMap;

/// `Doctrine\ORM\Query\ResultSetMapping`.
#[derive(Debug, Clone)]
pub struct ResultSetMapping {
    /// Whether the result is mixed (contains scalar values together with
    /// field values).
    pub is_mixed: bool,
    /// Whether the result is a select statement.
    pub is_select: bool,
    /// Maps alias names to class names.
    pub alias_map: OrderedMap<String>,
    /// The aliases of partial entity results.
    pub partial_aliases: OrderedMap<bool>,
    /// Maps alias names to related association field names.
    pub relation_map: OrderedMap<String>,
    /// Maps alias names to parent alias names.
    pub parent_alias_map: OrderedMap<String>,
    /// Maps column names in the result set to field names for each class.
    pub field_mappings: OrderedMap<String>,
    /// Declaring class → alias → field name → column name.
    pub column_alias_mappings: OrderedMap<OrderedMap<OrderedMap<String>>>,
    /// Maps column names in the result set to the alias/field name to use
    /// in the mapped result.
    pub scalar_mappings: OrderedMap<Key>,
    /// Maps scalar columns to enums.
    pub enum_mappings: OrderedMap<String>,
    /// Maps column names in the result set to the alias/field type to use
    /// in the mapped result.
    pub type_mappings: OrderedMap<String>,
    /// Maps entities in the result set to the alias name to use in the
    /// mapped result.
    pub entity_mappings: OrderedMap<Option<String>>,
    /// Maps column names of meta columns (foreign keys, discriminator
    /// columns, ...) to field names.
    pub meta_mappings: OrderedMap<String>,
    /// Maps column names in the result set to the alias they belong to.
    pub column_owner_map: OrderedMap<String>,
    /// List of columns in the result set that are used as discriminator
    /// columns.
    pub discriminator_columns: OrderedMap<String>,
    /// Maps alias names to field names that should be used for indexing.
    pub index_by_map: OrderedMap<String>,
    /// Map from column names to class names that declare the field the
    /// column is mapped to.
    pub declaring_classes: OrderedMap<String>,
    /// This is necessary to hydrate derivate foreign keys correctly.
    pub is_identifier_column: OrderedMap<OrderedMap<bool>>,
    /// Maps column names in the result set to field names for each new
    /// object expression.
    pub new_object_mappings: PhpArray,
    /// Maps last argument for new objects in order to initiate object
    /// construction.
    pub new_object: PhpArray,
    /// Maps nested new object arguments.
    pub nested_new_object_arguments: PhpArray,
    /// Maps metadata parameter names to the metadata attribute.
    pub metadata_parameter_mapping: PhpArray,
    /// Contains query parameter names to be resolved as discriminator
    /// values.
    pub discriminator_parameters: PhpArray,
    /// Nested entities of new object expressions.
    pub nested_entities: PhpArray,
}

/// `new ResultSetMapping()`: `$isSelect` is `true`.
impl Default for ResultSetMapping {
    fn default() -> Self {
        Self {
            is_mixed: false,
            is_select: true,
            alias_map: Default::default(),
            partial_aliases: Default::default(),
            relation_map: Default::default(),
            parent_alias_map: Default::default(),
            field_mappings: Default::default(),
            column_alias_mappings: Default::default(),
            scalar_mappings: Default::default(),
            enum_mappings: Default::default(),
            type_mappings: Default::default(),
            entity_mappings: Default::default(),
            meta_mappings: Default::default(),
            column_owner_map: Default::default(),
            discriminator_columns: Default::default(),
            index_by_map: Default::default(),
            declaring_classes: Default::default(),
            is_identifier_column: Default::default(),
            new_object_mappings: Default::default(),
            new_object: Default::default(),
            nested_new_object_arguments: Default::default(),
            metadata_parameter_mapping: Default::default(),
            discriminator_parameters: Default::default(),
            nested_entities: Default::default(),
        }
    }
}

impl ResultSetMapping {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an entity result to this `ResultSetMapping`.
    pub fn add_entity_result(
        &mut self,
        class: &str,
        alias: &str,
        result_alias: Option<&str>,
    ) -> &mut Self {
        self.alias_map.insert(alias, class.to_owned());
        self.entity_mappings
            .insert(alias, result_alias.map(str::to_owned));
        if result_alias.is_some() {
            self.is_mixed = true;
        }
        self
    }

    pub fn mark_partial_entity_result(&mut self, alias: &str) -> &mut Self {
        self.partial_aliases.insert(alias, true);
        self
    }

    /// Sets a discriminator column for an entity result or joined entity
    /// result.
    pub fn set_discriminator_column(&mut self, alias: &str, discr_column: &str) -> &mut Self {
        self.discriminator_columns
            .insert(alias, discr_column.to_owned());
        self.column_owner_map.insert(discr_column, alias.to_owned());
        self
    }

    /// Sets a field to use for indexing an entity result or joined entity
    /// result.
    pub fn add_index_by(&mut self, alias: &str, field_name: &str) -> &mut Self {
        let mut found = None;
        // [...$this->metaMappings, ...$this->fieldMappings]: string keys of
        // the field mappings replace those of the meta mappings.
        let mut merged: OrderedMap<String> = self.meta_mappings.clone();
        for (column, field) in self.field_mappings.iter() {
            merged.insert(column, field.clone());
        }
        for (column_name, column_field_name) in merged.iter() {
            if column_field_name == field_name
                && self.column_owner_map.get(column_name).map(String::as_str) == Some(alias)
            {
                found = Some(column_name.to_owned());
                break;
            }
        }
        if let Some(column_name) = found {
            self.add_index_by_column(alias, &column_name);
        }
        self
    }

    pub fn add_index_by_scalar(&mut self, result_column_name: &str) -> &mut Self {
        self.index_by_map
            .insert("scalars", result_column_name.to_owned());
        self
    }

    pub fn add_index_by_column(&mut self, alias: &str, result_column_name: &str) -> &mut Self {
        self.index_by_map
            .insert(alias, result_column_name.to_owned());
        self
    }

    pub fn has_index_by(&self, alias: &str) -> bool {
        self.index_by_map.contains_key(alias)
    }

    pub fn is_field_result(&self, column_name: &str) -> bool {
        self.field_mappings.contains_key(column_name)
    }

    /// Adds a field to the result that belongs to an entity or joined
    /// entity.
    pub fn add_field_result(
        &mut self,
        alias: &str,
        column_name: &str,
        field_name: &str,
        declaring_class: Option<&str>,
    ) -> &mut Self {
        self.field_mappings
            .insert(column_name, field_name.to_owned());
        self.column_owner_map.insert(column_name, alias.to_owned());
        let declaring_class = match declaring_class {
            Some(class) if !class.is_empty() => class.to_owned(),
            _ => self.alias_map.get(alias).cloned().unwrap_or_default(),
        };
        self.declaring_classes
            .insert(column_name, declaring_class.clone());
        if !self.column_alias_mappings.contains_key(&declaring_class) {
            self.column_alias_mappings
                .insert(declaring_class.clone(), OrderedMap::new());
        }
        if let Some(by_alias) = self.column_alias_mappings.get_mut(&declaring_class) {
            if !by_alias.contains_key(alias) {
                by_alias.insert(alias, OrderedMap::new());
            }
            if let Some(fields) = by_alias.get_mut(alias) {
                fields.insert(field_name, column_name.to_owned());
            }
        }
        if !self.is_mixed && !self.scalar_mappings.is_empty() {
            self.is_mixed = true;
        }
        self
    }

    pub fn has_column_alias_by_field(&self, alias: &str, field_name: &str) -> bool {
        let Some(declaring_class) = self.alias_map.get(alias) else {
            return false;
        };
        self.column_alias_mappings
            .get(declaring_class)
            .and_then(|by_alias| by_alias.get(alias))
            .is_some_and(|fields| fields.contains_key(field_name))
    }

    pub fn get_column_alias_by_field(&self, alias: &str, field_name: &str) -> Option<String> {
        let declaring_class = self.alias_map.get(alias)?;
        self.column_alias_mappings
            .get(declaring_class)?
            .get(alias)?
            .get(field_name)
            .cloned()
    }

    /// Adds a joined entity result.
    pub fn add_joined_entity_result(
        &mut self,
        class: &str,
        alias: &str,
        parent_alias: &str,
        relation: &str,
        is_partial: bool,
    ) -> &mut Self {
        self.alias_map.insert(alias, class.to_owned());
        self.parent_alias_map.insert(alias, parent_alias.to_owned());
        self.relation_map.insert(alias, relation.to_owned());
        if is_partial {
            self.partial_aliases.insert(alias, true);
        }
        self
    }

    /// Adds a scalar result mapping.
    pub fn add_scalar_result(
        &mut self,
        column_name: &str,
        alias: impl Into<Key>,
        r#type: &str,
    ) -> &mut Self {
        self.scalar_mappings.insert(column_name, alias.into());
        self.type_mappings.insert(column_name, r#type.to_owned());
        if !self.is_mixed && !self.field_mappings.is_empty() {
            self.is_mixed = true;
        }
        self
    }

    pub fn add_enum_result(&mut self, column_name: &str, enum_type: &str) -> &mut Self {
        self.enum_mappings.insert(column_name, enum_type.to_owned());
        self
    }

    pub fn add_metadata_parameter_mapping(&mut self, parameter: impl Into<Key>, attribute: &str) {
        self.metadata_parameter_mapping
            .insert(parameter.into(), attribute.into());
    }

    pub fn is_scalar_result(&self, column_name: &str) -> bool {
        self.scalar_mappings.contains_key(column_name)
    }

    pub fn get_class_name(&self, alias: &str) -> Option<&str> {
        self.alias_map.get(alias).map(String::as_str)
    }

    pub fn get_scalar_alias(&self, column_name: &str) -> Option<&Key> {
        self.scalar_mappings.get(column_name)
    }

    pub fn get_declaring_class(&self, column_name: &str) -> Option<&str> {
        self.declaring_classes.get(column_name).map(String::as_str)
    }

    pub fn get_relation(&self, alias: &str) -> Option<&str> {
        self.relation_map.get(alias).map(String::as_str)
    }

    pub fn is_relation(&self, alias: &str) -> bool {
        self.relation_map.contains_key(alias)
    }

    pub fn get_entity_alias(&self, column_name: &str) -> Option<&str> {
        self.column_owner_map.get(column_name).map(String::as_str)
    }

    pub fn get_parent_alias(&self, alias: &str) -> Option<&str> {
        self.parent_alias_map.get(alias).map(String::as_str)
    }

    pub fn has_parent_alias(&self, alias: &str) -> bool {
        self.parent_alias_map.contains_key(alias)
    }

    pub fn get_field_name(&self, column_name: &str) -> Option<&str> {
        self.field_mappings.get(column_name).map(String::as_str)
    }

    pub fn get_alias_map(&self) -> &OrderedMap<String> {
        &self.alias_map
    }

    pub fn get_entity_result_count(&self) -> usize {
        self.alias_map.len()
    }

    pub fn is_mixed_result(&self) -> bool {
        self.is_mixed
    }

    /// Adds a meta column (foreign key or discriminator column) to the
    /// result set.
    pub fn add_meta_result(
        &mut self,
        alias: &str,
        column_name: &str,
        field_name: &str,
        is_identifier_column: bool,
        r#type: Option<&str>,
    ) -> &mut Self {
        self.meta_mappings
            .insert(column_name, field_name.to_owned());
        self.column_owner_map.insert(column_name, alias.to_owned());
        if is_identifier_column {
            if !self.is_identifier_column.contains_key(alias) {
                self.is_identifier_column.insert(alias, OrderedMap::new());
            }
            if let Some(columns) = self.is_identifier_column.get_mut(alias) {
                columns.insert(column_name, true);
            }
        }
        if let Some(t) = r#type.filter(|t| !t.is_empty()) {
            self.type_mappings.insert(column_name, t.to_owned());
        }
        self
    }
}
