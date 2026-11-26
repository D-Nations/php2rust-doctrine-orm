//! `Doctrine\ORM\Tools\Event`: the event arguments of the schema tool's
//! events.
//!
//! A listener reaches the schema and the class table through closures
//! (`with_schema()`, `with_class_table()`), as the arguments hold them
//! behind a lock. The class table is the table of the class's name in the
//! current schema until `setClassTable()` replaces it.
//!
//! Upstream: doctrine/orm src/Tools/Event/GenerateSchemaTableEventArgs.php src/Tools/Event/GenerateSchemaEventArgs.php

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use php2rust_doctrine_dbal::schema::{Schema, Table};
use php2rust_doctrine_event_manager::EventArgs;

use crate::entity_manager::EntityManager;
use crate::mapping::ClassMetadata;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Event Args used for the `Events::postGenerateSchemaTable` event.
pub struct GenerateSchemaTableEventArgs {
    class_metadata: Arc<ClassMetadata>,
    schema: Mutex<Schema>,
    class_table_name: String,
    class_table: Mutex<Option<Table>>,
}

impl std::fmt::Debug for GenerateSchemaTableEventArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenerateSchemaTableEventArgs")
            .field("class_table_name", &self.class_table_name)
            .finish_non_exhaustive()
    }
}

impl GenerateSchemaTableEventArgs {
    /// `new GenerateSchemaTableEventArgs($classMetadata, $schema, $classTable)`:
    /// the class table is the schema's table of that name.
    pub fn new(
        class_metadata: Arc<ClassMetadata>,
        schema: Schema,
        class_table_name: impl Into<String>,
    ) -> Self {
        Self {
            class_metadata,
            schema: Mutex::new(schema),
            class_table_name: class_table_name.into(),
            class_table: Mutex::new(None),
        }
    }

    /// `getClassMetadata()`.
    pub fn get_class_metadata(&self) -> &Arc<ClassMetadata> {
        &self.class_metadata
    }

    /// `getSchema()`: the schema, to read or change.
    pub fn with_schema<R>(&self, f: impl FnOnce(&mut Schema) -> R) -> R {
        f(&mut lock(&self.schema))
    }

    /// `getClassTable()`: the class table, to read or change; `None` when
    /// a new schema no longer has it.
    pub fn with_class_table<R>(&self, f: impl FnOnce(&mut Table) -> R) -> Option<R> {
        let mut class_table = lock(&self.class_table);
        if let Some(table) = class_table.as_mut() {
            return Some(f(table));
        }
        drop(class_table);
        let mut schema = lock(&self.schema);
        schema.get_table_mut(&self.class_table_name).ok().map(f)
    }

    /// `setSchema()`.
    pub fn set_schema(&self, schema: Schema) {
        *lock(&self.schema) = schema;
    }

    /// `setClassTable()`.
    pub fn set_class_table(&self, class_table: Table) {
        *lock(&self.class_table) = Some(class_table);
    }

    /// `classTableWasMutated()`.
    pub fn class_table_was_mutated(&self) -> bool {
        lock(&self.class_table).is_some()
    }

    /// The schema, and the class table when `setClassTable()` replaced it.
    pub fn into_parts(self) -> (Schema, Option<Table>) {
        (
            self.schema
                .into_inner()
                .unwrap_or_else(PoisonError::into_inner),
            self.class_table
                .into_inner()
                .unwrap_or_else(PoisonError::into_inner),
        )
    }
}

impl EventArgs for GenerateSchemaTableEventArgs {}

/// Event Args used for the `Events::postGenerateSchema` event.
pub struct GenerateSchemaEventArgs {
    em: Arc<EntityManager>,
    schema: Mutex<Schema>,
}

impl std::fmt::Debug for GenerateSchemaEventArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenerateSchemaEventArgs")
            .finish_non_exhaustive()
    }
}

impl GenerateSchemaEventArgs {
    pub fn new(em: Arc<EntityManager>, schema: Schema) -> Self {
        Self {
            em,
            schema: Mutex::new(schema),
        }
    }

    /// `getEntityManager()`.
    pub fn get_entity_manager(&self) -> &Arc<EntityManager> {
        &self.em
    }

    /// `getSchema()`: the schema, to read or change.
    pub fn with_schema<R>(&self, f: impl FnOnce(&mut Schema) -> R) -> R {
        f(&mut lock(&self.schema))
    }

    /// `setSchema()`.
    pub fn set_schema(&self, schema: Schema) {
        *lock(&self.schema) = schema;
    }

    /// The schema.
    pub fn into_schema(self) -> Schema {
        self.schema
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl EventArgs for GenerateSchemaEventArgs {}
