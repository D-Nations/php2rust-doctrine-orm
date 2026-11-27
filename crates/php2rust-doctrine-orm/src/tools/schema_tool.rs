//! The `SchemaTool` is a tool to create/drop/update database schemas based
//! on `ClassMetadata` class descriptors.
//!
//! The schema comes from DBAL's (deprecated) table API, as upstream's
//! default (`useDbalEditorApi` off) does. Sequences are not ported.
//! `getUpdateSchemaSql()` introspects the tables of the target schema that
//! the connection's schema assets filter leaves out one by one, instead of
//! swapping the filter (the connection's configuration cannot change). A
//! foreign key of a table that a
//! `postGenerateSchemaTable` listener replaced, or that the listener's new
//! schema does not have, is not added (upstream adds it to the detached
//! table). `dropSchema()` drops the tables of the classes
//! without introspecting the database first (the statements that fail
//! are ignored, as upstream ignores them).
//!
//! Upstream: doctrine/orm src/Tools/SchemaTool.php src/Tools/ToolsException.php src/Tools/Exception/MissingColumnException.php

use std::sync::Arc;

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::ParameterTypes;
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::schema::{
    AbstractSchemaManager as _, ComparatorConfig, Index, IndexType, SQLiteSchemaManager, Schema,
    Sequence, Table,
};
use php2rust_doctrine_event_manager::EventDispatcher;

use crate::entity_manager::EntityManager;
use crate::event;
use crate::exception;
use crate::mapping::{
    AssociationMapping, ClassMetadata, ClassMetadataInfo, FieldMapping, JoinColumnMapping,
    QuoteStrategy,
};
use crate::tools::event::{GenerateSchemaEventArgs, GenerateSchemaTableEventArgs};
use crate::tools::tool_events;
use crate::utility::{key_text, to_text};

const KNOWN_COLUMN_OPTIONS: [&str; 5] = ["comment", "unsigned", "fixed", "default", "values"];

/// A foreign key to add once all tables exist.
struct AddedFk {
    composite_name: String,
    table: String,
    foreign_table_name: String,
    foreign_columns: Vec<String>,
    local_columns: Vec<String>,
    fk_options: PhpArray,
    name: Option<String>,
}

/// `Doctrine\ORM\Tools\SchemaTool`.
pub struct SchemaTool {
    em: Arc<EntityManager>,
    quote_strategy: Arc<dyn QuoteStrategy>,
}

impl std::fmt::Debug for SchemaTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaTool").finish_non_exhaustive()
    }
}

fn dbal(e: &php2rust_doctrine_dbal::Exception) -> Thrown {
    exception::from_dbal(e)
}

fn missing_column(column: &str, source: &str, target: &str) -> Thrown {
    Thrown::new(
        "Doctrine\\ORM\\Tools\\Exception\\MissingColumnException",
        format!(
            "Column name \"{column}\" referenced for relation from {source} towards {target} does not exist."
        ),
        0,
    )
}

impl SchemaTool {
    /// Initializes a new `SchemaTool` instance that uses the connection of
    /// the provided `EntityManager`.
    pub fn new(em: &Arc<EntityManager>) -> Self {
        Self {
            em: em.clone(),
            quote_strategy: em.get_configuration().get_quote_strategy(),
        }
    }

    fn platform(&self) -> &dyn AbstractPlatform {
        self.em.get_platform()
    }

    /// Creates the database schema for the given array of `ClassMetadata`
    /// instances.
    pub async fn create_schema(&self, classes: &[Arc<ClassMetadata>]) -> Result<(), Thrown> {
        let create_schema_sql = self.get_create_schema_sql(classes)?;
        let conn = self.em.get_connection();
        for sql in create_schema_sql {
            if let Err(e) = conn
                .execute_statement(&sql, &PhpArray::new(), &ParameterTypes::new())
                .await
            {
                return Err(Thrown::new(
                    "Doctrine\\ORM\\Tools\\ToolsException",
                    format!(
                        "Schema-Tool failed with Error '{}' while executing DDL: {sql}",
                        e.message().to_string_lossy()
                    ),
                    0,
                ));
            }
        }
        Ok(())
    }

    /// Gets the list of DDL statements that are required to create the
    /// database schema for the given list of `ClassMetadata` instances.
    pub fn get_create_schema_sql(
        &self,
        classes: &[Arc<ClassMetadata>],
    ) -> Result<Vec<String>, Thrown> {
        let schema = self.get_schema_from_metadata(classes)?;
        schema.to_sql(self.platform()).map_err(|e| dbal(&e))
    }

    /// Drops the database schema for the given classes.
    ///
    /// In any way when an exception is thrown it is suppressed since drop
    /// was issued for all classes of the schema and some probably just
    /// don't exist.
    pub async fn drop_schema(&self, classes: &[Arc<ClassMetadata>]) -> Result<(), Thrown> {
        let drop_schema_sql = self.get_drop_schema_sql(classes)?;
        let conn = self.em.get_connection();
        for sql in drop_schema_sql {
            let _ = conn
                .execute_statement(&sql, &PhpArray::new(), &ParameterTypes::new())
                .await;
        }
        Ok(())
    }

    /// `dropDatabase()`: drops all elements in the database of the
    /// current connection.
    pub async fn drop_database(&self) -> Result<(), Thrown> {
        let drop_schema_sql = self.get_drop_database_sql().await?;
        let conn = self.em.get_connection();
        for sql in drop_schema_sql {
            conn.execute_statement(&sql, &PhpArray::new(), &ParameterTypes::new())
                .await
                .map_err(|e| dbal(&e))?;
        }
        Ok(())
    }

    /// `getDropDatabaseSQL()`: the SQL needed to drop the database schema
    /// for the connection's database.
    pub async fn get_drop_database_sql(&self) -> Result<Vec<String>, Thrown> {
        let conn = self.em.get_connection();
        let schema_manager = conn.create_schema_manager().await.map_err(|e| dbal(&e))?;
        schema_manager
            .introspect_schema()
            .await
            .map_err(|e| dbal(&e))?
            .to_drop_sql(self.platform())
            .map_err(|e| dbal(&e))
    }

    /// Gets SQL to drop the tables defined by the passed classes.
    pub fn get_drop_schema_sql(
        &self,
        classes: &[Arc<ClassMetadata>],
    ) -> Result<Vec<String>, Thrown> {
        let schema = self.get_schema_from_metadata(classes)?;
        schema.to_drop_sql(self.platform()).map_err(|e| dbal(&e))
    }

    /// Updates the database schema of the given classes by comparing the
    /// `ClassMetadata` instances to the current database schema that is
    /// inspected.
    pub async fn update_schema(&self, classes: &[Arc<ClassMetadata>]) -> Result<(), Thrown> {
        let conn = self.em.get_connection();
        for sql in self.get_update_schema_sql(classes).await? {
            conn.execute_statement(&sql, &PhpArray::new(), &ParameterTypes::new())
                .await
                .map_err(|e| dbal(&e))?;
        }
        Ok(())
    }

    /// Gets the sequence of SQL statements that need to be performed in
    /// order to bring the given class mappings in-synch with the relational
    /// schema.
    pub async fn get_update_schema_sql(
        &self,
        classes: &[Arc<ClassMetadata>],
    ) -> Result<Vec<String>, Thrown> {
        let to_schema = self.get_schema_from_metadata(classes)?;
        let conn = self.em.get_connection();
        let schema_manager = conn.create_schema_manager().await.map_err(|e| dbal(&e))?;
        let from_schema = Self::create_schema_for_comparison(&schema_manager, &to_schema).await?;

        let comparator = schema_manager.create_comparator_with(
            ComparatorConfig::default().with_report_modified_indexes(false),
        );
        let schema_diff = comparator
            .compare_schemas(&from_schema, &to_schema)
            .map_err(|e| dbal(&e))?;

        self.platform()
            .get_alter_schema_sql(&schema_diff)
            .map_err(|e| dbal(&e))
    }

    /// Creates the schema from the database, ensuring tables from the
    /// target schema are whitelisted for comparison.
    ///
    /// The connection's configuration cannot be changed here, so instead
    /// of swapping the schema assets filter, the tables of the target
    /// schema that the filter left out are introspected one by one (a
    /// missing one is skipped), as upstream's whitelisting filter keeps
    /// them.
    async fn create_schema_for_comparison(
        schema_manager: &SQLiteSchemaManager<'_>,
        to_schema: &Schema,
    ) -> Result<Schema, Thrown> {
        let introspected = schema_manager
            .introspect_schema()
            .await
            .map_err(|e| dbal(&e))?;

        let mut whitelisted = Vec::new();
        for table in to_schema.get_tables() {
            let name = table.get_name();
            if introspected.has_table(&name) {
                continue;
            }
            if let Ok(table) = schema_manager.introspect_table(&name).await {
                whitelisted.push(table);
            }
        }
        if whitelisted.is_empty() {
            return Ok(introspected);
        }

        let mut tables: Vec<Table> = introspected.get_tables().into_iter().cloned().collect();
        tables.extend(whitelisted);
        let sequences: Vec<Sequence> = introspected.get_sequences().into_iter().cloned().collect();
        let schema_config = schema_manager
            .create_schema_config()
            .await
            .map_err(|e| dbal(&e))?;
        Schema::new(tables, sequences, Some(schema_config), &[]).map_err(|e| dbal(&e))
    }

    fn processing_not_required(&self, class: &ClassMetadataInfo, processed: &[String]) -> bool {
        processed.contains(&class.name)
            || class.is_mapped_superclass
            || class.is_embedded_class
            || (class.is_inheritance_type_single_table() && class.name != class.root_entity_name)
            || self
                .em
                .get_configuration()
                .get_schema_ignore_classes()
                .contains(&class.name)
    }

    /// Resolves fields in index mapping to column names
    fn get_index_columns(
        &self,
        class: &ClassMetadataInfo,
        index_data: &PhpArray,
    ) -> Result<Vec<String>, Thrown> {
        let columns_value = index_data.get(&"columns".into()).filter(|v| !v.is_null());
        let fields_value = index_data.get(&"fields".into()).filter(|v| !v.is_null());
        if columns_value.is_some() == fields_value.is_some() {
            let name = index_data
                .get(&"name".into())
                .map_or_else(|| "unnamed".to_owned(), to_text);
            return Err(
                crate::mapping::mapping_exception::invalid_index_configuration(
                    &format!("{}@", class.name),
                    &name,
                ),
            );
        }
        let mut columns = Vec::new();
        if let Some(Value::Array(list)) = columns_value {
            columns.extend(list.values().map(to_text));
        }
        if let Some(Value::Array(list)) = fields_value {
            for field_name in list.values().map(to_text) {
                if class.has_field(&field_name) {
                    columns.push(self.quote_strategy.get_column_name(
                        &field_name,
                        class,
                        self.platform(),
                    ));
                } else if let Some(assoc) = class.association_mappings.get(&field_name) {
                    for join_column in &assoc.join_columns {
                        columns.push(self.quote_strategy.get_join_column_name(
                            join_column,
                            class,
                            self.platform(),
                        ));
                    }
                }
            }
        }
        Ok(columns)
    }

    /// Creates a Schema instance from a given set of metadata classes.
    #[allow(
        clippy::too_many_lines,
        reason = "getSchemaFromMetadata(), in its order"
    )]
    pub fn get_schema_from_metadata(
        &self,
        classes: &[Arc<ClassMetadata>],
    ) -> Result<Schema, Thrown> {
        let mut processed: Vec<String> = Vec::new();
        let mut schema = Schema::new(Vec::new(), Vec::new(), None, &[]).map_err(|e| dbal(&e))?;
        let mut added_fks: Vec<AddedFk> = Vec::new();
        let mut blacklisted_fks: Vec<String> = Vec::new();

        let event_manager = self.em.get_event_manager();
        let mut detached_tables: Vec<String> = Vec::new();

        for class_metadata in classes {
            let class = class_metadata.read().clone();
            if self.processing_not_required(&class, &processed) {
                continue;
            }
            let table_name = self.quote_strategy.get_table_name(&class, self.platform());
            schema.create_table(&table_name).map_err(|e| dbal(&e))?;

            if class.is_inheritance_type_single_table() {
                self.gather_columns(&class, &mut schema, &table_name)?;
                self.gather_relations_sql(
                    &class,
                    &table_name,
                    &mut schema,
                    &mut added_fks,
                    &mut blacklisted_fks,
                )?;
                self.add_discriminator_column_definition(&class, &mut schema, &table_name)?;
                // Add an ID FK column to child tables
                for parent_class_name in &class.parent_classes {
                    processed.push(parent_class_name.clone());
                }
                for sub_class_name in &class.sub_classes {
                    let sub_class = self.em.get_class_metadata(sub_class_name)?.read().clone();
                    self.gather_columns(&sub_class, &mut schema, &table_name)?;
                    self.gather_relations_sql(
                        &sub_class,
                        &table_name,
                        &mut schema,
                        &mut added_fks,
                        &mut blacklisted_fks,
                    )?;
                    processed.push(sub_class_name.clone());
                }
            } else if class.is_inheritance_type_joined() {
                self.gather_joined_class(
                    &class,
                    classes,
                    &table_name,
                    &mut schema,
                    &mut added_fks,
                    &mut blacklisted_fks,
                )?;
            } else {
                self.gather_columns(&class, &mut schema, &table_name)?;
                self.gather_relations_sql(
                    &class,
                    &table_name,
                    &mut schema,
                    &mut added_fks,
                    &mut blacklisted_fks,
                )?;
            }

            let mut pk_columns = Vec::new();
            for identifier_field in &class.identifier {
                if class.field_mappings.contains_key(identifier_field) {
                    pk_columns.push(self.quote_strategy.get_column_name(
                        identifier_field,
                        &class,
                        self.platform(),
                    ));
                } else if let Some(assoc) = class.association_mappings.get(identifier_field) {
                    for join_column in &assoc.join_columns {
                        pk_columns.push(self.quote_strategy.get_join_column_name(
                            join_column,
                            &class,
                            self.platform(),
                        ));
                    }
                }
            }

            let table = schema.get_table_mut(&table_name).map_err(|e| dbal(&e))?;
            if !table.has_index("primary") {
                let refs: Vec<&str> = pk_columns.iter().map(String::as_str).collect();
                table.set_primary_key(&refs, None).map_err(|e| dbal(&e))?;
            }

            // there can be unique indexes automatically created for join
            // column if join column is also primary key we should keep
            // only primary key on this column so, remove indexes overruled
            // by primary key
            if let Ok(primary_key) = table.get_index("primary") {
                let primary_key = primary_key.clone();
                let redundant: Vec<String> = table
                    .get_indexes()
                    .iter()
                    .filter(|(key, index)| {
                        *key != "primary" && primary_key.spans_columns(&index.get_columns())
                    })
                    .map(|(key, _)| key.to_owned())
                    .collect();
                for key in redundant {
                    table.drop_index(&key).map_err(|e| dbal(&e))?;
                }
            }

            if let Some(Value::Array(indexes)) = class.table.get(&"indexes".into()) {
                for (index_name, index_data) in indexes.iter() {
                    let Value::Array(index_data) = index_data else {
                        continue;
                    };
                    let flags: Vec<String> = match index_data.get(&"flags".into()) {
                        Some(Value::Array(flags)) => flags.values().map(to_text).collect(),
                        Some(Value::Null) | None => Vec::new(),
                        Some(other) => vec![to_text(other)],
                    };
                    let options = match index_data.get(&"options".into()) {
                        Some(Value::Array(options)) => options.clone(),
                        _ => PhpArray::new(),
                    };
                    let columns = self.get_index_columns(&class, index_data)?;
                    let name = match index_name {
                        php_value::Key::Int(_) => None,
                        key @ php_value::Key::Str(_) => Some(key_text(key)),
                    };
                    let table = schema.get_table_mut(&table_name).map_err(|e| dbal(&e))?;
                    table
                        .add_index(columns, name.as_deref(), flags, options)
                        .map_err(|e| dbal(&e))?;
                }
            }

            if let Some(Value::Array(unique_constraints)) =
                class.table.get(&"uniqueConstraints".into())
            {
                for (index_name, index_data) in unique_constraints.iter() {
                    let Value::Array(index_data) = index_data else {
                        continue;
                    };
                    let options = match index_data.get(&"options".into()) {
                        Some(Value::Array(options)) => options.clone(),
                        _ => PhpArray::new(),
                    };
                    let columns = self.get_index_columns(&class, index_data)?;
                    let uniq_index = Index::new(
                        Some(&format!("tmp__{}", key_text(index_name))),
                        columns.clone(),
                        true,
                        false,
                        Vec::<String>::new(),
                        options.clone(),
                    )
                    .map_err(|e| dbal(&e))?;
                    let table = schema.get_table_mut(&table_name).map_err(|e| dbal(&e))?;
                    let redundant = table
                        .get_indexes()
                        .iter()
                        .find(|(_, table_index)| {
                            is_index_redundant_for_unique_constraint(table_index, &uniq_index)
                        })
                        .map(|(key, _)| key.to_owned());
                    if let Some(key) = redundant {
                        table.drop_index(&key).map_err(|e| dbal(&e))?;
                    }
                    let name = match index_name {
                        php_value::Key::Int(_) => None,
                        key @ php_value::Key::Str(_) => Some(key_text(key)),
                    };
                    table
                        .add_unique_index(columns, name.as_deref(), options)
                        .map_err(|e| dbal(&e))?;
                }
            }

            if let Some(Value::Array(options)) = class.table.get(&"options".into()) {
                let table = schema.get_table_mut(&table_name).map_err(|e| dbal(&e))?;
                for (key, val) in options.iter() {
                    table.add_option(&key_text(key), val.clone());
                }
            }

            processed.push(class.name.clone());

            let original_table_name = schema
                .get_table(&table_name)
                .and_then(Table::get_object_name)
                .map_err(|e| dbal(&e))?;
            let table_event_args =
                GenerateSchemaTableEventArgs::new(class_metadata.clone(), schema, &table_name);
            let dispatched = event_manager
                .dispatch_event(
                    tool_events::POST_GENERATE_SCHEMA_TABLE,
                    Some(&table_event_args),
                )
                .map_err(event::listener_error);
            // Always retrieve the schema (listener may have mutated it)
            let (new_schema, new_table) = table_event_args.into_parts();
            schema = new_schema;
            dispatched?;

            // If the listener mutated the table, we need to replace it in the schema
            if let Some(new_table) = new_table {
                schema = schema
                    .edit()
                    .drop_table(&original_table_name)
                    .add_table(new_table)
                    .create()
                    .map_err(|e| dbal(&e))?;
                detached_tables.push(table_name);
            }
        }

        // After all classes have been processed and all FK metadata collected,
        // apply the non-conflicting FK constraints to their respective tables
        for fk in added_fks {
            // Skip if this composite key was blacklisted due to conflicts
            if blacklisted_fks.contains(&fk.composite_name) {
                continue;
            }
            // The table a listener replaced, or that a new schema does not
            // have, is no longer in the schema.
            if detached_tables.contains(&fk.table) {
                continue;
            }
            let Ok(table) = schema.get_table_mut(&fk.table) else {
                continue;
            };
            let local: Vec<&str> = fk.local_columns.iter().map(String::as_str).collect();
            let foreign: Vec<&str> = fk.foreign_columns.iter().map(String::as_str).collect();
            table
                .add_foreign_key_constraint(
                    &fk.foreign_table_name,
                    &local,
                    &foreign,
                    fk.fk_options,
                    fk.name.as_deref(),
                )
                .map_err(|e| dbal(&e))?;
        }

        let schema_event_args = GenerateSchemaEventArgs::new(self.em.clone(), schema);
        event_manager
            .dispatch_event(tool_events::POST_GENERATE_SCHEMA, Some(&schema_event_args))
            .map_err(event::listener_error)?;

        // Always retrieve the schema (listener may have mutated it)
        Ok(schema_event_args.into_schema())
    }

    /// The `isInheritanceTypeJoined()` branch of `getSchemaFromMetadata()`:
    /// the class's own columns and relations, the discriminator column on
    /// the root table, and on a child table the inherited identifier
    /// columns with a cascading foreign key to the root table.
    fn gather_joined_class(
        &self,
        class: &ClassMetadataInfo,
        classes: &[Arc<ClassMetadata>],
        table_name: &str,
        schema: &mut Schema,
        added_fks: &mut Vec<AddedFk>,
        blacklisted_fks: &mut Vec<String>,
    ) -> Result<(), Thrown> {
        // Add all non-inherited fields as columns
        {
            let table = schema.get_table_mut(table_name).map_err(|e| dbal(&e))?;
            for mapping in class.field_mappings.values() {
                if mapping.inherited.is_none() {
                    self.gather_column(class, mapping, table)?;
                }
            }
        }

        self.gather_relations_sql(class, table_name, schema, added_fks, blacklisted_fks)?;

        // Add the discriminator column only to the root table
        if class.name == class.root_entity_name {
            return self.add_discriminator_column_definition(class, schema, table_name);
        }

        // Add an ID FK column to child tables
        let mut pk_columns: Vec<String> = Vec::new();
        let mut inherited_key_columns: Vec<String> = Vec::new();

        for identifier_field in &class.identifier {
            if let Some(id_mapping) = class
                .field_mappings
                .get(identifier_field)
                .filter(|m| m.inherited.is_some())
            {
                let table = schema.get_table_mut(table_name).map_err(|e| dbal(&e))?;
                self.gather_column(class, id_mapping, table)?;
                let column_name =
                    self.quote_strategy
                        .get_column_name(identifier_field, class, self.platform());
                // TODO: This seems rather hackish, can we optimize it?
                table
                    .get_column_mut(&column_name)
                    .map_err(|e| dbal(&e))?
                    .set_autoincrement(false);

                pk_columns.push(column_name.clone());
                inherited_key_columns.push(column_name);
                continue;
            }

            if let Some(id_mapping) = class
                .association_mappings
                .get(identifier_field)
                .filter(|m| m.inherited.is_some())
            {
                let target_entity = classes
                    .iter()
                    .find(|c| c.read().name == id_mapping.target_entity)
                    .map(|c| c.read().clone());
                for join_column in &id_mapping.join_columns {
                    if target_entity.as_ref().is_some_and(|t| {
                        t.field_mappings
                            .contains_key(&join_column.referenced_column_name)
                    }) {
                        let column_name = self.quote_strategy.get_join_column_name(
                            join_column,
                            class,
                            self.platform(),
                        );
                        pk_columns.push(column_name.clone());
                        inherited_key_columns.push(column_name);
                    }
                }
            }
        }

        if !inherited_key_columns.is_empty() {
            // A FK collected on the same columns (e.g. for an inherited
            // identifier association) would get the same generated name
            // and replace the FK to the root table.
            let composite_name = format!("{table_name}.{}", inherited_key_columns.join(""));
            added_fks.retain(|fk| fk.composite_name != composite_name);

            // Add a FK constraint on the ID column
            let root_table = self.quote_strategy.get_table_name(
                &self.em.get_class_metadata(&class.root_entity_name)?.read(),
                self.platform(),
            );
            let mut fk_options = PhpArray::new();
            fk_options.insert("onDelete", Value::from("CASCADE"));
            let columns: Vec<&str> = inherited_key_columns.iter().map(String::as_str).collect();
            schema
                .get_table_mut(table_name)
                .map_err(|e| dbal(&e))?
                .add_foreign_key_constraint(&root_table, &columns, &columns, fk_options, None)
                .map_err(|e| dbal(&e))?;
        }

        if !pk_columns.is_empty() {
            let refs: Vec<&str> = pk_columns.iter().map(String::as_str).collect();
            schema
                .get_table_mut(table_name)
                .map_err(|e| dbal(&e))?
                .set_primary_key(&refs, None)
                .map_err(|e| dbal(&e))?;
        }
        Ok(())
    }

    /// Gets a portable column definition as required by the DBAL for the
    /// discriminator column of a class.
    fn add_discriminator_column_definition(
        &self,
        class: &ClassMetadataInfo,
        schema: &mut Schema,
        table_name: &str,
    ) -> Result<(), Thrown> {
        let mut discr_column = class.get_discriminator_column()?.clone();
        if discr_column.r#type.eq_ignore_ascii_case("string") && discr_column.length.is_none() {
            "string".clone_into(&mut discr_column.r#type);
            discr_column.length = Some(
                self.em
                    .get_configuration()
                    .get_default_string_type_schema_length(),
            );
        }
        let mut options = PhpArray::new();
        options.insert(
            "length",
            discr_column.length.map_or(Value::Null, Value::Int),
        );
        options.insert("notnull", Value::Bool(true));
        if let Some(definition) = &discr_column.column_definition {
            options.insert("columnDefinition", Value::from(definition.as_str()));
        }
        let mut gathered = gather_column_options(
            Some(&discr_column.options),
            discr_column.enum_type.as_deref(),
        );
        gathered.union(&options);
        schema
            .get_table_mut(table_name)
            .map_err(|e| dbal(&e))?
            .add_column(&discr_column.name, &discr_column.r#type, &gathered)
            .map_err(|e| dbal(&e))?;
        Ok(())
    }

    /// Gathers the column definitions as required by the DBAL of all field
    /// mappings found in the given class.
    fn gather_columns(
        &self,
        class: &ClassMetadataInfo,
        schema: &mut Schema,
        table_name: &str,
    ) -> Result<(), Thrown> {
        let table = schema.get_table_mut(table_name).map_err(|e| dbal(&e))?;
        for mapping in class.field_mappings.values() {
            if class.is_inheritance_type_single_table() && mapping.inherited.is_some() {
                continue;
            }
            self.gather_column(class, mapping, table)?;
        }
        Ok(())
    }

    /// Creates a column definition as required by the DBAL from an ORM
    /// field mapping definition.
    fn gather_column(
        &self,
        class: &ClassMetadataInfo,
        mapping: &FieldMapping,
        table: &mut Table,
    ) -> Result<(), Thrown> {
        let column_name =
            self.quote_strategy
                .get_column_name(&mapping.field_name, class, self.platform());
        let column_type = mapping.r#type.clone();

        let mut options = PhpArray::new();
        let mut length = mapping.length;
        let mut notnull = mapping.nullable.is_none_or(|n| !n);
        if class.is_inheritance_type_single_table() && !class.parent_classes.is_empty() {
            notnull = false;
        }
        let mut platform_options = PhpArray::new();
        platform_options.insert(
            "version",
            Value::Bool(
                class.is_versioned
                    && class.version_field.as_deref() == Some(mapping.field_name.as_str()),
            ),
        );
        if column_type.eq_ignore_ascii_case("string") && length.is_none() {
            length = Some(
                self.em
                    .get_configuration()
                    .get_default_string_type_schema_length(),
            );
        }
        options.insert("length", length.map_or(Value::Null, Value::Int));
        options.insert("notnull", Value::Bool(notnull));
        options.insert("platformOptions", Value::Array(platform_options));
        if let Some(precision) = mapping.precision {
            options.insert("precision", Value::Int(precision));
        }
        if let Some(scale) = mapping.scale {
            options.insert("scale", Value::Int(scale));
        }
        if !mapping.default.is_null() {
            options.insert("default", mapping.default.clone());
        }
        if let Some(definition) = &mapping.column_definition {
            options.insert("columnDefinition", Value::from(definition.as_str()));
        }
        let mut gathered =
            gather_column_options(mapping.options.as_ref(), mapping.enum_type.as_deref());
        gathered.union(&options);
        let mut options = gathered;

        if class.is_id_generator_identity()
            && class.get_identifier_field_names() == [mapping.field_name.clone()]
        {
            options.insert("autoincrement", Value::Bool(true));
        }

        if class.is_inheritance_type_joined() && class.name != class.root_entity_name {
            options.insert("autoincrement", Value::Bool(false));
        }

        if table.has_column(&column_name) {
            // required in some inheritance scenarios
            table
                .modify_column(&column_name, &options)
                .map_err(|e| dbal(&e))?;
        } else {
            table
                .add_column(&column_name, &column_type, &options)
                .map_err(|e| dbal(&e))?;
        }

        if mapping.unique == Some(true) {
            table
                .add_unique_index([column_name.clone()], None, PhpArray::new())
                .map_err(|e| dbal(&e))?;
        }
        if mapping.index == Some(true) {
            table
                .add_index([column_name], None, Vec::new(), PhpArray::new())
                .map_err(|e| dbal(&e))?;
        }
        Ok(())
    }

    /// Gathers the SQL for properly setting up the relations of the given
    /// class. This includes the SQL for foreign key constraints and join
    /// tables.
    fn gather_relations_sql(
        &self,
        class: &ClassMetadataInfo,
        table_name: &str,
        schema: &mut Schema,
        added_fks: &mut Vec<AddedFk>,
        blacklisted_fks: &mut Vec<String>,
    ) -> Result<(), Thrown> {
        for (id, mapping) in class.association_mappings.iter() {
            if mapping.inherited.is_some() && !class.identifier.iter().any(|i| i == id) {
                continue;
            }
            let foreign_class = self
                .em
                .get_class_metadata(&mapping.target_entity)?
                .read()
                .clone();

            if mapping.is_to_one_owning_side() {
                let mut primary_key_columns = Vec::new(); // PK is unnecessary for this relation-type
                self.gather_relation_join_columns(
                    &mapping.join_columns,
                    schema,
                    table_name,
                    &foreign_class,
                    mapping,
                    &mut primary_key_columns,
                    added_fks,
                    blacklisted_fks,
                    None,
                )?;
            } else if mapping.is_many_to_many_owning_side() {
                let Some(join_table) = &mapping.join_table else {
                    continue;
                };
                let join_table_name = self.quote_strategy.get_join_table_name(
                    mapping,
                    &foreign_class,
                    self.platform(),
                );
                {
                    let the_join_table = schema
                        .create_table(&join_table_name)
                        .map_err(|e| dbal(&e))?;
                    for (key, val) in join_table.options.iter() {
                        the_join_table.add_option(&key_text(key), val.clone());
                    }
                }
                let mut primary_key_columns = Vec::new();
                self.gather_relation_join_columns(
                    &join_table.join_columns,
                    schema,
                    &join_table_name,
                    class,
                    mapping,
                    &mut primary_key_columns,
                    added_fks,
                    blacklisted_fks,
                    join_table.foreign_key_name.as_deref(),
                )?;
                self.gather_relation_join_columns(
                    &join_table.inverse_join_columns,
                    schema,
                    &join_table_name,
                    &foreign_class,
                    mapping,
                    &mut primary_key_columns,
                    added_fks,
                    blacklisted_fks,
                    join_table.inverse_foreign_key_name.as_deref(),
                )?;
                let refs: Vec<&str> = primary_key_columns.iter().map(String::as_str).collect();
                schema
                    .get_table_mut(&join_table_name)
                    .map_err(|e| dbal(&e))?
                    .set_primary_key(&refs, None)
                    .map_err(|e| dbal(&e))?;
            }
        }
        Ok(())
    }

    /// Get the class metadata that is responsible for the definition of
    /// the referenced column name.
    ///
    /// Previously this was a simple task by just querying the referenced
    /// column name in the given class. With the advent of inheritance
    /// this is not possible anymore (the class may be a subclass whose
    /// columns are in the parent).
    fn get_defining_class(
        &self,
        class: &ClassMetadataInfo,
        referenced_column_name: &str,
    ) -> Result<Option<(ClassMetadataInfo, String)>, Thrown> {
        let referenced_field_name = class.get_field_name(referenced_column_name);
        if class.has_field(&referenced_field_name) {
            return Ok(Some((class.clone(), referenced_field_name)));
        }
        if class
            .get_identifier_column_names()
            .iter()
            .any(|c| c == referenced_column_name)
        {
            // it seems to be an entity as foreign key
            for field_name in class.get_identifier_field_names() {
                if class.has_association(field_name)
                    && class.get_single_association_join_column_name(field_name)?
                        == referenced_column_name
                {
                    let Some(assoc) = class.association_mappings.get(field_name) else {
                        continue;
                    };
                    let target = self
                        .em
                        .get_class_metadata(&assoc.target_entity)?
                        .read()
                        .clone();
                    return self.get_defining_class(
                        &target,
                        &class.get_single_association_referenced_join_column_name(field_name)?,
                    );
                }
            }
        }
        Ok(None)
    }

    /// Gathers columns and fk constraints that are required for one part
    /// of relationship.
    #[allow(
        clippy::too_many_arguments,
        reason = "gatherRelationJoinColumns(), as PHP declares it"
    )]
    #[allow(clippy::too_many_lines, reason = "upstream's method, in its order")]
    fn gather_relation_join_columns(
        &self,
        join_columns: &[JoinColumnMapping],
        schema: &mut Schema,
        the_join_table: &str,
        class: &ClassMetadataInfo,
        mapping: &AssociationMapping,
        primary_key_columns: &mut Vec<String>,
        added_fks: &mut Vec<AddedFk>,
        blacklisted_fks: &mut Vec<String>,
        foreign_key_name: Option<&str>,
    ) -> Result<(), Thrown> {
        let platform = self.platform();
        let mut local_columns = Vec::new();
        let mut foreign_columns = Vec::new();
        let mut fk_options = PhpArray::new();
        let foreign_table_name = self.quote_strategy.get_table_name(class, platform);
        let mut unique_constraints: Vec<Vec<String>> = Vec::new();

        for join_column in join_columns {
            let Some((defining_class, referenced_field_name)) =
                self.get_defining_class(class, &join_column.referenced_column_name)?
            else {
                return Err(missing_column(
                    &join_column.referenced_column_name,
                    &mapping.source_entity,
                    &mapping.target_entity,
                ));
            };

            let quoted_column_name =
                self.quote_strategy
                    .get_join_column_name(join_column, class, platform);
            let quoted_ref_column_name =
                self.quote_strategy
                    .get_referenced_join_column_name(join_column, class, platform);

            primary_key_columns.push(quoted_column_name.clone());
            local_columns.push(quoted_column_name.clone());
            foreign_columns.push(quoted_ref_column_name);

            let table = schema.get_table_mut(the_join_table).map_err(|e| dbal(&e))?;
            if !table.has_column(&quoted_column_name) {
                // Only add the column to the table if it does not exist
                // already. It might exist already if the foreign key is
                // mapped into a regular property as well.
                let field_mapping = defining_class
                    .get_field_mapping(&referenced_field_name)?
                    .clone();
                let mut column_options = PhpArray::new();
                column_options.insert("notnull", Value::Bool(false));
                if let Some(definition) = &join_column.column_definition {
                    column_options.insert("columnDefinition", Value::from(definition.as_str()));
                } else if let Some(definition) = &field_mapping.column_definition {
                    column_options.insert("columnDefinition", Value::from(definition.as_str()));
                }
                if let Some(nullable) = join_column.nullable {
                    column_options.insert("notnull", Value::Bool(!nullable));
                }
                column_options.union(&gather_column_options(
                    field_mapping.options.as_ref(),
                    field_mapping.enum_type.as_deref(),
                ));
                if let Some(length) = field_mapping.length {
                    column_options.insert("length", Value::Int(length));
                }
                if field_mapping.r#type == "decimal" {
                    column_options
                        .insert("scale", field_mapping.scale.map_or(Value::Null, Value::Int));
                    column_options.insert(
                        "precision",
                        field_mapping.precision.map_or(Value::Null, Value::Int),
                    );
                }
                let mut options = gather_column_options(join_column.options.as_ref(), None);
                options.union(&column_options);
                table
                    .add_column(&quoted_column_name, &field_mapping.r#type, &options)
                    .map_err(|e| dbal(&e))?;
            }

            if join_column.unique == Some(true) {
                unique_constraints.push(vec![quoted_column_name.clone()]);
            }
            if let Some(on_delete) = &join_column.on_delete {
                fk_options.insert("onDelete", Value::from(on_delete.as_str()));
            }
            if let Some(deferrable) = join_column.deferrable {
                fk_options.insert("deferrable", Value::Bool(deferrable));
            }
        }

        // Prefer unique constraints over implicit simple indexes created
        // for foreign keys. Also avoids index duplication.
        {
            let table = schema.get_table_mut(the_join_table).map_err(|e| dbal(&e))?;
            for columns in unique_constraints {
                table
                    .add_unique_index(columns, None, PhpArray::new())
                    .map_err(|e| dbal(&e))?;
            }
        }

        let foreign_key_names: Vec<&String> = join_columns
            .iter()
            .filter_map(|jc| jc.foreign_key_name.as_ref())
            .collect();
        let extracted_foreign_key_name = match foreign_key_names.len() {
            0 => None,
            1 => Some(foreign_key_names[0].clone()),
            n => {
                return Err(Thrown::new(
                    "RuntimeException",
                    format!(
                        "Only one JoinColumn in a composite foreign key may specify foreignKeyName. {}::${} has {n} join columns with foreignKeyName specified.",
                        mapping.source_entity, mapping.field_name
                    ),
                    0,
                ));
            }
        };
        let final_foreign_key_name = foreign_key_name
            .map(str::to_owned)
            .or(extracted_foreign_key_name);

        let composite_name = format!("{the_join_table}.{}", local_columns.join(""));
        if let Some(existing_fk) = added_fks
            .iter()
            .find(|fk| fk.composite_name == composite_name)
        {
            let is_foreign_table_identical = foreign_table_name == existing_fk.foreign_table_name;
            let are_foreign_columns_identical = foreign_columns
                .iter()
                .all(|c| existing_fk.foreign_columns.contains(c))
                && existing_fk
                    .foreign_columns
                    .iter()
                    .all(|c| foreign_columns.contains(c));
            if is_foreign_table_identical && are_foreign_columns_identical {
                return Ok(());
            }
            if !blacklisted_fks.contains(&composite_name) {
                schema
                    .get_table_mut(the_join_table)
                    .map_err(|e| dbal(&e))?
                    .add_index(local_columns.clone(), None, Vec::new(), PhpArray::new())
                    .map_err(|e| dbal(&e))?;
            }
            blacklisted_fks.push(composite_name);
        } else if !blacklisted_fks.contains(&composite_name) {
            added_fks.push(AddedFk {
                composite_name,
                table: the_join_table.to_owned(),
                foreign_table_name,
                foreign_columns,
                local_columns,
                fk_options,
                name: final_foreign_key_name,
            });
        }
        Ok(())
    }
}

/// `gatherColumnOptions()`: the known column options of a mapping, the
/// others as platform options.
fn gather_column_options(mapping_options: Option<&PhpArray>, enum_type: Option<&str>) -> PhpArray {
    let mut mapping_options = mapping_options.cloned().unwrap_or_default();
    if let Some(enum_type) = enum_type {
        mapping_options.insert("enumType", Value::from(enum_type));
    }
    if let Some(Value::Object(default)) = mapping_options.get(&"default".into()).cloned()
        && crate::utility::is_backed_enum(default.as_ref())
        && let Some(value) = default.property(b"value")
    {
        mapping_options.insert("default", value);
    }
    if mapping_options.is_empty() {
        return PhpArray::new();
    }
    let mut options = PhpArray::new();
    let mut platform_options = PhpArray::new();
    for (key, value) in mapping_options.iter() {
        if KNOWN_COLUMN_OPTIONS.contains(&key_text(key).as_str()) {
            options.insert(key.clone(), value.clone());
        } else {
            platform_options.insert(key.clone(), value.clone());
        }
    }
    options.insert("platformOptions", Value::Array(platform_options));
    options
}

/// `isIndexRedundantForUniqueConstraint()`: unique constraints can replace
/// ordinary indexes on the same columns, but not platform-specific indexes
/// such as fulltext or spatial whose flags they do not fulfill.
fn is_index_redundant_for_unique_constraint(existing_index: &Index, unique_index: &Index) -> bool {
    existing_index.is_fulfilled_by(unique_index) && index_flags_are_compatible(existing_index)
}

/// `indexFlagsAreCompatible()`: whether a unique constraint can replace
/// the index given its platform-specific type. Unflagged indexes remain
/// compatible with unique constraints so they can still be deduplicated.
/// Fulltext and spatial indexes keep their own semantics.
fn index_flags_are_compatible(existing_index: &Index) -> bool {
    !matches!(
        existing_index.get_type(),
        Ok(IndexType::Fulltext | IndexType::Spatial)
    )
}
