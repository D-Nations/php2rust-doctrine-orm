//! The `DatabaseDriver` reverse engineers the mapping metadata from a
//! database.
//!
//! The schema manager's `listTables()` is async and the mapping driver's
//! methods are not, so [`DatabaseDriver::new`] lists the tables when the
//! driver is made; the driver reverse engineers them on first use, as
//! upstream does with the tables it lists then.
//!
//! Upstream: doctrine/orm src/Mapping/Driver/DatabaseDriver.php

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use php_value::{Key, PhpArray, Thrown, Value, php_array};
use php2rust_doctrine_dbal::schema::{
    AbstractSchemaManager, Column, ForeignKeyConstraint, Index, IndexType, Table,
};
use php2rust_doctrine_dbal::types::{self, Types};
use php2rust_doctrine_inflector::{Inflector, InflectorFactory};
use php2rust_doctrine_persistence::mapping::ClassMetadata as PersistenceClassMetadata;
use php2rust_doctrine_persistence::mapping::driver::MappingDriver;

use crate::exception::from_dbal;
use crate::mapping::class_metadata::{self as cm, ClassMetadataInfo};
use crate::mapping::{ClassMetadata, mapping_exception};

/// `Doctrine\ORM\Mapping\Driver\DatabaseDriver`.
pub const CLASS: &str = "Doctrine\\ORM\\Mapping\\Driver\\DatabaseDriver";

/// Replacement for `Types::ARRAY` (DBAL 3).
const ARRAY: &str = "array";
/// Replacement for `Types::OBJECT` (DBAL 3).
const OBJECT: &str = "object";

/// An array keyed by name, in insertion order.
type Named<T> = Vec<(String, T)>;

fn set<T>(map: &mut Named<T>, key: &str, value: T) {
    match map.iter_mut().find(|(k, _)| k == key) {
        Some(entry) => entry.1 = value,
        None => map.push((key.to_owned(), value)),
    }
}

fn get<'a, T>(map: &'a Named<T>, key: &str) -> Option<&'a T> {
    map.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

struct State {
    /// The tables the schema manager listed, reverse engineered on first
    /// use.
    listed: Vec<Table>,
    tables: Option<Named<Table>>,
    class_to_table_names: Named<String>,
    many_to_many_tables: Named<Table>,
    class_names_for_tables: HashMap<String, String>,
    field_names_for_columns: HashMap<String, HashMap<String, String>>,
    /// The namespace for the generated entities.
    namespace: Option<String>,
    inflector: Inflector,
}

/// `Doctrine\ORM\Mapping\Driver\DatabaseDriver`.
pub struct DatabaseDriver {
    state: Mutex<State>,
}

fn inflector() -> Result<Inflector, Thrown> {
    InflectorFactory::create()
        .and_then(|factory| factory.build())
        .map_err(|e| Thrown::new("Error", e.to_string(), 0))
}

impl DatabaseDriver {
    /// `new DatabaseDriver($sm)`, with the tables `$sm->listTables()` gives.
    pub async fn new<'c, S: AbstractSchemaManager<'c>>(sm: &S) -> Result<Self, Thrown> {
        let listed = sm.list_tables().await.map_err(|e| from_dbal(&e))?;
        Ok(Self {
            state: Mutex::new(State {
                listed,
                tables: None,
                class_to_table_names: Vec::new(),
                many_to_many_tables: Vec::new(),
                class_names_for_tables: HashMap::new(),
                field_names_for_columns: HashMap::new(),
                namespace: None,
                inflector: inflector()?,
            }),
        })
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Set the namespace for the generated entities.
    pub fn set_namespace(&self, namespace: &str) {
        self.state().namespace = Some(namespace.to_owned());
    }

    /// Sets class name for a table.
    pub fn set_class_name_for_table(&self, table_name: &str, class_name: &str) {
        self.state()
            .class_names_for_tables
            .insert(table_name.to_owned(), class_name.to_owned());
    }

    /// Sets field name for a column on a specific table.
    pub fn set_field_name_for_column(&self, table_name: &str, column_name: &str, field_name: &str) {
        self.state()
            .field_names_for_columns
            .entry(table_name.to_owned())
            .or_default()
            .insert(column_name.to_owned(), field_name.to_owned());
    }

    /// Sets tables manually instead of relying on the reverse engineering
    /// capabilities of `SchemaManager`.
    pub fn set_tables(
        &self,
        entity_tables: Vec<Table>,
        many_to_many_tables: Vec<Table>,
    ) -> Result<(), Thrown> {
        let mut state = self.state();
        let mut tables = Vec::new();
        state.many_to_many_tables = Vec::new();
        state.class_to_table_names = Vec::new();

        for table in entity_tables {
            let name = table_name(&table)?;
            let class_name = state.get_class_name_for_table(&name);
            set(&mut state.class_to_table_names, &class_name, name.clone());
            set(&mut tables, &name, table);
        }
        state.tables = Some(tables);

        for table in many_to_many_tables {
            let name = table_name(&table)?;
            set(&mut state.many_to_many_tables, &name, table);
        }
        Ok(())
    }

    pub fn set_inflector(&self, inflector: Inflector) {
        self.state().inflector = inflector;
    }
}

impl MappingDriver for DatabaseDriver {
    fn is_transient(&self, _class_name: &str) -> Result<bool, Thrown> {
        Ok(true)
    }

    fn get_all_class_names(&self) -> Result<Vec<String>, Thrown> {
        let mut state = self.state();
        state.reverse_engineer_mapping_from_database()?;
        Ok(state
            .class_to_table_names
            .iter()
            .map(|(class, _)| class.clone())
            .collect())
    }

    fn load_metadata_for_class(
        &self,
        class_name: &str,
        metadata: &dyn PersistenceClassMetadata,
    ) -> Result<(), Thrown> {
        let Some(metadata) = metadata.downcast_ref::<ClassMetadata>() else {
            return Err(Thrown::new(
                "TypeError",
                format!(
                    "Argument #2 passed to {CLASS}::loadMetadataForClass() must be an instance of {}, {} given.",
                    cm::CLASS,
                    metadata.get_name()
                ),
                0,
            ));
        };
        let mut state = self.state();
        state.reverse_engineer_mapping_from_database()?;
        let mut info = metadata.write();
        state.load(class_name, &mut info)
    }
}

impl State {
    fn load(&self, class_name: &str, metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
        let Some(table_name) = get(&self.class_to_table_names, class_name).cloned() else {
            return Err(Thrown::new(
                "InvalidArgumentException",
                format!("Unknown class {class_name}"),
                0,
            ));
        };

        class_name.clone_into(&mut metadata.name);
        metadata
            .table
            .insert(Key::from("name"), Value::string(table_name.as_str()));

        self.build_indexes(metadata, &table_name)?;
        self.build_field_mappings(metadata, &table_name)?;
        self.build_to_one_association_mappings(metadata, &table_name)?;

        for (many_table_name, many_table) in &self.many_to_many_tables {
            let foreign_keys: Vec<&ForeignKeyConstraint> =
                many_table.get_foreign_keys().values().collect();
            for &my_fk in &foreign_keys {
                // foreign key maps to the table of the current entity, many to many association probably exists
                if table_name.to_lowercase() != referenced_table_name(my_fk)?.to_lowercase() {
                    continue;
                }

                let Some(other_fk) = foreign_keys
                    .iter()
                    .copied()
                    .find(|&fk| !std::ptr::eq(fk, my_fk))
                else {
                    // the definition of this many to many table does not contain
                    // enough foreign key information to continue reverse engineering.
                    continue;
                };

                let my_columns = referencing_column_names(my_fk)?;
                let other_columns = referencing_column_names(other_fk)?;
                let local_column = my_columns.first().cloned().unwrap_or_default();

                let mut association_mapping = php_array! {
                    "fieldName" => self.get_field_name_for_column(
                        many_table_name,
                        other_columns.first().map_or("", String::as_str),
                        true,
                    ),
                    "targetEntity" => self.get_class_name_for_table(&referenced_table_name(other_fk)?),
                };

                let first_column = many_table
                    .get_columns()
                    .first()
                    .map(|column| column_name(column))
                    .transpose()?;
                if first_column.as_deref() == Some(local_column.as_str()) {
                    association_mapping.insert(
                        Key::from("inversedBy"),
                        Value::string(
                            self.get_field_name_for_column(many_table_name, &local_column, true)
                                .as_str(),
                        ),
                    );
                    association_mapping.insert(
                        Key::from("joinTable"),
                        Value::Array(php_array! {
                            "name" => many_table_name.to_lowercase(),
                            "joinColumns" => join_columns(my_fk)?,
                            "inverseJoinColumns" => join_columns(other_fk)?,
                        }),
                    );
                } else {
                    association_mapping.insert(
                        Key::from("mappedBy"),
                        Value::string(
                            self.get_field_name_for_column(many_table_name, &local_column, true)
                                .as_str(),
                        ),
                    );
                }

                metadata.map_many_to_many(association_mapping)?;

                break;
            }
        }
        Ok(())
    }

    fn reverse_engineer_mapping_from_database(&mut self) -> Result<(), Thrown> {
        if self.tables.is_some() {
            return Ok(());
        }

        let mut tables = Vec::new();
        self.many_to_many_tables = Vec::new();
        self.class_to_table_names = Vec::new();

        for table in self.listed.clone() {
            let name = table_name(&table)?;
            let foreign_keys = table.get_foreign_keys();

            let mut all_foreign_key_columns = Vec::new();
            for foreign_key in foreign_keys.values() {
                all_foreign_key_columns.extend(referencing_column_names(foreign_key)?);
            }

            let Some(primary_key) = table
                .get_primary_key_constraint()
                .map_err(|e| from_dbal(&e))?
            else {
                return Err(Thrown::new(
                    mapping_exception::CLASS,
                    format!(
                        "Table {name} has no primary key. Doctrine does not support reverse engineering from tables that don't have a primary key."
                    ),
                    0,
                ));
            };

            let mut pk_columns: Vec<String> = primary_key
                .get_column_names()
                .iter()
                .map(ToString::to_string)
                .collect();

            pk_columns.sort();
            all_foreign_key_columns.sort();

            if pk_columns == all_foreign_key_columns && foreign_keys.len() == 2 {
                set(&mut self.many_to_many_tables, &name, table);
            } else {
                // lower-casing is necessary because of Oracle Uppercase Tablenames,
                // assumption is lower-case + underscore separated.
                let class_name = self.get_class_name_for_table(&name);

                set(&mut tables, &name, table);
                set(&mut self.class_to_table_names, &class_name, name);
            }
        }
        self.tables = Some(tables);
        Ok(())
    }

    fn table(&self, table_name: &str) -> Result<&Table, Thrown> {
        self.tables
            .as_ref()
            .and_then(|tables| get(tables, table_name))
            .ok_or_else(|| Thrown::new("Error", format!("Undefined array key \"{table_name}\""), 0))
    }

    /// Build indexes from a class metadata.
    fn build_indexes(
        &self,
        metadata: &mut ClassMetadataInfo,
        table_name: &str,
    ) -> Result<(), Thrown> {
        let table = self.table(table_name)?;
        let primary_key = get_primary_key(table)?;

        for index in table.get_indexes().values() {
            if primary_key.is_some_and(|primary_key| std::ptr::eq(index, primary_key)) {
                continue;
            }

            let is_unique = index.get_type().map_err(|e| from_dbal(&e))? == IndexType::Unique;

            let index_name = index
                .get_object_name()
                .map_err(|e| from_dbal(&e))?
                .to_string();
            let index_columns = indexed_columns(index)?;
            let constraint_type = if is_unique {
                "uniqueConstraints"
            } else {
                "indexes"
            };

            let mut constraints = match metadata.table.get(&Key::from(constraint_type)) {
                Some(Value::Array(constraints)) => constraints.clone(),
                _ => PhpArray::new(),
            };
            let mut entry = match constraints.get(&Key::from(index_name.as_str())) {
                Some(Value::Array(entry)) => entry.clone(),
                _ => PhpArray::new(),
            };
            entry.insert(Key::from("columns"), strings(&index_columns));
            constraints.insert(Key::from(index_name.as_str()), Value::Array(entry));
            metadata
                .table
                .insert(Key::from(constraint_type), Value::Array(constraints));
        }
        Ok(())
    }

    /// Build field mapping from class metadata.
    fn build_field_mappings(
        &self,
        metadata: &mut ClassMetadataInfo,
        table_name: &str,
    ) -> Result<(), Thrown> {
        let table = self.table(table_name)?;
        let primary_keys = table_primary_keys(table)?;
        let mut all_foreign_keys = Vec::new();
        for foreign_key in table.get_foreign_keys().values() {
            all_foreign_keys.extend(referencing_column_names(foreign_key)?);
        }

        let mut has_ids = false;
        let mut field_mappings = Vec::new();

        for column in table.get_columns() {
            let name = column_name(column)?;
            if all_foreign_keys.contains(&name) {
                continue;
            }

            let mut field_mapping = self.build_field_mapping(table_name, column)?;

            if primary_keys.contains(&name) {
                field_mapping.insert(Key::from("id"), Value::Bool(true));
                has_ids = true;
            }

            field_mappings.push(field_mapping);
        }

        // We need to check for the columns here, because we might have associations as id as well.
        if has_ids && primary_keys.len() == 1 {
            metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
        }

        for field_mapping in field_mappings {
            metadata.map_field(field_mapping)?;
        }
        Ok(())
    }

    /// Build field mapping from a schema column definition.
    fn build_field_mapping(&self, table_name: &str, column: &Column) -> Result<PhpArray, Thrown> {
        let name = column_name(column)?;
        let column_type = column.get_type().map_err(|e| from_dbal(&e))?;
        let type_name =
            types::lookup_name(&column_type).map_err(|e| Thrown::new(e.class(), e.message(), 0))?;
        let mut options = php_array! { "comment" => column.get_comment() };
        let mut field_mapping = php_array! {
            "fieldName" => self.get_field_name_for_column(table_name, &name, false),
            "columnName" => name.as_str(),
            "type" => type_name.as_str(),
            "nullable" => !column.get_notnull(),
        };

        // Type specific elements
        match type_name.as_str() {
            ARRAY
            | Types::BLOB
            | Types::GUID
            | OBJECT
            | Types::SIMPLE_ARRAY
            | Types::STRING
            | Types::TEXT => {
                field_mapping.insert(Key::from("length"), optional(column.get_length()));
                options.insert(Key::from("fixed"), Value::Bool(column.get_fixed()));
            }
            Types::DECIMAL | Types::FLOAT => {
                field_mapping.insert(Key::from("precision"), optional(column.get_precision()));
                field_mapping.insert(Key::from("scale"), Value::Int(column.get_scale()));
            }
            Types::INTEGER | Types::BIGINT | Types::SMALLINT => {
                options.insert(Key::from("unsigned"), Value::Bool(column.get_unsigned()));
            }
            _ => {}
        }

        // Default
        let default = column.get_default();
        if !matches!(default, Value::Null) {
            options.insert(Key::from("default"), default.clone());
        }
        field_mapping.insert(Key::from("options"), Value::Array(options));

        Ok(field_mapping)
    }

    /// Build to one (one to one, many to one) association mapping from
    /// class metadata.
    fn build_to_one_association_mappings(
        &self,
        metadata: &mut ClassMetadataInfo,
        table_name: &str,
    ) -> Result<(), Thrown> {
        let table = self.table(table_name)?;
        let primary_keys = table_primary_keys(table)?;

        for foreign_key in table.get_foreign_keys().values() {
            let foreign_table_name = referenced_table_name(foreign_key)?;
            let fk_columns = referencing_column_names(foreign_key)?;
            let local_column = fk_columns.first().cloned().unwrap_or_default();
            let mut field_name = self.get_field_name_for_column(table_name, &local_column, true);

            if metadata.field_mappings.get(&field_name).is_some() {
                field_name.push('2'); // "foo" => "foo2"
            }

            let mut association_mapping = php_array! {
                "fieldName" => field_name,
                "targetEntity" => self.get_class_name_for_table(&foreign_table_name),
            };

            if primary_keys.contains(&local_column) {
                association_mapping.insert(Key::from("id"), Value::Bool(true));
            }

            association_mapping.insert(Key::from("joinColumns"), join_columns(foreign_key)?);

            // Here we need to check if $fkColumns are the same as $primaryKeys
            if fk_columns
                .iter()
                .all(|column| primary_keys.contains(column))
            {
                metadata.map_one_to_one(association_mapping)?;
            } else {
                metadata.map_many_to_one(association_mapping)?;
            }
        }
        Ok(())
    }

    /// Returns the mapped class name for a table if it exists. Otherwise
    /// return "classified" version.
    fn get_class_name_for_table(&self, table_name: &str) -> String {
        let namespace = self.namespace.as_deref().unwrap_or("");
        if let Some(class_name) = self.class_names_for_tables.get(table_name) {
            return format!("{namespace}{class_name}");
        }

        let classified = self
            .inflector
            .classify(table_name.to_lowercase().as_bytes())
            .to_string_lossy()
            .into_owned();
        format!("{namespace}{classified}")
    }

    /// Return the mapped field name for a column, if it exists. Otherwise
    /// return camelized version.
    fn get_field_name_for_column(&self, table_name: &str, column_name: &str, fk: bool) -> String {
        if let Some(field_name) = self
            .field_names_for_columns
            .get(table_name)
            .and_then(|columns| columns.get(column_name))
        {
            return field_name.clone();
        }

        let mut column_name = column_name.to_lowercase();

        // Replace _id if it is a foreignkey column
        if fk && let Some(stripped) = column_name.strip_suffix("_id") {
            column_name = stripped.to_owned();
        }

        self.inflector
            .camelize(column_name.as_bytes())
            .to_string_lossy()
            .into_owned()
    }
}

/// `['name' => $cols[$i], 'referencedColumnName' => $fkCols[$i]]` for each
/// column of the foreign key.
fn join_columns(foreign_key: &ForeignKeyConstraint) -> Result<Value, Thrown> {
    let columns = referencing_column_names(foreign_key)?;
    let referenced = referenced_column_names(foreign_key)?;
    let join_columns: Vec<Value> = columns
        .iter()
        .enumerate()
        .map(|(i, column)| {
            Value::Array(php_array! {
                "name" => column.as_str(),
                "referencedColumnName" => referenced.get(i).map_or(Value::Null, |c| Value::string(c.as_str())),
            })
        })
        .collect();
    Ok(Value::Array(PhpArray::list(join_columns)))
}

fn optional(value: Option<i64>) -> Value {
    value.map_or(Value::Null, Value::Int)
}

fn strings(values: &[String]) -> Value {
    Value::Array(PhpArray::list(
        values.iter().map(|value| Value::string(value.as_str())),
    ))
}

/// Retrieve schema table definition primary keys.
fn table_primary_keys(table: &Table) -> Result<Vec<String>, Thrown> {
    match table.get_primary_key_constraint() {
        Ok(Some(primary_key)) => Ok(primary_key
            .get_column_names()
            .iter()
            .map(ToString::to_string)
            .collect()),
        Ok(None) => Err(Thrown::new(
            "Error",
            "Call to a member function getColumnNames() on null",
            0,
        )),
        // Do nothing
        Err(_) => Ok(Vec::new()),
    }
}

fn table_name(table: &Table) -> Result<String, Thrown> {
    Ok(table
        .get_object_name()
        .map_err(|e| from_dbal(&e))?
        .to_string())
}

fn column_name(column: &Column) -> Result<String, Thrown> {
    Ok(column
        .get_object_name()
        .map_err(|e| from_dbal(&e))?
        .to_string())
}

fn referenced_table_name(foreign_key: &ForeignKeyConstraint) -> Result<String, Thrown> {
    Ok(foreign_key
        .get_referenced_table_name()
        .map_err(|e| from_dbal(&e))?
        .to_string())
}

fn referencing_column_names(foreign_key: &ForeignKeyConstraint) -> Result<Vec<String>, Thrown> {
    Ok(foreign_key
        .get_referencing_column_names()
        .map_err(|e| from_dbal(&e))?
        .iter()
        .map(ToString::to_string)
        .collect())
}

fn referenced_column_names(foreign_key: &ForeignKeyConstraint) -> Result<Vec<String>, Thrown> {
    Ok(foreign_key
        .get_referenced_column_names()
        .map_err(|e| from_dbal(&e))?
        .iter()
        .map(ToString::to_string)
        .collect())
}

fn indexed_columns(index: &Index) -> Result<Vec<String>, Thrown> {
    Ok(index
        .get_indexed_columns()
        .map_err(|e| from_dbal(&e))?
        .iter()
        .map(|column| column.get_column_name().to_string())
        .collect())
}

fn get_primary_key(table: &Table) -> Result<Option<&Index>, Thrown> {
    let primary_key_constraint = table
        .get_primary_key_constraint()
        .map_err(|e| from_dbal(&e))?;

    for index in table.get_indexes().values() {
        if let Some(constraint) = primary_key_constraint {
            let constraint_columns: Vec<String> = constraint
                .get_column_names()
                .iter()
                .map(ToString::to_string)
                .collect();

            if constraint_columns == indexed_columns(index)? {
                return Ok(Some(index));
            }
        } else if index.is_primary() {
            return Ok(Some(index));
        }
    }

    Ok(None)
}
