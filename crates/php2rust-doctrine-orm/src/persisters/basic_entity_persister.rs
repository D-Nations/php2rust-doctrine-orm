//! A `BasicEntityPersister` maps an entity to a single table in a
//! relational database.
//!
//! A persister is always responsible for a single entity type.
//!
//! `EntityPersisters` are used during a `UnitOfWork` to apply any changes to
//! the persistent state of entities onto a relational database when the
//! `UnitOfWork` is committed, as well as for basic querying of entities and
//! their associations (not DQL).
//!
//! The persister does not join the inverse sides of one-to-one
//! associations and `EAGER` to-one associations into its `SELECT`; the
//! unit of work loads them when it creates the entity (a second query,
//! which is what PHP does without the join). Lock modes and filters are
//! not ported.
//!
//! Upstream: doctrine/orm src/Persisters/Entity/BasicEntityPersister.php src/Persisters/Entity/CachedPersisterContext.php
//! Upstream: doctrine/orm src/Persisters/Entity/EntityPersister.php

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_collections::expr::{Comparison, ExpressionVisitor};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::{LockMode, ParameterTypes, types};

use crate::entity_manager::EntityManager;
use crate::exception;
use crate::id::IdGenerator;
use crate::internal::hydration::{self, CollectionTarget};
use crate::internal::object_id;
use crate::mapping::{AssociationMapping, ClassMetadata, JoinColumnMapping, QuoteStrategy};
use crate::persistent_collection::PersistentCollection;
use crate::query::ResultSetMapping;
use crate::unit_of_work::Hints;

use super::sql_expression_visitor::SqlExpressionVisitor;
use super::sql_value_visitor::SqlValueVisitor;
use crate::utility::{
    ParamType, convert_to_parameter_value, flatten_identifier, get_type_of_column,
    get_type_of_field, infer_parameter_types, key_text, to_text,
};

/// `CachedPersisterContext`: a set of cached SQL fragments and result set
/// mappings for one kind of query (with or without limits).
#[derive(Debug, Clone, Default)]
struct CachedPersisterContext {
    /// `ResultSetMapping` that is used for all queries. Is generated lazily
    /// once per request.
    rsm: ResultSetMapping,
    /// The SELECT column list SQL fragment used for querying entities by
    /// this persister. This SQL fragment is only generated once per
    /// request, if at all.
    select_column_list_sql: Option<String>,
    /// The JOIN SQL fragment used to eagerly load all many-to-one and
    /// one-to-one associations.
    select_join_sql: String,
    /// Counter for creating unique SQL table and column aliases.
    sql_alias_counter: i64,
    /// Map from class names (FQCN) to the corresponding generated SQL
    /// table aliases.
    sql_table_aliases: HashMap<String, String>,
}

#[derive(Debug, Default)]
struct PersisterState {
    /// Queued inserts.
    queued_inserts: Vec<(usize, Object)>,
    /// The map of column names to DBAL mapping types of all prepared
    /// columns used when `INSERTing` or `UPDATEing` an entity.
    column_types: HashMap<String, String>,
    /// The map of quoted column names.
    quoted_columns: HashMap<String, String>,
    no_limits_context: CachedPersisterContext,
    limits_handling_context: CachedPersisterContext,
    /// Whether the limits handling context is the current one.
    limits: bool,
}

impl PersisterState {
    fn ctx(&mut self) -> &mut CachedPersisterContext {
        if self.limits {
            &mut self.limits_handling_context
        } else {
            &mut self.no_limits_context
        }
    }
}

/// `Doctrine\ORM\Persisters\Entity\BasicEntityPersister`.
pub struct BasicEntityPersister {
    em: Weak<EntityManager>,
    /// Metadata object that describes the mapping of the mapped entity
    /// class.
    class: Arc<ClassMetadata>,
    /// The quote strategy.
    quote_strategy: Arc<dyn QuoteStrategy>,
    state: Mutex<PersisterState>,
}

impl std::fmt::Debug for BasicEntityPersister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicEntityPersister")
            .finish_non_exhaustive()
    }
}

/// The parameters and their types.
type Params = (Vec<Value>, Vec<ParamType>);

fn types_of(types: Vec<ParamType>) -> ParameterTypes {
    types
        .into_iter()
        .enumerate()
        .map(|(i, t)| (Key::Int(i64::try_from(i).unwrap_or(i64::MAX)), t))
        .collect()
}

fn dbal(e: &php2rust_doctrine_dbal::Exception) -> Thrown {
    exception::from_dbal(e)
}

/// The criteria of `getSelectSQL()` and `getCountSQL()`:
/// `array|Criteria`.
#[derive(Debug, Clone, Copy)]
pub enum PersisterCriteria<'a> {
    Array(&'a PhpArray),
    Criteria(&'a Criteria),
}

impl<'a> From<&'a PhpArray> for PersisterCriteria<'a> {
    fn from(criteria: &'a PhpArray) -> Self {
        Self::Array(criteria)
    }
}

impl<'a> From<&'a Criteria> for PersisterCriteria<'a> {
    fn from(criteria: &'a Criteria) -> Self {
        Self::Criteria(criteria)
    }
}

/// `self::$comparisonMap[$comparison]`.
fn comparison_sql(comparison: &str, placeholder: &str) -> Result<String, Thrown> {
    Ok(match comparison {
        Comparison::EQ => format!("= {placeholder}"),
        Comparison::NEQ => format!("!= {placeholder}"),
        Comparison::GT => format!("> {placeholder}"),
        Comparison::GTE => format!(">= {placeholder}"),
        Comparison::LT => format!("< {placeholder}"),
        Comparison::LTE => format!("<= {placeholder}"),
        Comparison::IN => format!("IN ({placeholder})"),
        Comparison::NIN => format!("NOT IN ({placeholder})"),
        Comparison::CONTAINS | Comparison::STARTS_WITH | Comparison::ENDS_WITH => {
            format!("LIKE {placeholder}")
        }
        _ => {
            // An undefined key of the map: `sprintf(null, ...)` under
            // strict types.
            return Err(Thrown::new(
                "TypeError",
                "sprintf(): Argument #1 ($format) must be of type string, null given",
                0,
            ));
        }
    })
}

/// The orderings of a criteria as `getOrderingsAsStringMap()` gives them.
fn orderings_as_string_map(criteria: &Criteria) -> PhpArray {
    let mut map = PhpArray::new();
    for (field, direction) in criteria.get_orderings() {
        let direction = if direction == php2rust_doctrine_collections::SortDirection::Ascending {
            "ASC"
        } else {
            "DESC"
        };
        map.insert(field.as_str(), Value::string(direction));
    }
    map
}

/// One criterion of a to-many statement (`expandToManyParameters()`).
struct ToManyCriterion {
    value: Value,
    field: String,
    class: Arc<ClassMetadata>,
}

impl BasicEntityPersister {
    /// Initializes a new `BasicEntityPersister` that uses the given
    /// `EntityManager` and persists instances of the class described by the
    /// given `ClassMetadata` descriptor.
    pub fn new(em: &Arc<EntityManager>, class: Arc<ClassMetadata>) -> Self {
        Self {
            em: Arc::downgrade(em),
            class,
            quote_strategy: em.get_configuration().get_quote_strategy(),
            state: Mutex::new(PersisterState::default()),
        }
    }

    fn st(&self) -> MutexGuard<'_, PersisterState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn em(&self) -> Result<Arc<EntityManager>, Thrown> {
        self.em
            .upgrade()
            .ok_or_else(|| Thrown::new("Error", "The entity manager is gone", 0))
    }

    pub fn get_class_metadata(&self) -> &Arc<ClassMetadata> {
        &self.class
    }

    /// The result set mapping of the current persister context.
    pub fn get_result_set_mapping(&self) -> ResultSetMapping {
        self.st().ctx().rsm.clone()
    }

    /// Adds an entity to the queued insertions. The entity remains
    /// queued until `executeInserts()` is invoked.
    pub fn add_insert(&self, entity: &Object) {
        let oid = object_id(entity);
        let mut st = self.st();
        if !st.queued_inserts.iter().any(|(o, _)| *o == oid) {
            st.queued_inserts.push((oid, entity.clone()));
        }
    }

    /// Gets the queued inserts.
    pub fn get_inserts(&self) -> Vec<Object> {
        self.st()
            .queued_inserts
            .iter()
            .map(|(_, e)| e.clone())
            .collect()
    }

    /// Executes all queued entity insertions.
    ///
    /// If no inserts are queued, invoking this method is a NOOP.
    pub async fn execute_inserts(&self) -> Result<(), Thrown> {
        if self.st().queued_inserts.is_empty() {
            return Ok(());
        }
        let em = self.em()?;
        let uow = em.get_unit_of_work();
        let (is_post_insert_id, table_name, requires_fetch) = {
            let info = self.class.read();
            (
                info.id_generator
                    .as_ref()
                    .is_some_and(IdGenerator::is_post_insert_generator),
                info.get_table_name()?,
                info.requires_fetch_after_change,
            )
        };
        let sql = self.get_insert_sql(&em)?;
        let conn = em.get_connection();

        let queued: Vec<(usize, Object)> = self.st().queued_inserts.clone();
        for (key, entity) in queued {
            let insert_data = self.prepare_insert_data(&em, &entity)?;
            let mut params = Vec::new();
            let mut types: Vec<ParamType> = Vec::new();
            if let Some(Value::Array(data)) = insert_data.get(&table_name.as_str().into()) {
                let column_types = self.st().column_types.clone();
                for (column, value) in data.iter() {
                    let column = key_text(column);
                    params.push(value.clone());
                    types.push(column_types.get(&column).map_or_else(
                        || php2rust_doctrine_dbal::ParameterType::String.into(),
                        |t| t.as_str().into(),
                    ));
                }
            }
            conn.execute_statement(&sql, &PhpArray::list(params), &types_of(types))
                .await
                .map_err(|e| dbal(&e))?;

            let id = if is_post_insert_id {
                // IdentityGenerator::generateId()
                let generated_id = conn.last_insert_id().await.map_err(|e| dbal(&e))?;
                let id_field = self
                    .class
                    .read()
                    .identifier
                    .first()
                    .cloned()
                    .unwrap_or_default();
                let mut id = PhpArray::new();
                id.insert(id_field.as_str(), generated_id.clone());
                uow.assign_post_insert_id(&entity, &generated_id)?;
                id
            } else {
                self.class.read().get_identifier_values(&entity)?
            };

            if requires_fetch {
                self.assign_default_version_and_upsertable_values(&em, &entity, &id)
                    .await?;
            }
            self.st().queued_inserts.retain(|(k, _)| *k != key);
        }
        Ok(())
    }

    /// Retrieves the default version value which was created by the
    /// database and assigns it back in to the entities version field if
    /// the entity is versioned. Also retrieves values of columns marked
    /// as 'non insertable' and / or 'not updatable' and assigns them back
    /// to the entities corresponding fields.
    async fn assign_default_version_and_upsertable_values(
        &self,
        em: &Arc<EntityManager>,
        entity: &Object,
        id: &PhpArray,
    ) -> Result<(), Thrown> {
        let values = self.fetch_version_and_not_upsertable_values(em, id).await?;
        let platform = em.get_platform();
        for (field, value) in values {
            let ty = self
                .class
                .read()
                .field_mappings
                .get(&field)
                .map(|m| m.r#type.clone())
                .unwrap_or_default();
            let ty = types::get_type(&ty).map_err(|e| Thrown::new(e.class(), e.message(), 0))?;
            let value = ty
                .convert_to_php_value(&value, platform)
                .map_err(|e| exception::from_conversion(&e))?;
            self.class.read().set_field_value(entity, &field, value)?;
        }
        Ok(())
    }

    /// Fetches the current version value of a versioned entity and / or
    /// the values of fields marked as 'not insertable' and / or 'not
    /// updatable'.
    async fn fetch_version_and_not_upsertable_values(
        &self,
        em: &Arc<EntityManager>,
        id: &PhpArray,
    ) -> Result<Vec<(String, Value)>, Thrown> {
        let platform = em.get_platform();
        let (sql, column_fields, flat_id) = {
            let info = self.class.read();
            let mut column_names: Vec<(String, String)> = Vec::new();
            for (key, column) in info.field_mappings.iter() {
                if column.generated.is_some()
                    || (info.is_versioned && info.version_field.as_deref() == Some(key))
                {
                    column_names.push((
                        key.to_owned(),
                        self.quote_strategy.get_column_name(key, &info, platform),
                    ));
                }
            }
            let table_name = self.quote_strategy.get_table_name(&info, platform);
            let identifier = self
                .quote_strategy
                .get_identifier_column_names(&info, platform);
            let sql = format!(
                "SELECT {} FROM {table_name} WHERE {} = ?",
                column_names
                    .iter()
                    .map(|(_, c)| c.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                identifier.join(" = ? AND ")
            );
            let flat_id = flatten_identifier(em, &info, id)?;
            (sql, column_names, flat_id)
        };
        let types = self.extract_identifier_types(em, id)?;
        let values = em
            .get_connection()
            .fetch_numeric(
                &sql,
                &PhpArray::list(flat_id.values().cloned()),
                &types_of(types),
            )
            .await
            .map_err(|e| dbal(&e))?;
        let Some(values) = values else {
            return Err(Thrown::new(
                "LengthException",
                "Unexpected empty result for database query.",
                0,
            ));
        };
        if values.len() != column_fields.len() || values.is_empty() {
            return Err(Thrown::new(
                "LengthException",
                "Unexpected number of database columns.",
                0,
            ));
        }
        Ok(column_fields
            .into_iter()
            .map(|(field, _)| field)
            .zip(values.values().cloned())
            .collect())
    }

    fn extract_identifier_types(
        &self,
        em: &EntityManager,
        id: &PhpArray,
    ) -> Result<Vec<ParamType>, Thrown> {
        let info = self.class.read();
        let mut types = Vec::new();
        for (field, value) in id.iter() {
            types.extend(infer_parameter_types(&key_text(field), value, &info, em)?);
        }
        Ok(types)
    }

    /// Updates a managed entity. The entity is updated according to its
    /// current changeset in the running `UnitOfWork`. If there is no
    /// changeset, nothing is updated.
    pub async fn update(&self, entity: &Object) -> Result<(), Thrown> {
        let em = self.em()?;
        let table_name = self.class.read().get_table_name()?;
        let update_data = self.prepare_update_data(&em, entity, false)?;
        let Some(Value::Array(data)) = update_data.get(&table_name.as_str().into()).cloned() else {
            return Ok(());
        };
        if data.is_empty() {
            return Ok(());
        }
        let (is_versioned, quoted_table_name, requires_fetch) = {
            let info = self.class.read();
            (
                info.is_versioned,
                self.quote_strategy.get_table_name(&info, em.get_platform()),
                info.requires_fetch_after_change,
            )
        };
        self.update_table(&em, entity, &quoted_table_name, &data, is_versioned)
            .await?;
        if requires_fetch {
            let id = self.class.read().get_identifier_values(entity)?;
            self.assign_default_version_and_upsertable_values(&em, entity, &id)
                .await?;
        }
        Ok(())
    }

    /// Performs an UPDATE statement for an entity on a specific table.
    /// The UPDATE can optionally be versioned, which requires the entity
    /// to have a version field.
    #[allow(clippy::too_many_lines, reason = "upstream's method, in its order")]
    async fn update_table(
        &self,
        em: &Arc<EntityManager>,
        entity: &Object,
        quoted_table_name: &str,
        update_data: &PhpArray,
        versioned: bool,
    ) -> Result<(), Thrown> {
        let platform = em.get_platform();
        let mut set = Vec::new();
        let mut types: Vec<ParamType> = Vec::new();
        let mut params = Vec::new();
        let (column_types, quoted_columns) = {
            let st = self.st();
            (st.column_types.clone(), st.quoted_columns.clone())
        };
        {
            let info = self.class.read();
            for (column_name, value) in update_data.iter() {
                let column_name = key_text(column_name);
                let mut placeholder = "?".to_owned();
                let mut column = column_name.clone();
                if let Some(field_name) = info.field_names.get(&column_name) {
                    column = self
                        .quote_strategy
                        .get_column_name(field_name, &info, platform);
                    if info.field_mappings.contains_key(field_name) {
                        let ty = column_types.get(&column_name).cloned().unwrap_or_default();
                        let ty = types::get_type(&ty)
                            .map_err(|e| Thrown::new(e.class(), e.message(), 0))?;
                        placeholder = ty
                            .convert_to_database_value_sql(b"?", platform)
                            .to_string_lossy()
                            .into_owned();
                    }
                } else if let Some(quoted) = quoted_columns.get(&column_name) {
                    column.clone_from(quoted);
                }
                params.push(value.clone());
                set.push(format!("{column} = {placeholder}"));
                types.push(column_types.get(&column_name).map_or_else(
                    || php2rust_doctrine_dbal::ParameterType::String.into(),
                    |t| t.as_str().into(),
                ));
            }
        }

        let mut where_ = Vec::new();
        let identifier = em.get_unit_of_work().get_entity_identifier(entity)?;
        let id_fields = self.class.read().identifier.clone();
        for id_field in id_fields {
            let id_value = identifier
                .get(&id_field.as_str().into())
                .cloned()
                .unwrap_or(Value::Null);
            let assoc = self
                .class
                .read()
                .association_mappings
                .get(&id_field)
                .cloned();
            let Some(assoc) = assoc else {
                let info = self.class.read();
                params.push(id_value);
                types.push(
                    info.field_mappings
                        .get(&id_field)
                        .map(|m| m.r#type.as_str())
                        .unwrap_or_default()
                        .into(),
                );
                where_.push(
                    self.quote_strategy
                        .get_column_name(&id_field, &info, platform),
                );
                continue;
            };
            params.push(id_value);
            if let Some(join_column) = assoc.join_columns.first() {
                let info = self.class.read();
                where_.push(
                    self.quote_strategy
                        .get_join_column_name(join_column, &info, platform),
                );
            }
            let target_mapping = em.get_class_metadata(&assoc.target_entity)?;
            let target = target_mapping.read();
            let first = target.identifier.first().cloned().unwrap_or_default();
            let target_type = get_type_of_field(&first, &target, em)?;
            let Some(first_type) = target_type.first() else {
                return Err(exception::unrecognized_field(
                    &self.class.read().name,
                    &first,
                ));
            };
            types.push(first_type.as_str().into());
        }

        if versioned {
            let info = self.class.read();
            let version_field = info.version_field.clone().unwrap_or_default();
            let version_field_type = info
                .field_mappings
                .get(&version_field)
                .map(|m| m.r#type.clone())
                .unwrap_or_default();
            let version_column =
                self.quote_strategy
                    .get_column_name(&version_field, &info, platform);
            where_.push(version_column.clone());
            types.push(version_field_type.as_str().into());
            params.push(info.get_field_value(entity, &version_field)?);
            match version_field_type.as_str() {
                "smallint" | "integer" | "bigint" => {
                    set.push(format!("{version_column} = {version_column} + 1"));
                }
                "datetime" => set.push(format!("{version_column} = CURRENT_TIMESTAMP")),
                _ => {}
            }
        }

        let sql = format!(
            "UPDATE {quoted_table_name} SET {} WHERE {} = ?",
            set.join(", "),
            where_.join(" = ? AND ")
        );
        let result = em
            .get_connection()
            .execute_statement(&sql, &PhpArray::list(params), &types_of(types))
            .await
            .map_err(|e| dbal(&e))?;
        if versioned && result == 0 {
            return Err(exception::optimistic_lock_failed(entity));
        }
        Ok(())
    }

    /// Deletes the join table records of the many-to-many associations of
    /// the entity.
    async fn delete_join_table_records(
        &self,
        em: &Arc<EntityManager>,
        identifier: &PhpArray,
        types: &[ParamType],
    ) -> Result<(), Thrown> {
        let platform = em.get_platform();
        let mappings: Vec<AssociationMapping> = self
            .class
            .read()
            .association_mappings
            .values()
            .cloned()
            .collect();
        for mapping in mappings {
            if !mapping.is_many_to_many() || mapping.is_on_delete_cascade == Some(true) {
                continue;
            }
            let self_referential = mapping.target_entity == mapping.source_entity;
            let class = if mapping.is_owning_side() {
                self.class.clone()
            } else {
                em.get_class_metadata(&mapping.target_entity)?
            };
            let association = em.get_metadata_factory().get_owning_side(&mapping)?;
            let Some(join_table) = association.join_table.clone() else {
                continue;
            };
            let join_columns = if mapping.is_owning_side() {
                join_table.join_columns.clone()
            } else {
                join_table.inverse_join_columns.clone()
            };
            let other_columns = if self_referential {
                if mapping.is_owning_side() {
                    join_table.inverse_join_columns.clone()
                } else {
                    join_table.join_columns.clone()
                }
            } else {
                Vec::new()
            };
            let (keys, other_keys, join_table_name) = {
                let info = class.read();
                let keys: Vec<String> = join_columns
                    .iter()
                    .map(|jc| {
                        self.quote_strategy
                            .get_join_column_name(jc, &info, platform)
                    })
                    .collect();
                let other_keys: Vec<String> = other_columns
                    .iter()
                    .map(|jc| {
                        self.quote_strategy
                            .get_join_column_name(jc, &info, platform)
                    })
                    .collect();
                let join_table_name = self.quote_strategy.get_join_table_name(
                    &association,
                    &self.class.read(),
                    platform,
                );
                (keys, other_keys, join_table_name)
            };
            let conn = em.get_connection();
            let combine = |keys: &[String]| {
                let mut criteria = PhpArray::new();
                for (key, value) in keys.iter().zip(identifier.values()) {
                    criteria.insert(key.as_str(), value.clone());
                }
                criteria
            };
            conn.delete(&join_table_name, &combine(&keys), &types_of(types.to_vec()))
                .await
                .map_err(|e| dbal(&e))?;
            if self_referential {
                conn.delete(
                    &join_table_name,
                    &combine(&other_keys),
                    &types_of(types.to_vec()),
                )
                .await
                .map_err(|e| dbal(&e))?;
            }
        }
        Ok(())
    }

    /// Deletes a managed entity.
    ///
    /// The entity to delete must be managed and have a persistent
    /// identifier. The deletion happens instantaneously.
    ///
    /// Subclasses may override this method to customize the semantics of
    /// entity deletion.
    pub async fn delete(&self, entity: &Object) -> Result<bool, Thrown> {
        let em = self.em()?;
        let platform = em.get_platform();
        let identifier = em.get_unit_of_work().get_entity_identifier(entity)?;
        let (table_name, id_columns) = {
            let info = self.class.read();
            (
                self.quote_strategy.get_table_name(&info, platform),
                self.quote_strategy
                    .get_identifier_column_names(&info, platform),
            )
        };
        let mut id = PhpArray::new();
        for (column, value) in id_columns.iter().zip(identifier.values()) {
            id.insert(column.as_str(), value.clone());
        }
        let types = self.get_class_identifiers_types(&em, &self.class)?;
        self.delete_join_table_records(&em, &identifier, &types)
            .await?;
        let deleted = em
            .get_connection()
            .delete(&table_name, &id, &types_of(types))
            .await
            .map_err(|e| dbal(&e))?;
        Ok(deleted != 0)
    }

    /// Prepares the changeset of an entity for database insertion
    /// (UPDATE).
    ///
    /// The changeset is obtained from the currently running `UnitOfWork`.
    ///
    /// During this preparation the array that is passed as the second
    /// parameter is filled with <columnName> => <value> pairs, grouped by
    /// table name.
    ///
    /// Example:
    /// <code>
    /// array(
    ///    'foo_table' => array('column1' => 'value1', 'column2' => 'value2', ...),
    ///    'bar_table' => array('columnX' => 'valueX', 'columnY' => 'valueY', ...),
    ///    ...
    /// )
    /// </code>
    #[allow(clippy::too_many_lines, reason = "upstream's method, in its order")]
    fn prepare_update_data(
        &self,
        em: &Arc<EntityManager>,
        entity: &Object,
        is_insert: bool,
    ) -> Result<PhpArray, Thrown> {
        let platform = em.get_platform();
        let uow = em.get_unit_of_work();
        let mut result: Vec<(String, PhpArray)> = Vec::new();
        let mut push = |table: String, column: String, value: Value| {
            if let Some((_, data)) = result.iter_mut().find(|(t, _)| *t == table) {
                data.insert(column.as_str(), value);
            } else {
                let mut data = PhpArray::new();
                data.insert(column.as_str(), value);
                result.push((table, data));
            }
        };
        let version_field = {
            let info = self.class.read();
            if info.is_versioned {
                info.version_field.clone()
            } else {
                None
            }
        };
        let owning_table = self.class.read().get_table_name()?;

        for (field, change) in uow.get_entity_change_set(entity).iter() {
            let field = key_text(field);
            if version_field.as_deref() == Some(field.as_str()) {
                continue;
            }
            if self.class.read().embedded_classes.contains_key(&field) {
                continue;
            }
            let mut new_val = match change {
                Value::Array(pair) => pair.get(&Key::Int(1)).cloned().unwrap_or(Value::Null),
                _ => Value::Null,
            };

            let assoc = self.class.read().association_mappings.get(&field).cloned();
            let Some(assoc) = assoc else {
                let info = self.class.read();
                let Some(field_mapping) = info.field_mappings.get(&field) else {
                    continue;
                };
                let column_name = field_mapping.column_name.clone();
                if !is_insert && field_mapping.not_updatable.is_some() {
                    continue;
                }
                if is_insert && field_mapping.not_insertable.is_some() {
                    continue;
                }
                self.st()
                    .column_types
                    .insert(column_name.clone(), field_mapping.r#type.clone());
                push(owning_table.clone(), column_name, new_val);
                continue;
            };

            // Only owning side of x-1 associations can have a FK column.
            if !assoc.is_to_one_owning_side() {
                continue;
            }

            if let Value::Object(target) = &new_val {
                let oid = object_id(target);
                let queued = self.st().queued_inserts.iter().any(|(o, _)| *o == oid);
                if (queued || uow.is_scheduled_for_insert(target))
                    && !(object_id(target) == object_id(entity)
                        && self.class.read().is_identifier_natural())
                {
                    let mut extra = PhpArray::new();
                    extra.insert(
                        field.as_str(),
                        Value::Array(PhpArray::list([Value::Null, new_val.clone()])),
                    );
                    uow.schedule_extra_update(entity, extra);
                    new_val = Value::Null;
                }
            }

            let new_val_id = match &new_val {
                Value::Object(target) => Some(uow.get_entity_identifier(target)?),
                _ => None,
            };

            let target_class = em.get_class_metadata(&assoc.target_entity)?;
            let target = target_class.read();
            for join_column in &assoc.join_columns {
                let source_column = join_column.name.clone();
                let target_column = join_column.referenced_column_name.clone();
                let quoted_column = self.quote_strategy.get_join_column_name(
                    join_column,
                    &self.class.read(),
                    platform,
                );
                let ty = get_type_of_column(&target_column, &target, em)?;
                {
                    let mut st = self.st();
                    st.quoted_columns
                        .insert(source_column.clone(), quoted_column);
                    st.column_types.insert(source_column.clone(), ty);
                }
                let new_value = match &new_val_id {
                    Some(id) if !id.is_empty() => id
                        .get(&target.get_field_for_column(&target_column)?.as_str().into())
                        .cloned()
                        .unwrap_or(Value::Null),
                    _ => Value::Null,
                };
                push(
                    owning_table.clone(),
                    source_column,
                    crate::utility::identifier_flattener_backed(new_value),
                );
            }
        }
        let mut out = PhpArray::new();
        for (table, data) in result {
            out.insert(table.as_str(), Value::Array(data));
        }
        Ok(out)
    }

    /// Prepares the data changeset of a managed entity for database
    /// insertion (initial INSERT). The changeset of the entity is obtained
    /// from the currently running `UnitOfWork`.
    ///
    /// The default insert data preparation is the same as for updates.
    fn prepare_insert_data(
        &self,
        em: &Arc<EntityManager>,
        entity: &Object,
    ) -> Result<PhpArray, Thrown> {
        self.prepare_update_data(em, entity, true)
    }

    /// Gets the name of the table that owns the column the given field is
    /// mapped to.
    ///
    /// The default implementation in `BasicEntityPersister` always returns
    /// the name of the table the entity type of this persister is mapped
    /// to, since an entity is always persisted to a single table with a
    /// `BasicEntityPersister`.
    pub fn get_owning_table(&self, _field_name: &str) -> Result<String, Thrown> {
        self.class.read().get_table_name()
    }

    /// Loads an entity by a list of field criteria.
    #[allow(clippy::too_many_arguments, reason = "load(), as PHP declares it")]
    pub async fn load(
        &self,
        criteria: &PhpArray,
        entity: Option<&Object>,
        assoc: Option<&AssociationMapping>,
        hints: Hints,
        limit: Option<i64>,
        order_by: Option<&PhpArray>,
    ) -> Result<Option<Object>, Thrown> {
        let em = self.em()?;
        self.switch_persister_context(None, limit);
        let sql = self.get_select_sql(&em, criteria.into(), assoc, None, limit, None, order_by)?;
        let (params, types) = self.expand_parameters(&em, criteria)?;
        let rows = self.fetch_all(&em, &sql, params, types).await?;

        let mut hints = hints;
        if let Some(entity) = entity {
            hints.refresh = true;
            hints.refresh_entity = Some(entity.clone());
        }
        let rsm = self.get_result_set_mapping();
        let entities = hydration::hydrate_all(&em, rows, &rsm, &mut hints, None).await?;
        Ok(entities.into_iter().next())
    }

    async fn fetch_all(
        &self,
        em: &Arc<EntityManager>,
        sql: &str,
        params: Vec<Value>,
        types: Vec<ParamType>,
    ) -> Result<Vec<PhpArray>, Thrown> {
        let conn = em.get_connection();
        let mut stmt = conn
            .execute_query(sql, &PhpArray::list(params), &types_of(types))
            .await
            .map_err(|e| dbal(&e))?;
        stmt.fetch_all_associative().map_err(|e| dbal(&e))
    }

    /// Loads an entity by identifier.
    pub async fn load_by_id(
        &self,
        identifier: &PhpArray,
        entity: Option<&Object>,
    ) -> Result<Option<Object>, Thrown> {
        self.load(identifier, entity, None, Hints::default(), None, None)
            .await
    }

    /// Loads an entity of this persister's mapped class as part of a
    /// single-valued association from another entity.
    pub async fn load_one_to_one_entity(
        &self,
        assoc: &AssociationMapping,
        source_entity: &Object,
        identifier: Option<&PhpArray>,
    ) -> Result<Option<Object>, Thrown> {
        let em = self.em()?;
        let empty = PhpArray::new();
        let identifier = identifier.unwrap_or(&empty);
        if let Some(found) = em
            .get_unit_of_work()
            .try_get_by_id(identifier, &assoc.target_entity)
        {
            return Ok(Some(found));
        }

        let target_class = em.get_class_metadata(&assoc.target_entity)?;

        if assoc.is_owning_side() {
            let is_inverse_single_valued = assoc
                .inversed_by
                .as_ref()
                .is_some_and(|inv| !target_class.read().is_collection_valued_association(inv));

            // Mark inverse side as fetched in the hints, otherwise the
            // UoW would try to load it in a separate query (remember:
            // to-one inverse sides can not be lazy).
            let mut hints = Hints::default();
            if is_inverse_single_valued && let Some(inv) = &assoc.inversed_by {
                hints.fetched.push(("r".to_owned(), inv.clone()));
            }

            let target_entity = self
                .load(identifier, None, Some(assoc), hints, None, None)
                .await?;

            // Complete bidirectional association, if necessary
            if let (Some(target_entity), true, Some(inv)) =
                (&target_entity, is_inverse_single_valued, &assoc.inversed_by)
            {
                target_class.read().set_field_value(
                    target_entity,
                    inv,
                    Value::Object(source_entity.clone()),
                )?;
            }
            return Ok(target_entity);
        }

        let mapped_by = assoc.mapped_by.clone().unwrap_or_default();
        let source_class = em.get_class_metadata(&assoc.source_entity)?;
        let owning_assoc = target_class
            .read()
            .get_association_mapping(&mapped_by)?
            .clone();

        let mut computed_identifier = PhpArray::new();
        let mut source_entity_data: Option<PhpArray> = None;

        // TRICKY: since the association is specular source and target
        // are flipped
        for (source_key_column, target_key_column) in
            owning_assoc.target_to_source_key_columns.iter()
        {
            let source_key_column = key_text(source_key_column);
            let target_key_column = to_text(target_key_column);
            let target_field = target_class
                .read()
                .get_field_for_column(&target_key_column)?;
            let source_field = source_class
                .read()
                .field_names
                .get(&source_key_column)
                .cloned();
            match source_field {
                None => {
                    // The likely case here is that the column is a join
                    // column in an association mapping. However, there
                    // is no guarantee at this point that a corresponding
                    // (generally identifying) association has been
                    // mapped in the source entity. To handle this case
                    // we directly reference the column-keyed data used
                    // to initialize the source entity before throwing an
                    // exception.
                    let data = source_entity_data.get_or_insert_with(|| {
                        em.get_unit_of_work()
                            .get_original_entity_data(source_entity)
                    });
                    match data.get(&source_key_column.as_str().into()) {
                        Some(value) if !value.is_null() => {
                            computed_identifier.insert(target_field.as_str(), value.clone());
                        }
                        _ => {
                            return Err(crate::mapping::mapping_exception::join_column_must_point_to_mapped_field(
                                &source_class.read().name,
                                &source_key_column,
                            ));
                        }
                    }
                }
                Some(field) => {
                    let value = source_class.read().get_field_value(source_entity, &field)?;
                    computed_identifier.insert(target_field.as_str(), value);
                }
            }
        }

        let target_entity = self
            .load(
                &computed_identifier,
                None,
                Some(assoc),
                Hints::default(),
                None,
                None,
            )
            .await?;
        if let Some(target_entity) = &target_entity {
            target_class.read().set_field_value(
                target_entity,
                &mapped_by,
                Value::Object(source_entity.clone()),
            )?;
        }
        Ok(target_entity)
    }

    /// Refreshes a managed entity.
    pub async fn refresh(&self, id: &PhpArray, _entity: &Object) -> Result<(), Thrown> {
        let em = self.em()?;
        self.switch_persister_context(None, None);
        let sql = self.get_select_sql(&em, id.into(), None, None, None, None, None)?;
        let (params, types) = self.expand_parameters(&em, id)?;
        let rows = self.fetch_all(&em, &sql, params, types).await?;
        let rsm = self.get_result_set_mapping();
        let mut hints = Hints {
            refresh: true,
            ..Hints::default()
        };
        hydration::hydrate_all(&em, rows, &rsm, &mut hints, None).await?;
        Ok(())
    }

    /// Count entities (optionally filtered by a criteria)
    pub async fn count(&self, criteria: &PhpArray) -> Result<i64, Thrown> {
        let em = self.em()?;
        let sql = self.get_count_sql(&em, criteria.into())?;
        let (params, types) = self.expand_parameters(&em, criteria)?;
        let value = em
            .get_connection()
            .fetch_one(&sql, &PhpArray::list(params), &types_of(types))
            .await
            .map_err(|e| dbal(&e))?;
        Ok(match value {
            Some(Value::Int(i)) => i,
            Some(other) => to_text(&other).trim().parse().unwrap_or(0),
            None => 0,
        })
    }

    /// `count(Criteria)`: count entities filtered by a criteria.
    pub async fn count_criteria(&self, criteria: &Criteria) -> Result<i64, Thrown> {
        let em = self.em()?;
        let sql = self.get_count_sql(&em, criteria.into())?;
        let (params, types) = self.expand_criteria_parameters(&em, criteria)?;
        let value = em
            .get_connection()
            .fetch_one(&sql, &PhpArray::list(params), &types_of(types))
            .await
            .map_err(|e| dbal(&e))?;
        Ok(match value {
            Some(Value::Int(i)) => i,
            Some(other) => to_text(&other).trim().parse().unwrap_or(0),
            None => 0,
        })
    }

    /// Loads Entities matching the given Criteria object.
    pub async fn load_criteria(&self, criteria: &Criteria) -> Result<Vec<Object>, Thrown> {
        let em = self.em()?;
        let order_by = orderings_as_string_map(criteria);
        let limit = criteria.get_max_results();
        let offset = criteria.get_first_result();
        let query = self.get_select_sql(
            &em,
            criteria.into(),
            None,
            None,
            limit,
            offset,
            Some(&order_by),
        )?;
        let (params, types) = self.expand_criteria_parameters(&em, criteria)?;
        let rows = self.fetch_all(&em, &query, params, types).await?;
        let rsm = self.get_result_set_mapping();
        let mut hints = Hints {
            defer_eager_load: true,
            ..Hints::default()
        };
        hydration::hydrate_all(&em, rows, &rsm, &mut hints, None).await
    }

    /// Expands Criteria Parameters by walking the expressions and grabbing
    /// all parameters and types from it.
    pub fn expand_criteria_parameters(
        &self,
        em: &EntityManager,
        criteria: &Criteria,
    ) -> Result<Params, Thrown> {
        let mut sql_params = Vec::new();
        let mut sql_types = Vec::new();
        let Some(expression) = criteria.get_where_expression() else {
            return Ok((sql_params, sql_types));
        };

        let mut value_visitor = SqlValueVisitor::new();
        value_visitor.dispatch(expression)?;
        let (_, types) = value_visitor.get_params_and_types();

        let info = self.class.read();
        for ty in types {
            let (field, value, operator) = (ty.field, ty.value, ty.operator);
            if value.is_null() && (operator == Comparison::EQ || operator == Comparison::NEQ) {
                continue;
            }
            if operator == Comparison::IN || operator == Comparison::NIN {
                let values = match value {
                    Value::Array(values) => values,
                    other => PhpArray::list(vec![other]),
                };
                for item in values.values() {
                    if item.is_null() {
                        // Compare this to how getSelectConditionStatementSQL
                        // creates the "[NOT] IN (...)" expression - for NULL
                        // values, it does _not_ insert a placeholder in the
                        // SQL and instead adds an extra ... OR ... IS NULL
                        // condition. So we need to skip NULL values here as
                        // well to create a parameters list that matches the
                        // SQL.
                        continue;
                    }
                    sql_params.extend(convert_to_parameter_value(item, em)?);
                    sql_types.extend(infer_parameter_types(&field, item, &info, em)?);
                }
                continue;
            }
            sql_params.extend(convert_to_parameter_value(&value, em)?);
            sql_types.extend(infer_parameter_types(&field, &value, &info, em)?);
        }
        Ok((sql_params, sql_types))
    }

    /// Loads a list of entities by a list of field criteria.
    pub async fn load_all(
        &self,
        criteria: &PhpArray,
        order_by: Option<&PhpArray>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Object>, Thrown> {
        let em = self.em()?;
        self.switch_persister_context(offset, limit);
        let sql = self.get_select_sql(&em, criteria.into(), None, None, limit, offset, order_by)?;
        let (params, types) = self.expand_parameters(&em, criteria)?;
        let rows = self.fetch_all(&em, &sql, params, types).await?;
        let rsm = self.get_result_set_mapping();
        let mut hints = Hints {
            defer_eager_load: true,
            ..Hints::default()
        };
        hydration::hydrate_all(&em, rows, &rsm, &mut hints, None).await
    }

    /// Gets (sliced or full) elements of the given collection.
    pub async fn get_many_to_many_collection(
        &self,
        assoc: &AssociationMapping,
        source_entity: &Object,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> Result<PhpArray, Thrown> {
        let em = self.em()?;
        self.switch_persister_context(offset, limit);
        let rows = self
            .get_many_to_many_statement(&em, assoc, source_entity, offset, limit)
            .await?;
        self.load_array_from_result(&em, assoc, rows).await
    }

    async fn load_array_from_result(
        &self,
        em: &Arc<EntityManager>,
        assoc: &AssociationMapping,
        rows: Vec<PhpArray>,
    ) -> Result<PhpArray, Thrown> {
        let rsm = self.get_result_set_mapping();
        let mut hints = Hints {
            defer_eager_load: true,
            ..Hints::default()
        };
        let entities = hydration::hydrate_all(em, rows, &rsm, &mut hints, None).await?;
        if !assoc.is_indexed() {
            return Ok(PhpArray::list(entities.into_iter().map(Value::Object)));
        }
        // `$rsm->addIndexBy('r', $assoc->indexBy())`: the entities keyed
        // by the field.
        let index_by = assoc.index_by()?.to_owned();
        let mut result = PhpArray::new();
        for entity in entities {
            let entity_class = em.get_class_metadata(entity.class())?;
            let key = entity_class.read().get_field_value(&entity, &index_by)?;
            let key = Key::from_value(&key).unwrap_or_else(|| Key::from(to_text(&key).as_str()));
            result.insert(key, Value::Object(entity));
        }
        Ok(result)
    }

    async fn load_collection_from_statement(
        &self,
        em: &Arc<EntityManager>,
        assoc: &AssociationMapping,
        rows: Vec<PhpArray>,
        coll: &Arc<PersistentCollection>,
    ) -> Result<Vec<Object>, Thrown> {
        let rsm = self.get_result_set_mapping();
        let mut hints = Hints {
            defer_eager_load: true,
            ..Hints::default()
        };
        let index_by = if assoc.is_indexed() {
            Some(assoc.index_by()?.to_owned())
        } else {
            None
        };
        hydration::hydrate_all(
            em,
            rows,
            &rsm,
            &mut hints,
            Some(CollectionTarget {
                collection: coll,
                index_by,
            }),
        )
        .await
    }

    /// Loads a collection of entities of a many-to-many association.
    pub async fn load_many_to_many_collection(
        &self,
        assoc: &AssociationMapping,
        source_entity: &Object,
        collection: &Arc<PersistentCollection>,
    ) -> Result<Vec<Object>, Thrown> {
        let em = self.em()?;
        let rows = self
            .get_many_to_many_statement(&em, assoc, source_entity, None, None)
            .await?;
        self.load_collection_from_statement(&em, assoc, rows, collection)
            .await
    }

    async fn get_many_to_many_statement(
        &self,
        em: &Arc<EntityManager>,
        assoc: &AssociationMapping,
        source_entity: &Object,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<PhpArray>, Thrown> {
        self.switch_persister_context(offset, limit);
        let platform = em.get_platform();
        let source_class = em.get_class_metadata(&assoc.source_entity)?;
        let class = if assoc.is_owning_side() {
            source_class.clone()
        } else {
            em.get_class_metadata(&assoc.target_entity)?
        };
        let association = em.get_metadata_factory().get_owning_side(assoc)?;
        let join_table = association
            .join_table
            .clone()
            .unwrap_or_else(|| crate::mapping::JoinTableMapping::new(""));
        let join_columns = if assoc.is_owning_side() {
            join_table.join_columns.clone()
        } else {
            join_table.inverse_join_columns.clone()
        };
        let quoted_join_table =
            self.quote_strategy
                .get_join_table_name(&association, &class.read(), platform);

        let mut criteria = PhpArray::new();
        let mut parameters = Vec::new();
        for join_column in &join_columns {
            let source_key_column = join_column.referenced_column_name.clone();
            let quoted_key_column =
                self.quote_strategy
                    .get_join_column_name(join_column, &class.read(), platform);
            let (field, value) = {
                let source = source_class.read();
                if source.contains_foreign_identifier {
                    let field = source.get_field_for_column(&source_key_column)?;
                    let mut value = source.get_field_value(source_entity, &field)?;
                    if let (Some(assoc), Value::Object(related)) =
                        (source.association_mappings.get(&field), &value)
                    {
                        let id = em.get_unit_of_work().get_entity_identifier(related)?;
                        let target = em.get_class_metadata(&assoc.target_entity)?;
                        let first = target
                            .read()
                            .identifier
                            .first()
                            .cloned()
                            .unwrap_or_default();
                        value = id
                            .get(&first.as_str().into())
                            .cloned()
                            .unwrap_or(Value::Null);
                    }
                    (field, value)
                } else if let Some(field) = source.field_names.get(&source_key_column) {
                    let value = source.get_field_value(source_entity, field)?;
                    (field.clone(), value)
                } else {
                    return Err(
                        crate::mapping::mapping_exception::join_column_must_point_to_mapped_field(
                            &source.name,
                            &source_key_column,
                        ),
                    );
                }
            };
            criteria.insert(
                format!("{quoted_join_table}.{quoted_key_column}").as_str(),
                value.clone(),
            );
            parameters.push(ToManyCriterion {
                value,
                field,
                class: source_class.clone(),
            });
        }

        let sql = self.get_select_sql(
            em,
            (&criteria).into(),
            Some(assoc),
            None,
            limit,
            offset,
            None,
        )?;
        let (params, types) = self.expand_to_many_parameters(em, &parameters)?;
        self.fetch_all(em, &sql, params, types).await
    }

    /// Gets the SELECT SQL to select one or more entities by a set of
    /// field criteria.
    #[allow(
        clippy::too_many_arguments,
        reason = "getSelectSQL(), as PHP declares it"
    )]
    pub fn get_select_sql(
        &self,
        em: &Arc<EntityManager>,
        criteria: PersisterCriteria<'_>,
        assoc: Option<&AssociationMapping>,
        lock_mode: Option<LockMode>,
        limit: Option<i64>,
        offset: Option<i64>,
        order_by: Option<&PhpArray>,
    ) -> Result<String, Thrown> {
        self.switch_persister_context(offset, limit);
        let platform = em.get_platform();
        let mut join_sql = String::new();
        let mut order_by_sql = String::new();

        if let Some(assoc) = assoc
            && assoc.is_many_to_many()
        {
            join_sql = self.get_select_many_to_many_join_sql(em, assoc)?;
        }

        let mut order_by = order_by.cloned();
        if let Some(assoc) = assoc
            && assoc.is_ordered()
        {
            order_by = Some(assoc.order_by().clone());
        }

        if let Some(order_by) = order_by.filter(|o| !o.is_empty()) {
            let alias = self.get_sql_table_alias(&self.class.read().name.clone(), "");
            order_by_sql = self.get_order_by_sql(em, &order_by, &alias)?;
        }

        let condition_sql = match criteria {
            PersisterCriteria::Criteria(criteria) => {
                self.get_select_condition_criteria_sql(em, criteria)?
            }
            PersisterCriteria::Array(criteria) => {
                self.get_select_condition_sql(em, criteria, assoc)?
            }
        };
        let column_list = self.get_select_columns_sql(em)?;
        let table_alias = self.get_sql_table_alias(&self.class.read().name.clone(), "");
        let table_name = self
            .quote_strategy
            .get_table_name(&self.class.read(), platform);

        let select = format!("SELECT {column_list}");
        let from = format!(" FROM {table_name} {table_alias}");
        let join = format!("{}{join_sql}", self.st().ctx().select_join_sql);
        let where_ = if condition_sql.is_empty() {
            String::new()
        } else {
            format!(" WHERE {condition_sql}")
        };
        let lock = platform.append_lock_hint(&from, lock_mode.unwrap_or(LockMode::None));
        let query = format!("{select}{lock}{join}{where_}{order_by_sql}");
        platform
            .modify_limit_query(&query, limit, offset.unwrap_or(0))
            .map_err(|e| {
                Thrown::new(
                    "Doctrine\\DBAL\\Exception\\InvalidArgumentException",
                    e.message(),
                    0,
                )
            })
    }

    /// Get the COUNT SQL to count entities (optionally filtered by a
    /// criteria)
    pub fn get_count_sql(
        &self,
        em: &Arc<EntityManager>,
        criteria: PersisterCriteria<'_>,
    ) -> Result<String, Thrown> {
        let platform = em.get_platform();
        let table_name = self
            .quote_strategy
            .get_table_name(&self.class.read(), platform);
        let table_alias = self.get_sql_table_alias(&self.class.read().name.clone(), "");
        let condition_sql = match criteria {
            PersisterCriteria::Criteria(criteria) => {
                self.get_select_condition_criteria_sql(em, criteria)?
            }
            PersisterCriteria::Array(criteria) => {
                self.get_select_condition_sql(em, criteria, None)?
            }
        };
        Ok(format!(
            "SELECT COUNT(*) FROM {table_name} {table_alias}{}",
            if condition_sql.is_empty() {
                String::new()
            } else {
                format!(" WHERE {condition_sql}")
            }
        ))
    }

    /// Gets the ORDER BY SQL snippet for ordered collections.
    fn get_order_by_sql(
        &self,
        em: &Arc<EntityManager>,
        order_by: &PhpArray,
        base_table_alias: &str,
    ) -> Result<String, Thrown> {
        let platform = em.get_platform();
        let mut order_by_list = Vec::new();
        for (field_name, orientation) in order_by.iter() {
            let field_name = key_text(field_name);
            let orientation = match orientation {
                Value::Object(o) if o.class().ends_with("SortDirection") => {
                    match o.property(b"name").map(|n| to_text(&n)).as_deref() {
                        Some("Ascending") => "ASC".to_owned(),
                        _ => "DESC".to_owned(),
                    }
                }
                other => to_text(other).trim().to_ascii_uppercase(),
            };
            if orientation != "ASC" && orientation != "DESC" {
                return Err(exception::invalid_orientation(
                    &self.class.read().name,
                    &field_name,
                ));
            }

            let (field_inherited, assoc) = {
                let info = self.class.read();
                (
                    info.field_mappings
                        .get(&field_name)
                        .map(|m| m.inherited.clone()),
                    info.association_mappings.get(&field_name).cloned(),
                )
            };
            if let Some(inherited) = field_inherited {
                let table_alias = match inherited {
                    Some(inherited) => self.get_sql_table_alias(&inherited, ""),
                    None => base_table_alias.to_owned(),
                };
                let column_name =
                    self.quote_strategy
                        .get_column_name(&field_name, &self.class.read(), platform);
                order_by_list.push(format!("{table_alias}.{column_name} {orientation}"));
                continue;
            }

            if let Some(association) = assoc {
                if !association.is_owning_side() {
                    return Err(exception::invalid_find_by_call_from_inverse_side_usage(
                        &self.class.read().name,
                        &field_name,
                    ));
                }
                let table_alias = match &association.inherited {
                    Some(inherited) => self.get_sql_table_alias(inherited, ""),
                    None => base_table_alias.to_owned(),
                };
                for join_column in &association.join_columns {
                    let column_name = self.quote_strategy.get_join_column_name(
                        join_column,
                        &self.class.read(),
                        platform,
                    );
                    order_by_list.push(format!("{table_alias}.{column_name} {orientation}"));
                }
                continue;
            }

            return Err(exception::unrecognized_field(
                &self.class.read().name,
                &field_name,
            ));
        }
        Ok(format!(" ORDER BY {}", order_by_list.join(", ")))
    }

    /// Gets the SQL fragment with the list of columns to select when
    /// querying for an entity in this persister.
    ///
    /// Subclasses should override this method to alter or change the
    /// select column list SQL fragment. Note that in the implementation of
    /// `BasicEntityPersister` the resulting SQL fragment is generated only
    /// once and cached in {@link selectColumnListSql}. Subclasses may or
    /// may not do the same.
    fn get_select_columns_sql(&self, em: &Arc<EntityManager>) -> Result<String, Thrown> {
        if let Some(sql) = self.st().ctx().select_column_list_sql.clone() {
            return Ok(sql);
        }
        let mut column_list = Vec::new();
        let class_name = self.class.read().name.clone();
        self.st()
            .ctx()
            .rsm
            .add_entity_result(&class_name, "r", None); // r for root

        let field_names: Vec<String> = self.class.read().field_names.values().cloned().collect();
        for field in field_names {
            column_list.push(self.get_select_column_sql(em, &field, &self.class, "r")?);
        }

        self.st().ctx().select_join_sql = String::new();

        let associations: Vec<(String, AssociationMapping)> = self
            .class
            .read()
            .association_mappings
            .iter()
            .map(|(k, v)| (k.to_owned(), v.clone()))
            .collect();
        for (assoc_field, assoc) in associations {
            let assoc_column_sql =
                self.get_select_column_association_sql(em, &assoc_field, &assoc, &self.class, "r")?;
            if !assoc_column_sql.is_empty() {
                column_list.push(assoc_column_sql);
            }
        }

        let sql = column_list.join(", ");
        self.st().ctx().select_column_list_sql = Some(sql.clone());
        Ok(sql)
    }

    /// Gets the SQL join fragment used when selecting entities from an
    /// association.
    fn get_select_column_association_sql(
        &self,
        em: &Arc<EntityManager>,
        _field: &str,
        assoc: &AssociationMapping,
        class: &ClassMetadata,
        alias: &str,
    ) -> Result<String, Thrown> {
        if !assoc.is_to_one_owning_side() {
            return Ok(String::new());
        }
        let platform = em.get_platform();
        let mut column_list = Vec::new();
        let target_class = em.get_class_metadata(&assoc.target_entity)?;
        let is_identifier = assoc.id == Some(true);
        let sql_table_alias = self.get_sql_table_alias(
            &class.read().name.clone(),
            if alias == "r" { "" } else { alias },
        );

        for join_column in &assoc.join_columns {
            let quoted_column =
                self.quote_strategy
                    .get_join_column_name(join_column, &self.class.read(), platform);
            let result_column_name = self.get_sql_column_alias(em, &join_column.name);
            let ty = get_type_of_column(
                &join_column.referenced_column_name,
                &target_class.read(),
                em,
            )?;
            self.st().ctx().rsm.add_meta_result(
                alias,
                &result_column_name,
                &join_column.name,
                is_identifier,
                Some(&ty),
            );
            column_list.push(format!(
                "{sql_table_alias}.{quoted_column} AS {result_column_name}"
            ));
        }
        Ok(column_list.join(", "))
    }

    /// Gets the SQL join fragment used when selecting entities from a
    /// many-to-many association.
    fn get_select_many_to_many_join_sql(
        &self,
        em: &Arc<EntityManager>,
        many_to_many: &AssociationMapping,
    ) -> Result<String, Thrown> {
        let platform = em.get_platform();
        let mut conditions = Vec::new();
        let source_table_alias = self.get_sql_table_alias(&self.class.read().name.clone(), "");
        let association = em.get_metadata_factory().get_owning_side(many_to_many)?;
        let info = self.class.read();
        let join_table_name =
            self.quote_strategy
                .get_join_table_name(&association, &info, platform);
        let join_table = association
            .join_table
            .clone()
            .unwrap_or_else(|| crate::mapping::JoinTableMapping::new(""));
        let join_columns: Vec<JoinColumnMapping> = if many_to_many.is_owning_side() {
            join_table.inverse_join_columns.clone()
        } else {
            join_table.join_columns.clone()
        };
        for join_column in &join_columns {
            let quoted_source_column =
                self.quote_strategy
                    .get_join_column_name(join_column, &info, platform);
            let quoted_target_column =
                self.quote_strategy
                    .get_referenced_join_column_name(join_column, &info, platform);
            conditions.push(format!(
                "{source_table_alias}.{quoted_target_column} = {join_table_name}.{quoted_source_column}"
            ));
        }
        Ok(format!(
            " INNER JOIN {join_table_name} ON {}",
            conditions.join(" AND ")
        ))
    }

    /// Gets the INSERT SQL used by the persister to persist a new entity.
    pub fn get_insert_sql(&self, em: &Arc<EntityManager>) -> Result<String, Thrown> {
        let platform = em.get_platform();
        let columns = self.get_insert_column_list(platform);
        let info = self.class.read();
        let table_name = self.quote_strategy.get_table_name(&info, platform);

        if columns.is_empty() {
            let first = info.identifier.first().cloned().unwrap_or_default();
            let identity_column = self.quote_strategy.get_column_name(&first, &info, platform);
            return Ok(platform.get_empty_identity_insert_sql(&table_name, &identity_column));
        }

        let mut unique: Vec<String> = Vec::new();
        for column in columns {
            if !unique.contains(&column) {
                unique.push(column);
            }
        }
        let column_types = self.st().column_types.clone();
        let mut placeholders = Vec::new();
        for column in &unique {
            let mut placeholder = "?".to_owned();
            if let Some(field) = info.field_names.get(column)
                && let Some(ty) = column_types.get(field)
                && info.field_mappings.contains_key(field)
            {
                let ty = types::get_type(ty).map_err(|e| Thrown::new(e.class(), e.message(), 0))?;
                placeholder = ty
                    .convert_to_database_value_sql(b"?", platform)
                    .to_string_lossy()
                    .into_owned();
            }
            placeholders.push(placeholder);
        }
        Ok(format!(
            "INSERT INTO {table_name} ({}) VALUES ({})",
            unique.join(", "),
            placeholders.join(", ")
        ))
    }

    /// Gets the list of columns to put in the INSERT SQL statement.
    ///
    /// Subclasses should override this method to alter or change the list
    /// of columns placed in the INSERT statements used by the persister.
    fn get_insert_column_list(&self, platform: &dyn AbstractPlatform) -> Vec<String> {
        let info = self.class.read();
        let mut columns = Vec::new();
        for (name, _) in info.property_accessors.iter() {
            if info.is_versioned && info.version_field.as_deref() == Some(name) {
                continue;
            }
            if info.embedded_classes.contains_key(name) {
                continue;
            }
            if let Some(assoc) = info.association_mappings.get(name) {
                if assoc.is_to_one_owning_side() {
                    for join_column in &assoc.join_columns {
                        columns.push(self.quote_strategy.get_join_column_name(
                            join_column,
                            &info,
                            platform,
                        ));
                    }
                }
                continue;
            }
            if !info.is_id_generator_identity()
                || info.identifier.first().map(String::as_str) != Some(name)
            {
                let Some(mapping) = info.field_mappings.get(name) else {
                    continue;
                };
                if mapping.not_insertable.is_some() {
                    continue;
                }
                columns.push(self.quote_strategy.get_column_name(name, &info, platform));
                self.st()
                    .column_types
                    .insert(name.to_owned(), mapping.r#type.clone());
            }
        }
        columns
    }

    /// Gets the SQL snippet of a qualified column name for the given
    /// field name.
    fn get_select_column_sql(
        &self,
        em: &Arc<EntityManager>,
        field: &str,
        class: &ClassMetadata,
        alias: &str,
    ) -> Result<String, Thrown> {
        let platform = em.get_platform();
        let root = if alias == "r" { "" } else { alias };
        let table_alias = self.get_sql_table_alias(&class.read().name.clone(), root);
        let (column_name, ty, enum_type, quoted) = {
            let info = class.read();
            let mapping = info.get_field_mapping(field)?;
            (
                mapping.column_name.clone(),
                mapping.r#type.clone(),
                mapping.enum_type.clone(),
                self.quote_strategy.get_column_name(field, &info, platform),
            )
        };
        let sql = format!("{table_alias}.{quoted}");
        let column_alias = {
            let mut st = self.st();
            let rsm = &st.ctx().rsm;
            if rsm.has_column_alias_by_field(alias, field) {
                rsm.get_column_alias_by_field(alias, field)
            } else {
                None
            }
        };
        let column_alias = match column_alias {
            Some(a) => a,
            None => self.get_sql_column_alias(em, &column_name),
        };
        {
            let mut st = self.st();
            st.ctx()
                .rsm
                .add_field_result(alias, &column_alias, field, None);
            if let Some(enum_type) = enum_type.filter(|t| !t.is_empty()) {
                st.ctx().rsm.add_enum_result(&column_alias, &enum_type);
            }
        }
        let ty = types::get_type(&ty).map_err(|e| Thrown::new(e.class(), e.message(), 0))?;
        let sql = ty.convert_to_php_value_sql(sql.as_bytes(), platform);
        Ok(format!("{} AS {column_alias}", sql.to_string_lossy()))
    }

    /// Gets the SQL table alias for the given class name.
    fn get_sql_table_alias(&self, class_name: &str, assoc_name: &str) -> String {
        let key = if assoc_name.is_empty() {
            class_name.to_owned()
        } else {
            format!("{class_name}#{assoc_name}")
        };
        let mut st = self.st();
        let ctx = st.ctx();
        if let Some(alias) = ctx.sql_table_aliases.get(&key) {
            return alias.clone();
        }
        let table_alias = format!("t{}", ctx.sql_alias_counter);
        ctx.sql_alias_counter += 1;
        ctx.sql_table_aliases.insert(key, table_alias.clone());
        table_alias
    }

    /// Gets the conditional SQL fragment used in the WHERE clause when
    /// selecting entities in this persister.
    ///
    /// Subclasses are supposed to override this method if they intend to
    /// change or alter the criteria by which entities are selected.
    pub fn get_select_condition_statement_sql(
        &self,
        em: &Arc<EntityManager>,
        field: &str,
        value: &Value,
        assoc: Option<&AssociationMapping>,
        comparison: Option<&str>,
    ) -> Result<String, Thrown> {
        let platform = em.get_platform();
        let comparison = comparison.unwrap_or(if matches!(value, Value::Array(_)) {
            Comparison::IN
        } else {
            Comparison::EQ
        });

        let mut selected_columns = Vec::new();
        let columns = self.get_select_condition_statement_column_sql(em, field, assoc)?;

        if columns.len() > 1 && comparison == Comparison::IN {
            // @todo try to support multi-column IN expressions.
            // Example: (col1, col2) IN (('val1A', 'val2A'), ('val1B', 'val2B'))
            return Err(exception::cant_use_in_operator_on_composite_keys());
        }

        for column in columns {
            let mut placeholder = "?".to_owned();
            if let Some(mapping) = self.class.read().field_mappings.get(field) {
                let ty = types::get_type(&mapping.r#type)
                    .map_err(|e| Thrown::new(e.class(), e.message(), 0))?;
                placeholder = ty
                    .convert_to_database_value_sql(placeholder.as_bytes(), platform)
                    .to_string_lossy()
                    .into_owned();
            }

            // special case null value handling
            if (comparison == Comparison::EQ || comparison == Comparison::IS) && value.is_null() {
                selected_columns.push(format!("{column} IS NULL"));
                continue;
            }

            if comparison == Comparison::NEQ && value.is_null() {
                selected_columns.push(format!("{column} IS NOT NULL"));
                continue;
            }

            if comparison == Comparison::IN || comparison == Comparison::NIN {
                let values = match value {
                    Value::Array(values) => values.clone(),
                    other => PhpArray::list(vec![other.clone()]),
                };
                if values.is_empty() {
                    selected_columns.push("1=0".to_owned());
                    continue;
                }
                let non_null = values.values().filter(|v| !v.is_null()).count();
                let has_null = non_null < values.len();
                let placeholders = vec![placeholder.as_str(); non_null].join(", ");
                let in_ = format!("{column} {}", comparison_sql(comparison, &placeholders)?);
                if has_null {
                    if non_null > 0 {
                        selected_columns.push(format!("({in_} OR {column} IS NULL)"));
                    } else {
                        selected_columns.push(format!("{column} IS NULL"));
                    }
                } else {
                    selected_columns.push(in_);
                }
                continue;
            }

            selected_columns.push(format!(
                "{column} {}",
                comparison_sql(comparison, &placeholder)?
            ));
        }
        Ok(selected_columns.join(" AND "))
    }

    /// Gets the Select Where Condition from a Criteria object.
    fn get_select_condition_criteria_sql(
        &self,
        _em: &Arc<EntityManager>,
        criteria: &Criteria,
    ) -> Result<String, Thrown> {
        let Some(expression) = criteria.get_where_expression() else {
            return Ok(String::new());
        };
        let mut visitor = SqlExpressionVisitor::new(self, &self.class);
        visitor.dispatch(expression)
    }

    /// Builds the left-hand-side of a where condition statement.
    fn get_select_condition_statement_column_sql(
        &self,
        em: &Arc<EntityManager>,
        field: &str,
        assoc: Option<&AssociationMapping>,
    ) -> Result<Vec<String>, Thrown> {
        let platform = em.get_platform();
        let (field_inherited, association, class_name) = {
            let info = self.class.read();
            (
                info.field_mappings.get(field).map(|m| m.inherited.clone()),
                info.association_mappings.get(field).cloned(),
                info.name.clone(),
            )
        };
        if let Some(inherited) = field_inherited {
            let class_name = inherited.unwrap_or(class_name);
            let alias = self.get_sql_table_alias(&class_name, "");
            return Ok(vec![format!(
                "{alias}.{}",
                self.quote_strategy
                    .get_column_name(field, &self.class.read(), platform)
            )]);
        }

        if let Some(association) = association {
            let mut columns = Vec::new();
            if association.is_many_to_many() {
                let Some(assoc) = assoc else {
                    return Err(Thrown::new("AssertionError", "assert($assoc !== null)", 0));
                };
                let association = if association.is_owning_side() {
                    association
                } else {
                    assoc.clone()
                };
                let info = self.class.read();
                let join_table_name =
                    self.quote_strategy
                        .get_join_table_name(&association, &info, platform);
                let join_table = association
                    .join_table
                    .clone()
                    .unwrap_or_else(|| crate::mapping::JoinTableMapping::new(""));
                let join_columns = if assoc.is_owning_side() {
                    join_table.join_columns
                } else {
                    join_table.inverse_join_columns
                };
                for join_column in &join_columns {
                    columns.push(format!(
                        "{join_table_name}.{}",
                        self.quote_strategy
                            .get_join_column_name(join_column, &info, platform)
                    ));
                }
            } else {
                if !association.is_owning_side() {
                    return Err(exception::invalid_find_by_call_from_inverse_side_usage(
                        &class_name,
                        field,
                    ));
                }
                let class_name = association.inherited.clone().unwrap_or(class_name);
                let alias = self.get_sql_table_alias(&class_name, "");
                for join_column in &association.join_columns {
                    columns.push(format!(
                        "{alias}.{}",
                        self.quote_strategy.get_join_column_name(
                            join_column,
                            &self.class.read(),
                            platform
                        )
                    ));
                }
            }
            return Ok(columns);
        }

        if assoc.is_some() && !field.contains(' ') && !field.contains('(') {
            // very careless developers could potentially open up this
            // normally hidden api for userland attacks, therefore checking
            // for spaces and function calls which are not allowed.

            // found a join column condition, not really a "field"
            return Ok(vec![field.to_owned()]);
        }

        Err(exception::unrecognized_field(&class_name, field))
    }

    /// Gets the conditional SQL fragment used in the WHERE clause when
    /// selecting entities in this persister.
    fn get_select_condition_sql(
        &self,
        em: &Arc<EntityManager>,
        criteria: &PhpArray,
        assoc: Option<&AssociationMapping>,
    ) -> Result<String, Thrown> {
        let mut conditions = Vec::new();
        for (field, value) in criteria.iter() {
            conditions.push(self.get_select_condition_statement_sql(
                em,
                &key_text(field),
                value,
                assoc,
                None,
            )?);
        }
        Ok(conditions.join(" AND "))
    }

    /// Returns an array with (sliced or full list) of elements in the
    /// specified collection.
    pub async fn get_one_to_many_collection(
        &self,
        assoc: &AssociationMapping,
        source_entity: &Object,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> Result<PhpArray, Thrown> {
        let em = self.em()?;
        self.switch_persister_context(offset, limit);
        let rows = self
            .get_one_to_many_statement(&em, assoc, source_entity, offset, limit)
            .await?;
        self.load_array_from_result(&em, assoc, rows).await
    }

    /// Loads a collection of entities in a one-to-many association.
    pub async fn load_one_to_many_collection(
        &self,
        assoc: &AssociationMapping,
        source_entity: &Object,
        collection: &Arc<PersistentCollection>,
    ) -> Result<Vec<Object>, Thrown> {
        let em = self.em()?;
        let rows = self
            .get_one_to_many_statement(&em, assoc, source_entity, None, None)
            .await?;
        self.load_collection_from_statement(&em, assoc, rows, collection)
            .await
    }

    /// Builds criteria and execute SQL statement to fetch the one to many
    /// entities from.
    async fn get_one_to_many_statement(
        &self,
        em: &Arc<EntityManager>,
        assoc: &AssociationMapping,
        source_entity: &Object,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<PhpArray>, Thrown> {
        self.switch_persister_context(offset, limit);
        let mut criteria = PhpArray::new();
        let mut parameters = Vec::new();
        let mapped_by = assoc.mapped_by.clone().unwrap_or_default();
        let owning_assoc = self
            .class
            .read()
            .get_association_mapping(&mapped_by)?
            .clone();
        let source_class = em.get_class_metadata(&assoc.source_entity)?;
        let alias_class = owning_assoc
            .inherited
            .clone()
            .unwrap_or_else(|| self.class.read().name.clone());
        let table_alias = self.get_sql_table_alias(&alias_class, "");

        for (source_key_column, target_key_column) in
            owning_assoc.target_to_source_key_columns.iter()
        {
            let source_key_column = key_text(source_key_column);
            let target_key_column = to_text(target_key_column);
            let source = source_class.read();
            if source.contains_foreign_identifier {
                let field = source.get_field_for_column(&source_key_column)?;
                let mut value = source.get_field_value(source_entity, &field)?;
                if let (Some(assoc), Value::Object(related)) =
                    (source.association_mappings.get(&field), &value)
                {
                    let id = em.get_unit_of_work().get_entity_identifier(related)?;
                    let target = em.get_class_metadata(&assoc.target_entity)?;
                    let first = target
                        .read()
                        .identifier
                        .first()
                        .cloned()
                        .unwrap_or_default();
                    value = id
                        .get(&first.as_str().into())
                        .cloned()
                        .unwrap_or(Value::Null);
                }
                criteria.insert(
                    format!("{table_alias}.{target_key_column}").as_str(),
                    value.clone(),
                );
                parameters.push(ToManyCriterion {
                    value,
                    field,
                    class: source_class.clone(),
                });
                continue;
            }
            let field = source
                .field_names
                .get(&source_key_column)
                .cloned()
                .unwrap_or_default();
            let value = source.get_field_value(source_entity, &field)?;
            criteria.insert(
                format!("{table_alias}.{target_key_column}").as_str(),
                value.clone(),
            );
            parameters.push(ToManyCriterion {
                value,
                field,
                class: source_class.clone(),
            });
        }

        let sql = self.get_select_sql(
            em,
            (&criteria).into(),
            Some(assoc),
            None,
            limit,
            offset,
            None,
        )?;
        let (params, types) = self.expand_to_many_parameters(em, &parameters)?;
        self.fetch_all(em, &sql, params, types).await
    }

    /// Expands the parameters from the given criteria and use the correct
    /// binding types if found.
    pub fn expand_parameters(
        &self,
        em: &EntityManager,
        criteria: &PhpArray,
    ) -> Result<Params, Thrown> {
        let info = self.class.read();
        let mut params = Vec::new();
        let mut types = Vec::new();
        for (field, value) in criteria.iter() {
            if value.is_null() {
                continue; // skip null values.
            }
            let field = key_text(field);
            if let Value::Array(values) = value {
                for item in values.values().filter(|v| !v.is_null()) {
                    types.extend(infer_parameter_types(&field, item, &info, em)?);
                    params.extend(convert_to_parameter_value(item, em)?);
                }
                continue;
            }
            types.extend(infer_parameter_types(&field, value, &info, em)?);
            params.extend(convert_to_parameter_value(value, em)?);
        }
        Ok((params, types))
    }

    /// Expands the parameters from the given criteria and use the correct
    /// binding types if found, specialized for `OneToMany` or `ManyToMany`
    /// associations.
    #[allow(clippy::unused_self, reason = "an instance method upstream")]
    fn expand_to_many_parameters(
        &self,
        em: &EntityManager,
        criteria: &[ToManyCriterion],
    ) -> Result<Params, Thrown> {
        let mut params = Vec::new();
        let mut types = Vec::new();
        for criterion in criteria {
            if criterion.value.is_null() {
                continue; // skip null values.
            }
            types.extend(infer_parameter_types(
                &criterion.field,
                &criterion.value,
                &criterion.class.read(),
                em,
            )?);
            params.extend(convert_to_parameter_value(&criterion.value, em)?);
        }
        Ok((params, types))
    }

    /// Checks whether the given managed entity exists in the database.
    pub async fn exists(
        &self,
        entity: &Object,
        extra_conditions: Option<&Criteria>,
    ) -> Result<bool, Thrown> {
        let em = self.em()?;
        let criteria = self.class.read().get_identifier_values(entity)?;
        if criteria.is_empty() {
            return Ok(false);
        }
        let platform = em.get_platform();
        let alias = self.get_sql_table_alias(&self.class.read().name.clone(), "");
        let from = format!(
            "FROM {} {alias}",
            self.quote_strategy
                .get_table_name(&self.class.read(), platform)
        );
        let mut sql = format!(
            "SELECT 1 {} WHERE {}",
            platform.append_lock_hint(&from, LockMode::None),
            self.get_select_condition_sql(&em, &criteria, None)?
        );
        let (mut params, mut types) = self.expand_parameters(&em, &criteria)?;
        if let Some(extra_conditions) = extra_conditions {
            sql.push_str(" AND ");
            sql.push_str(&self.get_select_condition_criteria_sql(&em, extra_conditions)?);
            let (criteria_params, criteria_types) =
                self.expand_criteria_parameters(&em, extra_conditions)?;
            params.extend(criteria_params);
            types.extend(criteria_types);
        }
        let value = em
            .get_connection()
            .fetch_one(&sql, &PhpArray::list(params), &types_of(types))
            .await
            .map_err(|e| dbal(&e))?;
        Ok(value.is_some_and(|v| v.is_truthy()))
    }

    /// Gets an SQL column alias for a column name.
    pub fn get_sql_column_alias(&self, em: &EntityManager, column_name: &str) -> String {
        let counter = {
            let mut st = self.st();
            let ctx = st.ctx();
            let counter = ctx.sql_alias_counter;
            ctx.sql_alias_counter += 1;
            counter
        };
        self.quote_strategy
            .get_column_alias(column_name, counter, em.get_platform(), None)
    }

    /// Switches persister context according to current query offset/limits
    ///
    /// This is due to the fact that to-many associations cannot be
    /// fetch-joined when a limit is involved
    fn switch_persister_context(&self, offset: Option<i64>, limit: Option<i64>) {
        self.st().limits = offset.is_some() || limit.is_some();
    }

    #[allow(clippy::unused_self, reason = "an instance method upstream")]
    fn get_class_identifiers_types(
        &self,
        em: &EntityManager,
        class: &ClassMetadata,
    ) -> Result<Vec<ParamType>, Thrown> {
        let info = class.read();
        let mut types = Vec::new();
        for field_name in &info.identifier {
            let field_types = get_type_of_field(field_name, &info, em)?;
            let Some(first) = field_types.first() else {
                return Err(Thrown::new("AssertionError", "assert(isset($types[0]))", 0));
            };
            types.push(first.as_str().into());
        }
        Ok(types)
    }
}

impl super::EntityPersister for BasicEntityPersister {
    fn count_criteria<'a>(&'a self, criteria: &'a Criteria) -> crate::unit_of_work::Fut<'a, i64> {
        Box::pin(BasicEntityPersister::count_criteria(self, criteria))
    }

    fn load_criteria<'a>(
        &'a self,
        criteria: &'a Criteria,
    ) -> crate::unit_of_work::Fut<'a, Vec<Value>> {
        Box::pin(async move {
            Ok(BasicEntityPersister::load_criteria(self, criteria)
                .await?
                .into_iter()
                .map(Value::Object)
                .collect())
        })
    }

    fn exists<'a>(
        &'a self,
        entity: &'a Object,
        extra_conditions: Option<&'a Criteria>,
    ) -> crate::unit_of_work::Fut<'a, bool> {
        Box::pin(BasicEntityPersister::exists(self, entity, extra_conditions))
    }

    fn get_select_condition_statement_sql(
        &self,
        field: &str,
        value: &Value,
        assoc: Option<&AssociationMapping>,
        comparison: Option<&str>,
    ) -> Result<String, Thrown> {
        let em = self.em()?;
        BasicEntityPersister::get_select_condition_statement_sql(
            self, &em, field, value, assoc, comparison,
        )
    }
}
