//! The collection persisters: `OneToManyPersister` (the deletion of the
//! elements of an orphan-removing collection) and `ManyToManyPersister`
//! (the rows of the join table). The extra-lazy operations (`count()`,
//! `slice()`, `contains()`, `get()`, ...) and `loadCriteria()` are not
//! ported; joined-inheritance targets are refused (`NotSupported`).
//!
//! Upstream: doctrine/orm src/Persisters/Collection/OneToManyPersister.php src/Persisters/Collection/ManyToManyPersister.php
//! Upstream: doctrine/orm src/Persisters/Collection/AbstractCollectionPersister.php src/Persisters/Collection/CollectionPersister.php

use std::sync::Arc;

use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::ParameterTypes;

use crate::entity_manager::EntityManager;
use crate::exception;
use crate::mapping::AssociationMapping;
use crate::persistent_collection::PersistentCollection;
use crate::utility::{ParamType, get_type_of_column, key_text, to_text};

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
pub async fn delete(em: &Arc<EntityManager>, collection: &Arc<PersistentCollection>) -> Result<(), Thrown> {
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
pub async fn update(em: &Arc<EntityManager>, collection: &Arc<PersistentCollection>) -> Result<(), Thrown> {
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
    let identifier = em.get_unit_of_work().get_entity_identifier(&owner_of(collection)?)?;
    let source_class = em.get_class_metadata(&mapping.source_entity)?;
    let owning = em.get_metadata_factory().get_owning_side(mapping)?;
    let mut columns = Vec::new();
    let mut parameters = Vec::new();
    let mut types: Vec<ParamType> = Vec::new();
    for join_column in &owning.join_columns {
        columns.push(quote_strategy.get_join_column_name(join_column, &target_class.read(), platform));
        let field = source_class.read().get_field_for_column(&join_column.referenced_column_name)?;
        parameters.push(identifier.get(&field.as_str().into()).cloned().unwrap_or(Value::Null));
        types.push(
            get_type_of_column(&join_column.referenced_column_name, &source_class.read(), em)?
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
            statement.push_str(&format!(
                " AND {} IN ({})",
                discriminator_column.name,
                vec!["?"; discriminator_values.len()].join(", ")
            ));
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
    let join_table = mapping.join_table.clone().unwrap_or_else(|| crate::mapping::JoinTableMapping::new(""));
    let mut types: Vec<ParamType> = Vec::new();
    for join_column in &join_table.join_columns {
        types.push(get_type_of_column(&join_column.referenced_column_name, &class.read(), em)?.as_str().into());
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
        let sql = format!("DELETE FROM {join_table_name} WHERE {} = ?", columns.join(" = ? AND "));

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
                params.push(identifier.get(&field.as_str().into()).cloned().unwrap_or(Value::Null));
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
    let join_table = mapping.join_table.clone().unwrap_or_else(|| crate::mapping::JoinTableMapping::new(""));

    // getDeleteRowSQL()
    let (delete_sql, delete_types, insert_sql, insert_types) = {
        let source = class.read();
        let target = target_class.read();
        let join_table_name = quote_strategy.get_join_table_name(mapping, &source, platform);
        let mut delete_columns = Vec::new();
        let mut delete_types: Vec<ParamType> = Vec::new();
        for join_column in &join_table.join_columns {
            delete_columns.push(quote_strategy.get_join_column_name(join_column, &source, platform));
            delete_types.push(get_type_of_column(&join_column.referenced_column_name, &source, em)?.as_str().into());
        }
        for join_column in &join_table.inverse_join_columns {
            delete_columns.push(quote_strategy.get_join_column_name(join_column, &target, platform));
            delete_types.push(get_type_of_column(&join_column.referenced_column_name, &target, em)?.as_str().into());
        }
        let delete_sql = format!(
            "DELETE FROM {join_table_name} WHERE {} = ?",
            delete_columns.join(" = ? AND ")
        );

        // getInsertRowSQL()
        let mut insert_columns = Vec::new();
        let mut insert_types: Vec<ParamType> = Vec::new();
        for join_column in &join_table.join_columns {
            insert_columns.push(quote_strategy.get_join_column_name(join_column, &target, platform));
            insert_types.push(get_type_of_column(&join_column.referenced_column_name, &source, em)?.as_str().into());
        }
        for join_column in &join_table.inverse_join_columns {
            insert_columns.push(quote_strategy.get_join_column_name(join_column, &target, platform));
            insert_types.push(get_type_of_column(&join_column.referenced_column_name, &target, em)?.as_str().into());
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
        let Value::Object(element) = element else { continue };
        let params = collect_join_table_column_parameters(em, collection, mapping, &element)?;
        conn.execute_statement(&delete_sql, &PhpArray::list(params), &types_of(delete_types.clone()))
            .await
            .map_err(|e| exception::from_dbal(&e))?;
    }
    for element in collection.get_insert_diff() {
        let Value::Object(element) = element else { continue };
        let params = collect_join_table_column_parameters(em, collection, mapping, &element)?;
        conn.execute_statement(&insert_sql, &PhpArray::list(params), &types_of(insert_types.clone()))
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
            params.push(if is_relation_to_source { ids1.pop() } else { ids2.pop() }.unwrap_or(Value::Null));
            continue;
        }
        if is_relation_to_source {
            let column = to_text(mapping.relation_to_source_key_columns.get(&key).unwrap_or(&Value::Null));
            let field = class1.read().get_field_for_column(&column)?;
            params.push(identifier1.get(&field.as_str().into()).cloned().unwrap_or(Value::Null));
            continue;
        }
        let column = to_text(mapping.relation_to_target_key_columns.get(&key).unwrap_or(&Value::Null));
        let field = match &class2 {
            Some(class2) => class2.read().get_field_for_column(&column)?,
            None => key_text(&key),
        };
        params.push(identifier2.get(&field.as_str().into()).cloned().unwrap_or(Value::Null));
    }
    Ok(params)
}
