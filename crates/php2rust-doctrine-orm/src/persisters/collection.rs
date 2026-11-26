//! The collection persisters: `OneToManyPersister` (the deletion of the
//! elements of an orphan-removing collection) and `ManyToManyPersister`
//! (the rows of the join table), with the extra-lazy operations
//! (`count()`, `slice()`, `contains()`, `containsKey()`, `get()`) and
//! `loadCriteria()`. The persister is chosen by the collection's mapping,
//! so the functions take the collection. The many-to-many operations
//! join the target table for the enabled SQL filters (`getFilterSql()`);
//! joined-inheritance targets are refused
//! (`NotSupported`) where the orphan removal deletes them.
//!
//! Upstream: doctrine/orm src/Persisters/Collection/OneToManyPersister.php src/Persisters/Collection/ManyToManyPersister.php
//! Upstream: doctrine/orm src/Persisters/Collection/AbstractCollectionPersister.php src/Persisters/Collection/CollectionPersister.php

use std::sync::Arc;

use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_collections::expr::{Comparison, ExpressionVisitor};
use php2rust_doctrine_collections::{Criteria, SortDirection};
use php2rust_doctrine_dbal::ParameterTypes;

use crate::entity_manager::EntityManager;
use crate::exception;
use crate::internal::hydration::{self, HydrationHints};
use crate::mapping::ordered_map::OrderedMap;
use crate::mapping::{AssociationMapping, ClassMetadataInfo};
use crate::persistent_collection::PersistentCollection;
use crate::query::ResultSetMappingBuilder;
use crate::unit_of_work::{Hints, STATE_MANAGED, STATE_NEW};
use crate::utility::{
    ParamType, convert_to_parameter_value, get_type_of_column, infer_parameter_types, key_text,
    to_text,
};

use super::SqlValueVisitor;

fn types_of(types: Vec<ParamType>) -> ParameterTypes {
    types
        .into_iter()
        .enumerate()
        .map(|(i, t)| (Key::Int(i64::try_from(i).unwrap_or(i64::MAX)), t))
        .collect()
}

fn owner_of(collection: &PersistentCollection) -> Result<Object, Thrown> {
    collection.get_owner().ok_or_else(|| {
        Thrown::new(
            "TypeError",
            "Doctrine\\ORM\\UnitOfWork::getEntityIdentifier(): Argument #1 ($entity) must be of type object, null given",
            0,
        )
    })
}

/// `CollectionPersister::delete()`: deletes the persistent state
/// represented by the given collection.
pub async fn delete(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
) -> Result<(), Thrown> {
    let mapping = collection.get_mapping()?;
    if mapping.is_one_to_many() {
        one_to_many_delete(em, collection, &mapping).await
    } else {
        many_to_many_delete(em, collection, &mapping).await
    }
}

/// `CollectionPersister::update()`: updates the given collection,
/// synchronizing its state with the database by inserting, updating and
/// deleting individual elements.
pub async fn update(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
) -> Result<(), Thrown> {
    let mapping = collection.get_mapping()?;
    if mapping.is_one_to_many() {
        // This can never happen. One to many can only be inverse side.
        // For owning side one to many, it is required to have a join
        // table, then classifying it as a ManyToManyPersister.
        return Ok(());
    }
    many_to_many_update(em, collection, &mapping).await
}

async fn one_to_many_delete(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
    mapping: &AssociationMapping,
) -> Result<(), Thrown> {
    // The only valid case here is when you have weak entities. In this
    // scenario, you have @OneToMany with orphanRemoval=true, and replacing
    // the entire collection with a new would trigger this operation.
    if !mapping.orphan_removal {
        // Handling non-orphan removal should never happen, as @OneToMany
        // can only be inverse side. For owning side one to many, it is
        // required to have a join table, which would classify as a
        // ManyToManyPersister.
        return Ok(());
    }
    let target_class = em.get_class_metadata(&mapping.target_entity)?;
    if target_class.read().is_inheritance_type_joined() {
        return Err(exception::not_supported());
    }

    // deleteEntityCollection()
    let platform = em.get_platform();
    let quote_strategy = em.get_configuration().get_quote_strategy();
    let identifier = em
        .get_unit_of_work()
        .get_entity_identifier(&owner_of(collection)?)?;
    let source_class = em.get_class_metadata(&mapping.source_entity)?;
    let owning = em.get_metadata_factory().get_owning_side(mapping)?;
    let mut columns = Vec::new();
    let mut parameters = Vec::new();
    let mut types: Vec<ParamType> = Vec::new();
    for join_column in &owning.join_columns {
        columns.push(quote_strategy.get_join_column_name(
            join_column,
            &target_class.read(),
            platform,
        ));
        let field = source_class
            .read()
            .get_field_for_column(&join_column.referenced_column_name)?;
        parameters.push(
            identifier
                .get(&field.as_str().into())
                .cloned()
                .unwrap_or(Value::Null),
        );
        types.push(
            get_type_of_column(
                &join_column.referenced_column_name,
                &source_class.read(),
                em,
            )?
            .as_str()
            .into(),
        );
    }
    let mut statement = format!(
        "DELETE FROM {} WHERE {} = ?",
        quote_strategy.get_table_name(&target_class.read(), platform),
        columns.join(" = ? AND ")
    );
    {
        let target = target_class.read();
        if target.is_inheritance_type_single_table() {
            let discriminator_column = target.get_discriminator_column()?.clone();
            let discriminator_values: Vec<Value> = if target.discriminator_value.is_truthy() {
                vec![target.discriminator_value.clone()]
            } else {
                target.discriminator_map.keys().map(Key::to_value).collect()
            };
            statement.push_str(" AND ");
            statement.push_str(&discriminator_column.name);
            statement.push_str(" IN (");
            statement.push_str(&vec!["?"; discriminator_values.len()].join(", "));
            statement.push(')');
            for value in discriminator_values {
                parameters.push(value);
                types.push(discriminator_column.r#type.as_str().into());
            }
        }
    }
    em.get_connection()
        .execute_statement(&statement, &PhpArray::list(parameters), &types_of(types))
        .await
        .map_err(|e| exception::from_dbal(&e))?;
    Ok(())
}

async fn many_to_many_delete(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
    mapping: &AssociationMapping,
) -> Result<(), Thrown> {
    if !mapping.is_owning_side() {
        return Ok(()); // ignore inverse side
    }
    let class = em.get_class_metadata(&mapping.source_entity)?;
    let join_table = mapping
        .join_table
        .clone()
        .unwrap_or_else(|| crate::mapping::JoinTableMapping::new(""));
    let mut types: Vec<ParamType> = Vec::new();
    for join_column in &join_table.join_columns {
        types.push(
            get_type_of_column(&join_column.referenced_column_name, &class.read(), em)?
                .as_str()
                .into(),
        );
    }

    // getDeleteSQL()
    let platform = em.get_platform();
    let quote_strategy = em.get_configuration().get_quote_strategy();
    let owner = owner_of(collection)?;
    let owner_class = em.get_class_metadata(owner.class())?;
    let (sql, params) = {
        let info = owner_class.read();
        let join_table_name = quote_strategy.get_join_table_name(mapping, &info, platform);
        let columns: Vec<String> = join_table
            .join_columns
            .iter()
            .map(|jc| quote_strategy.get_join_column_name(jc, &info, platform))
            .collect();
        let sql = format!(
            "DELETE FROM {join_table_name} WHERE {} = ?",
            columns.join(" = ? AND ")
        );

        // getDeleteSQLParameters()
        let identifier = em.get_unit_of_work().get_entity_identifier(&owner)?;
        let params: Vec<Value> = if mapping.relation_to_source_key_columns.len() == 1 {
            identifier.values().next().cloned().into_iter().collect()
        } else {
            let source = class.read();
            let mut params = Vec::new();
            for ref_column in mapping.relation_to_source_key_columns.values() {
                let ref_column = to_text(ref_column);
                let field = match source.field_names.get(&ref_column) {
                    Some(field) => field.clone(),
                    None => source.get_field_for_column(&ref_column)?,
                };
                params.push(
                    identifier
                        .get(&field.as_str().into())
                        .cloned()
                        .unwrap_or(Value::Null),
                );
            }
            params
        };
        (sql, params)
    };
    em.get_connection()
        .execute_statement(&sql, &PhpArray::list(params), &types_of(types))
        .await
        .map_err(|e| exception::from_dbal(&e))?;
    Ok(())
}

#[allow(clippy::too_many_lines, reason = "upstream's method, in its order")]
async fn many_to_many_update(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
    mapping: &AssociationMapping,
) -> Result<(), Thrown> {
    if !mapping.is_owning_side() {
        return Ok(()); // ignore inverse side
    }
    let platform = em.get_platform();
    let quote_strategy = em.get_configuration().get_quote_strategy();
    let class = em.get_class_metadata(&mapping.source_entity)?;
    let target_class = em.get_class_metadata(&mapping.target_entity)?;
    let join_table = mapping
        .join_table
        .clone()
        .unwrap_or_else(|| crate::mapping::JoinTableMapping::new(""));

    // getDeleteRowSQL()
    let (delete_sql, delete_types, insert_sql, insert_types) = {
        let source = class.read();
        let target = target_class.read();
        let join_table_name = quote_strategy.get_join_table_name(mapping, &source, platform);
        let mut delete_columns = Vec::new();
        let mut delete_types: Vec<ParamType> = Vec::new();
        for join_column in &join_table.join_columns {
            delete_columns.push(quote_strategy.get_join_column_name(
                join_column,
                &source,
                platform,
            ));
            delete_types.push(
                get_type_of_column(&join_column.referenced_column_name, &source, em)?
                    .as_str()
                    .into(),
            );
        }
        for join_column in &join_table.inverse_join_columns {
            delete_columns.push(quote_strategy.get_join_column_name(
                join_column,
                &target,
                platform,
            ));
            delete_types.push(
                get_type_of_column(&join_column.referenced_column_name, &target, em)?
                    .as_str()
                    .into(),
            );
        }
        let delete_sql = format!(
            "DELETE FROM {join_table_name} WHERE {} = ?",
            delete_columns.join(" = ? AND ")
        );

        // getInsertRowSQL()
        let mut insert_columns = Vec::new();
        let mut insert_types: Vec<ParamType> = Vec::new();
        for join_column in &join_table.join_columns {
            insert_columns.push(quote_strategy.get_join_column_name(
                join_column,
                &target,
                platform,
            ));
            insert_types.push(
                get_type_of_column(&join_column.referenced_column_name, &source, em)?
                    .as_str()
                    .into(),
            );
        }
        for join_column in &join_table.inverse_join_columns {
            insert_columns.push(quote_strategy.get_join_column_name(
                join_column,
                &target,
                platform,
            ));
            insert_types.push(
                get_type_of_column(&join_column.referenced_column_name, &target, em)?
                    .as_str()
                    .into(),
            );
        }
        let insert_sql = format!(
            "INSERT INTO {join_table_name} ({}) VALUES ({})",
            insert_columns.join(", "),
            vec!["?"; insert_columns.len()].join(", ")
        );
        (delete_sql, delete_types, insert_sql, insert_types)
    };

    let conn = em.get_connection();
    for element in collection.get_delete_diff() {
        let Value::Object(element) = element else {
            continue;
        };
        let params = collect_join_table_column_parameters(em, collection, mapping, &element)?;
        conn.execute_statement(
            &delete_sql,
            &PhpArray::list(params),
            &types_of(delete_types.clone()),
        )
        .await
        .map_err(|e| exception::from_dbal(&e))?;
    }
    for element in collection.get_insert_diff() {
        let Value::Object(element) = element else {
            continue;
        };
        let params = collect_join_table_column_parameters(em, collection, mapping, &element)?;
        conn.execute_statement(
            &insert_sql,
            &PhpArray::list(params),
            &types_of(insert_types.clone()),
        )
        .await
        .map_err(|e| exception::from_dbal(&e))?;
    }
    Ok(())
}

/// Collects the parameters for inserting/deleting on the join table in
/// the order of the join table columns as specified in
/// ManyToManyMapping#joinTableColumns.
fn collect_join_table_column_parameters(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
    mapping: &AssociationMapping,
    element: &Object,
) -> Result<Vec<Value>, Thrown> {
    let mut params = Vec::new();
    let is_composite = mapping.join_table_columns.len() > 2;
    let uow = em.get_unit_of_work();
    let owner = owner_of(collection)?;
    let identifier1 = uow.get_entity_identifier(&owner)?;
    let identifier2 = uow.get_entity_identifier(element)?;
    let mut ids1: Vec<Value> = identifier1.values().cloned().collect();
    let mut ids2: Vec<Value> = identifier2.values().cloned().collect();

    let class1 = em.get_class_metadata(owner.class())?;
    let class2 = collection.get_type_class().cloned();

    for join_table_column in &mapping.join_table_columns {
        let key = Key::from(join_table_column.as_str());
        let is_relation_to_source = mapping.relation_to_source_key_columns.contains_key(&key);
        if !is_composite {
            params.push(
                if is_relation_to_source {
                    ids1.pop()
                } else {
                    ids2.pop()
                }
                .unwrap_or(Value::Null),
            );
            continue;
        }
        if is_relation_to_source {
            let column = to_text(
                mapping
                    .relation_to_source_key_columns
                    .get(&key)
                    .unwrap_or(&Value::Null),
            );
            let field = class1.read().get_field_for_column(&column)?;
            params.push(
                identifier1
                    .get(&field.as_str().into())
                    .cloned()
                    .unwrap_or(Value::Null),
            );
            continue;
        }
        let column = to_text(
            mapping
                .relation_to_target_key_columns
                .get(&key)
                .unwrap_or(&Value::Null),
        );
        let field = match &class2 {
            Some(class2) => class2.read().get_field_for_column(&column)?,
            None => key_text(&key),
        };
        params.push(
            identifier2
                .get(&field.as_str().into())
                .cloned()
                .unwrap_or(Value::Null),
        );
    }
    Ok(params)
}

fn bad_method_call(message: &str) -> Thrown {
    Thrown::new("BadMethodCallException", message, 0)
}

const NOT_INDEXED: &str =
    "Selecting a collection by index is only supported on indexed collections.";

fn dbal(e: &php2rust_doctrine_dbal::Exception) -> Thrown {
    exception::from_dbal(e)
}

/// `$id[$field]`.
fn id_value(id: &PhpArray, field: &str) -> Value {
    id.get(&field.into()).cloned().unwrap_or(Value::Null)
}

/// `(int) $value` of a `fetchOne()` result.
fn to_int(value: Option<Value>) -> i64 {
    match value {
        Some(Value::Int(i)) => i,
        Some(other) => to_text(&other).trim().parse().unwrap_or(0),
        None => 0,
    }
}

/// `AbstractCollectionPersister::isValidEntityState()`: check if entity
/// is in a valid state for operations.
fn is_valid_entity_state(em: &EntityManager, entity: &Object) -> bool {
    let uow = em.get_unit_of_work();
    let entity_state = uow.get_entity_state_or(entity, STATE_NEW);
    if entity_state == STATE_NEW {
        return false;
    }
    // If Entity is scheduled for inclusion, it is not in this collection.
    // We can assure that because it would have return true before on
    // array check
    !(entity_state == STATE_MANAGED && uow.is_scheduled_for_insert(entity))
}

/// `CollectionPersister::get()`: gets an element by key.
pub async fn get(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
    index: &Value,
) -> Result<Option<Object>, Thrown> {
    let mapping = collection.get_mapping()?;
    if !mapping.is_indexed() {
        return Err(bad_method_call(NOT_INDEXED));
    }
    let persister = em
        .get_unit_of_work()
        .get_entity_persister(&mapping.target_entity)?;
    let mapped_key = if mapping.is_one_to_many() || !mapping.is_owning_side() {
        mapping.mapped_by.clone()
    } else {
        mapping.inversed_by.clone()
    }
    .unwrap_or_default();
    let mut criteria = PhpArray::new();
    criteria.insert(
        mapped_key.as_str(),
        collection.get_owner().map_or(Value::Null, Value::Object),
    );
    criteria.insert(mapping.index_by()?, index.clone());
    persister
        .load(
            &criteria,
            None,
            Some(&mapping),
            Hints::default(),
            Some(1),
            None,
        )
        .await
}

/// `CollectionPersister::count()`: counts the size of this persistent
/// collection.
pub async fn count(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
) -> Result<i64, Thrown> {
    let mapping = collection.get_mapping()?;
    if mapping.is_one_to_many() {
        let persister = em
            .get_unit_of_work()
            .get_entity_persister(&mapping.target_entity)?;
        // only works with single id identifier entities. Will throw an
        // exception in Entity Persisters if that is not the case for the
        // 'mappedBy' field.
        let mut criteria = Criteria::create();
        criteria.r#where(Criteria::expr().eq(
            mapping.mapped_by.as_deref().unwrap_or_default(),
            collection.get_owner().map_or(Value::Null, Value::Object),
        ));
        return persister.count_criteria(&criteria).await;
    }

    let platform = em.get_platform();
    let quote_strategy = em.get_configuration().get_quote_strategy();
    let id = em
        .get_unit_of_work()
        .get_entity_identifier(&owner_of(collection)?)?;
    let source_class = em.get_class_metadata(&mapping.source_entity)?;
    let association = em.get_metadata_factory().get_owning_side(&mapping)?;
    let mut conditions = Vec::new();
    let mut params = Vec::new();
    let mut types: Vec<ParamType> = Vec::new();
    let sql = {
        let source = source_class.read();
        let join_table_name = quote_strategy.get_join_table_name(&association, &source, platform);
        let join_table = association
            .join_table
            .clone()
            .unwrap_or_else(|| crate::mapping::JoinTableMapping::new(""));
        let join_columns = if mapping.is_owning_side() {
            join_table.join_columns
        } else {
            join_table.inverse_join_columns
        };
        for join_column in &join_columns {
            let column_name = quote_strategy.get_join_column_name(join_column, &source, platform);
            let referenced_name = &join_column.referenced_column_name;
            conditions.push(format!("t.{column_name} = ?"));
            params.push(id_value(
                &id,
                &source.get_field_for_column(referenced_name)?,
            ));
            types.push(
                get_type_of_column(referenced_name, &source, em)?
                    .as_str()
                    .into(),
            );
        }
        let (join_target_entity_sql, filter_sql) = get_filter_sql(em, &mapping)?;

        if !filter_sql.is_empty() {
            conditions.push(filter_sql);
        }

        format!(
            "SELECT COUNT(*) FROM {join_table_name} t{join_target_entity_sql} WHERE {}",
            conditions.join(" AND ")
        )
    };
    let value = em
        .get_connection()
        .fetch_one(&sql, &PhpArray::list(params), &types_of(types))
        .await
        .map_err(|e| dbal(&e))?;
    Ok(to_int(value))
}

/// `CollectionPersister::slice()`: slices elements.
pub async fn slice(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
    offset: i64,
    length: Option<i64>,
) -> Result<PhpArray, Thrown> {
    let mapping = collection.get_mapping()?;
    let persister = em
        .get_unit_of_work()
        .get_entity_persister(&mapping.target_entity)?;
    let owner = owner_of(collection)?;
    if mapping.is_one_to_many() {
        persister
            .get_one_to_many_collection(&mapping, &owner, Some(offset), length)
            .await
    } else {
        persister
            .get_many_to_many_collection(&mapping, &owner, Some(offset), length)
            .await
    }
}

/// `CollectionPersister::containsKey()`: checks for existence of an
/// element by key.
pub async fn contains_key(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
    key: &Key,
) -> Result<bool, Thrown> {
    let mapping = collection.get_mapping()?;
    if !mapping.is_indexed() {
        return Err(bad_method_call(NOT_INDEXED));
    }
    if mapping.is_one_to_many() {
        let persister = em
            .get_unit_of_work()
            .get_entity_persister(&mapping.target_entity)?;
        // only works with single id identifier entities. Will throw an
        // exception in Entity Persisters if that is not the case for the
        // 'mappedBy' field.
        let mut criteria = Criteria::create();
        criteria.and_where(Criteria::expr().eq(
            mapping.mapped_by.as_deref().unwrap_or_default(),
            collection.get_owner().map_or(Value::Null, Value::Object),
        ))?;
        criteria.and_where(Criteria::expr().eq(mapping.index_by()?, key.to_value()))?;
        return Ok(persister.count_criteria(&criteria).await? != 0);
    }

    let (quoted_join_table, where_clauses, params, types) =
        get_join_table_restrictions_with_key(em, collection, &key_text(key))?;
    let sql = format!(
        "SELECT 1 FROM {quoted_join_table} WHERE {}",
        where_clauses.join(" AND ")
    );
    let value = em
        .get_connection()
        .fetch_one(&sql, &PhpArray::list(params), &types_of(types))
        .await
        .map_err(|e| dbal(&e))?;
    Ok(value.is_some_and(|v| v.is_truthy()))
}

/// `CollectionPersister::contains()`: checks for existence of an element.
pub async fn contains(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
    element: &Object,
) -> Result<bool, Thrown> {
    if !is_valid_entity_state(em, element) {
        return Ok(false);
    }
    let mapping = collection.get_mapping()?;
    if mapping.is_one_to_many() {
        let persister = em
            .get_unit_of_work()
            .get_entity_persister(&mapping.target_entity)?;
        // only works with single id identifier entities. Will throw an
        // exception in Entity Persisters if that is not the case for the
        // 'mappedBy' field.
        let mut criteria = Criteria::create();
        criteria.r#where(Criteria::expr().eq(
            mapping.mapped_by.as_deref().unwrap_or_default(),
            collection.get_owner().map_or(Value::Null, Value::Object),
        ));
        return persister.exists(element, Some(&criteria)).await;
    }

    let (quoted_join_table, where_clauses, params, types) =
        get_join_table_restrictions(em, collection, element, true)?;
    let sql = format!(
        "SELECT 1 FROM {quoted_join_table} WHERE {}",
        where_clauses.join(" AND ")
    );
    let value = em
        .get_connection()
        .fetch_one(&sql, &PhpArray::list(params), &types_of(types))
        .await
        .map_err(|e| dbal(&e))?;
    Ok(value.is_some_and(|v| v.is_truthy()))
}

/// The quoted join table, the where clauses, the parameters and their
/// types of `getJoinTableRestrictions*()`.
type Restrictions = (String, Vec<String>, Vec<Value>, Vec<ParamType>);

/// `ManyToManyPersister::getJoinTableRestrictionsWithKey()`.
fn get_join_table_restrictions_with_key(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
    key: &str,
) -> Result<Restrictions, Thrown> {
    let platform = em.get_platform();
    let quote_strategy = em.get_configuration().get_quote_strategy();
    let filter_mapping = collection.get_mapping()?;
    let mut mapping = filter_mapping.clone();
    let index_by = mapping.index_by()?.to_owned();
    let id = em
        .get_unit_of_work()
        .get_entity_identifier(&owner_of(collection)?)?;
    let source_class = em.get_class_metadata(&mapping.source_entity)?;
    let target_class = em.get_class_metadata(&mapping.target_entity)?;

    let (association_source_class, join_columns, source_to_target) = if mapping.is_owning_side() {
        let association_source_class = em.get_class_metadata(&mapping.source_entity)?;
        let join_columns = mapping
            .join_table
            .clone()
            .unwrap_or_else(|| crate::mapping::JoinTableMapping::new(""))
            .inverse_join_columns;
        (association_source_class, join_columns, true)
    } else {
        let association_source_class = em.get_class_metadata(&mapping.target_entity)?;
        mapping = association_source_class
            .read()
            .get_association_mapping(mapping.mapped_by.as_deref().unwrap_or_default())?
            .clone();
        let join_columns = mapping
            .join_table
            .clone()
            .unwrap_or_else(|| crate::mapping::JoinTableMapping::new(""))
            .join_columns;
        (association_source_class, join_columns, false)
    };
    // relationToSourceKeyColumns / relationToTargetKeyColumns
    let (source_relation, target_relation) = if source_to_target {
        (
            &mapping.relation_to_source_key_columns,
            &mapping.relation_to_target_key_columns,
        )
    } else {
        (
            &mapping.relation_to_target_key_columns,
            &mapping.relation_to_source_key_columns,
        )
    };

    let source = source_class.read();
    let target = target_class.read();
    let mut quoted_join_table = format!(
        "{} t",
        quote_strategy.get_join_table_name(&mapping, &association_source_class.read(), platform)
    );
    let mut where_clauses = Vec::new();
    let mut params = Vec::new();
    let mut types: Vec<ParamType> = Vec::new();

    let join_needed = !target.identifier.contains(&index_by);

    if join_needed {
        // extra join needed if indexBy is not a @id
        let join_conditions: Vec<String> = join_columns
            .iter()
            .map(|c| format!("t.{} = tr.{}", c.name, c.referenced_column_name))
            .collect();
        let table_name = quote_strategy.get_table_name(&target, platform);
        quoted_join_table.push_str(" JOIN ");
        quoted_join_table.push_str(&table_name);
        quoted_join_table.push_str(" tr ON ");
        quoted_join_table.push_str(&join_conditions.join(" AND "));
        let column_name = target.get_column_name(&index_by);
        where_clauses.push(format!("tr.{column_name} = ?"));
        params.push(Value::string(key));
        types.push(
            get_type_of_column(&column_name, &target, em)?
                .as_str()
                .into(),
        );
    }

    for join_table_column in &mapping.join_table_columns {
        let column_key = Key::from(join_table_column.as_str());
        if let Some(column) = source_relation.get(&column_key) {
            let column = to_text(column);
            where_clauses.push(format!("t.{join_table_column} = ?"));
            let field = if source.contains_foreign_identifier {
                source.get_field_for_column(&column)?
            } else {
                source.field_names.get(&column).cloned().unwrap_or_default()
            };
            params.push(id_value(&id, &field));
            types.push(get_type_of_column(&column, &source, em)?.as_str().into());
        } else if !join_needed {
            let column = to_text(target_relation.get(&column_key).unwrap_or(&Value::Null));
            where_clauses.push(format!("t.{join_table_column} = ?"));
            params.push(Value::string(key));
            types.push(get_type_of_column(&column, &target, em)?.as_str().into());
        }
    }
    drop((source, target));
    // getJoinTableRestrictionsWithKey() is always called with $addFilters.
    let (join_target_entity_sql, filter_sql) = get_filter_sql(em, &filter_mapping)?;

    if !filter_sql.is_empty() {
        quoted_join_table.push(' ');
        quoted_join_table.push_str(&join_target_entity_sql);
        where_clauses.push(filter_sql);
    }
    Ok((quoted_join_table, where_clauses, params, types))
}

/// `ManyToManyPersister::getJoinTableRestrictions()`.
fn get_join_table_restrictions(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
    element: &Object,
    add_filters: bool,
) -> Result<Restrictions, Thrown> {
    let platform = em.get_platform();
    let quote_strategy = em.get_configuration().get_quote_strategy();
    let uow = em.get_unit_of_work();
    let filter_mapping = collection.get_mapping()?;
    let mapping = filter_mapping.clone();
    let owner = owner_of(collection)?;
    let (source_class, target_class, source_id, target_id) = if mapping.is_owning_side() {
        (
            em.get_class_metadata(&mapping.source_entity)?,
            em.get_class_metadata(&mapping.target_entity)?,
            uow.get_entity_identifier(&owner)?,
            uow.get_entity_identifier(element)?,
        )
    } else {
        (
            em.get_class_metadata(&mapping.target_entity)?,
            em.get_class_metadata(&mapping.source_entity)?,
            uow.get_entity_identifier(element)?,
            uow.get_entity_identifier(&owner)?,
        )
    };
    let mapping = em.get_metadata_factory().get_owning_side(&mapping)?;
    let source = source_class.read();
    let target = target_class.read();

    let mut quoted_join_table = quote_strategy.get_join_table_name(&mapping, &source, platform);
    let mut where_clauses = Vec::new();
    let mut params = Vec::new();
    let mut types: Vec<ParamType> = Vec::new();

    for join_table_column in &mapping.join_table_columns {
        where_clauses.push(format!(
            "{}{join_table_column} = ?",
            if add_filters { "t." } else { "" }
        ));
        let column_key = Key::from(join_table_column.as_str());
        if let Some(target_column) = mapping.relation_to_target_key_columns.get(&column_key) {
            let target_column = to_text(target_column);
            params.push(id_value(
                &target_id,
                &target.get_field_for_column(&target_column)?,
            ));
            types.push(
                get_type_of_column(&target_column, &target, em)?
                    .as_str()
                    .into(),
            );
            continue;
        }
        // relationToSourceKeyColumns
        let target_column = to_text(
            mapping
                .relation_to_source_key_columns
                .get(&column_key)
                .unwrap_or(&Value::Null),
        );
        params.push(id_value(
            &source_id,
            &source.get_field_for_column(&target_column)?,
        ));
        types.push(
            get_type_of_column(&target_column, &source, em)?
                .as_str()
                .into(),
        );
    }

    if add_filters {
        quoted_join_table.push_str(" t");

        drop((source, target));
        let (join_target_entity_sql, filter_sql) = get_filter_sql(em, &filter_mapping)?;

        if !filter_sql.is_empty() {
            quoted_join_table.push(' ');
            quoted_join_table.push_str(&join_target_entity_sql);
            where_clauses.push(filter_sql);
        }
    }
    Ok((quoted_join_table, where_clauses, params, types))
}

/// `ManyToManyPersister::getFilterSql()`: generates the filter SQL for a
/// given mapping.
///
/// This method is not used for actually grabbing the related entities
/// but when the extra-lazy collection methods are called on a filtered
/// association. This is why besides the many to many table we also
/// have to join in the actual entities table leading to additional
/// JOIN.
///
/// Returns the join SQL and the filter SQL (both empty without filter
/// conditions).
pub fn get_filter_sql(
    em: &Arc<EntityManager>,
    mapping: &AssociationMapping,
) -> Result<(String, String), Thrown> {
    let target_class = em.get_class_metadata(&mapping.target_entity)?;
    let root_entity_name = target_class.read().root_entity_name.clone();
    let root_class = em.get_class_metadata(&root_entity_name)?;
    let filter_sql = generate_filter_condition_sql(em, &root_class.read(), "te")?;

    if filter_sql.is_empty() {
        return Ok((String::new(), String::new()));
    }

    // A join is needed if there is filtering on the target entity
    let platform = em.get_platform();
    let table_name = em
        .get_configuration()
        .get_quote_strategy()
        .get_table_name(&root_class.read(), platform);
    let join_sql = format!(
        " JOIN {table_name} te ON{}",
        get_on_condition_sql(em, mapping)?.join(" AND ")
    );

    Ok((join_sql, filter_sql))
}

/// `ManyToManyPersister::generateFilterConditionSQL()`: generates the
/// filter SQL for a given entity and table alias.
fn generate_filter_condition_sql(
    em: &Arc<EntityManager>,
    target_entity: &ClassMetadataInfo,
    target_table_alias: &str,
) -> Result<String, Thrown> {
    let mut filter_clauses = Vec::new();

    for (_, filter) in em.get_filters().get_enabled_filters() {
        let filter_expr = filter.add_filter_constraint(target_entity, target_table_alias)?;
        if !filter_expr.is_empty() {
            filter_clauses.push(format!("({filter_expr})"));
        }
    }

    Ok(if filter_clauses.is_empty() {
        String::new()
    } else {
        format!("({})", filter_clauses.join(" AND "))
    })
}

/// `CollectionPersister::loadCriteria()`: loads association entities
/// matching the given Criteria object.
pub async fn load_criteria(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
    criteria: &Criteria,
) -> Result<Vec<Value>, Thrown> {
    let mapping = collection.get_mapping()?;
    if mapping.is_one_to_many() {
        return Err(bad_method_call(
            "Filtering a collection by Criteria is not supported by this CollectionPersister.",
        ));
    }
    many_to_many_load_criteria(em, collection, &mapping, criteria).await
}

#[allow(clippy::too_many_lines, reason = "upstream's method, in its order")]
async fn many_to_many_load_criteria(
    em: &Arc<EntityManager>,
    collection: &Arc<PersistentCollection>,
    mapping: &AssociationMapping,
    criteria: &Criteria,
) -> Result<Vec<Value>, Thrown> {
    let platform = em.get_platform();
    let quote_strategy = em.get_configuration().get_quote_strategy();
    let owner = owner_of(collection)?;
    let owner_metadata = em.get_class_metadata(owner.class())?;
    let id = em.get_unit_of_work().get_entity_identifier(&owner)?;
    let target_class = em.get_class_metadata(&mapping.target_entity)?;
    let on_conditions = get_on_condition_sql(em, mapping)?;
    let mut where_clauses = Vec::new();
    let mut params = Vec::new();
    let mut param_types: Vec<ParamType> = Vec::new();

    let (association_source_class, source_to_target) = if mapping.is_owning_side() {
        (Arc::clone(&owner_metadata), true)
    } else {
        (Arc::clone(&target_class), false)
    };

    let mapping = em.get_metadata_factory().get_owning_side(mapping)?;
    let source_relation = if source_to_target {
        &mapping.relation_to_source_key_columns
    } else {
        &mapping.relation_to_target_key_columns
    };

    {
        let owner_info = owner_metadata.read();
        for (key, value) in source_relation.iter() {
            let value = to_text(value);
            where_clauses.push(format!("t.{} = ?", key_text(key)));
            let field = if owner_info.contains_foreign_identifier {
                owner_info.get_field_for_column(&value)?
            } else {
                owner_info
                    .field_names
                    .get(&value)
                    .cloned()
                    .unwrap_or_default()
            };
            params.push(id_value(&id, &field));
            param_types.push(get_type_of_column(&value, &owner_info, em)?.as_str().into());
        }
    }

    let parameters = expand_criteria_parameters(criteria)?;

    {
        let target = target_class.read();
        for parameter in parameters {
            let (name, value, operator) = (parameter.field, parameter.value, parameter.operator);
            let field = quote_strategy.get_column_name(&name, &target, platform);

            if value.is_null() && (operator == Comparison::EQ || operator == Comparison::NEQ) {
                where_clauses.push(format!(
                    "te.{field} {} NULL",
                    if operator == Comparison::EQ {
                        "IS"
                    } else {
                        "IS NOT"
                    }
                ));
            } else if operator == Comparison::IN || operator == Comparison::NIN {
                let values: Vec<Value> = match &value {
                    Value::Array(values) => values.values().cloned().collect(),
                    other => {
                        // `count($value)` of a non-countable value.
                        return Err(Thrown::new(
                            "TypeError",
                            format!(
                                "count(): Argument #1 ($value) must be of type Countable|array, {} given",
                                php_type_name(other)
                            ),
                            0,
                        ));
                    }
                };
                where_clauses.push(format!(
                    "te.{field} {} ({})",
                    if operator == Comparison::IN {
                        "IN"
                    } else {
                        "NOT IN"
                    },
                    vec!["?"; values.len()].join(", ")
                ));
                for item in &values {
                    params.extend(convert_to_parameter_value(item, em)?);
                    param_types.extend(infer_parameter_types(&name, item, &target, em)?);
                }
            } else {
                where_clauses.push(format!("te.{field} {operator} ?"));
                params.extend(convert_to_parameter_value(&value, em)?);
                param_types.extend(infer_parameter_types(&name, &value, &target, em)?);
            }
        }
    }

    let (table_name, join_table) = (
        quote_strategy.get_table_name(&target_class.read(), platform),
        quote_strategy.get_join_table_name(&mapping, &association_source_class.read(), platform),
    );

    let target_name = target_class.read().name.clone();
    let mut rsm = ResultSetMappingBuilder::new(em, None);
    rsm.add_root_entity_from_class_metadata(&target_name, "te", &OrderedMap::new(), None)?;

    let mut sql = format!(
        "SELECT {} FROM {table_name} te JOIN {join_table} t ON{} WHERE {}",
        rsm.generate_select_clause(&PhpArray::new())?,
        on_conditions.join(" AND "),
        where_clauses.join(" AND ")
    );

    sql.push_str(&get_ordering_sql(em, criteria, &target_class.read()));
    sql.push_str(&get_limit_sql(em, criteria)?);

    let mut stmt = em
        .get_connection()
        .execute_query(&sql, &PhpArray::list(params), &types_of(param_types))
        .await
        .map_err(|e| dbal(&e))?;
    let rows = stmt.fetch_all_associative().map_err(|e| dbal(&e))?;

    let rsm = rsm.into_result_set_mapping();
    let result = hydration::hydrate(
        em,
        crate::query::context::HYDRATE_OBJECT,
        rows,
        &rsm,
        &mut Hints::default(),
        HydrationHints::default(),
    )
    .await?;
    Ok(match result {
        Value::Array(entities) => entities.values().cloned().collect(),
        _ => Vec::new(),
    })
}

/// `get_debug_type()` of a scalar.
fn php_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// `ManyToManyPersister::getOnConditionSQL()`: generate ON condition.
fn get_on_condition_sql(
    em: &Arc<EntityManager>,
    mapping: &AssociationMapping,
) -> Result<Vec<String>, Thrown> {
    let platform = em.get_platform();
    let quote_strategy = em.get_configuration().get_quote_strategy();
    let association = em.get_metadata_factory().get_owning_side(mapping)?;
    let join_table = association
        .join_table
        .clone()
        .unwrap_or_else(|| crate::mapping::JoinTableMapping::new(""));
    let join_columns = if mapping.is_owning_side() {
        join_table.inverse_join_columns
    } else {
        join_table.join_columns
    };

    let target_class = em.get_class_metadata(&mapping.target_entity)?;
    let target = target_class.read();
    Ok(join_columns
        .iter()
        .map(|join_column| {
            let join_column_name =
                quote_strategy.get_join_column_name(join_column, &target, platform);
            let ref_column_name =
                quote_strategy.get_referenced_join_column_name(join_column, &target, platform);
            format!(" t.{join_column_name} = te.{ref_column_name}")
        })
        .collect())
}

/// `ManyToManyPersister::expandCriteriaParameters()`: expands Criteria
/// Parameters by walking the expressions and grabbing all parameters and
/// types from it.
fn expand_criteria_parameters(criteria: &Criteria) -> Result<Vec<super::ValueType>, Thrown> {
    let Some(expression) = criteria.get_where_expression() else {
        return Ok(Vec::new());
    };
    let mut value_visitor = SqlValueVisitor::new();
    value_visitor.dispatch(expression)?;
    let (_, types) = value_visitor.get_params_and_types();
    Ok(types)
}

/// `ManyToManyPersister::getOrderingSql()`.
fn get_ordering_sql(
    em: &EntityManager,
    criteria: &Criteria,
    target_class: &ClassMetadataInfo,
) -> String {
    let orderings = criteria.get_orderings();
    if orderings.is_empty() {
        return String::new();
    }
    let platform = em.get_platform();
    let quote_strategy = em.get_configuration().get_quote_strategy();
    let order_by: Vec<String> = orderings
        .iter()
        .map(|(name, direction)| {
            let field = quote_strategy.get_column_name(name, target_class, platform);
            let direction_string = if *direction == SortDirection::Ascending {
                "ASC"
            } else {
                "DESC"
            };
            format!("{field} {direction_string}")
        })
        .collect();
    format!(" ORDER BY {}", order_by.join(", "))
}

/// `ManyToManyPersister::getLimitSql()`.
fn get_limit_sql(em: &EntityManager, criteria: &Criteria) -> Result<String, Thrown> {
    let limit = criteria.get_max_results();
    let offset = criteria.get_first_result();
    em.get_platform()
        .modify_limit_query("", limit, offset.unwrap_or(0))
        .map_err(|e| {
            Thrown::new(
                "Doctrine\\DBAL\\Exception\\InvalidArgumentException",
                e.message(),
                0,
            )
        })
}
