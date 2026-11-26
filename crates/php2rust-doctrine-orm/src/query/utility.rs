//! The utilities the SQL walker and the executors use:
//! `PersisterHelper::getTypeOfColumn()`,
//! `HierarchyDiscriminatorResolver::resolveDiscriminatorsForClass()`,
//! the `LockSqlHelper` trait and `JoinedSubclassPersister::getOwningTable()`
//! (which the walker reaches through the unit of work's entity persister).
//! `get_type_of_column()` is `crate::utility::persister_helper`'s over a
//! [`QueryContext`] instead of the `EntityManager`.
//!
//! Upstream: doctrine/orm src/Utility/HierarchyDiscriminatorResolver.php src/Utility/LockSqlHelper.php

use std::sync::Arc;

use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;

use crate::mapping::ClassMetadata;
use crate::mapping::quote_strategy::platform_is_a;
use crate::query::context::QueryContext;

/// `PersisterHelper::getTypeOfColumn($columnName, $class, $em)`.
pub(crate) fn get_type_of_column(
    column_name: &str,
    class: &Arc<ClassMetadata>,
    em: &dyn QueryContext,
) -> Result<String, Thrown> {
    let (target, class_name) = {
        let info = class.read();
        if let Some(field_name) = info.field_names.get(column_name)
            && let Some(mapping) = info.field_mappings.get(field_name)
        {
            return Ok(mapping.r#type.clone());
        }

        // iterate over to-one association mappings
        let to_one = info
            .association_mappings
            .values()
            .filter(|assoc| assoc.is_to_one_owning_side())
            .find_map(|assoc| {
                assoc
                    .join_columns
                    .iter()
                    .find(|join_column| join_column.name == column_name)
                    .map(|join_column| {
                        (
                            join_column.referenced_column_name.clone(),
                            assoc.target_entity.clone(),
                        )
                    })
            });

        // iterate over to-many association mappings
        let target = to_one.or_else(|| {
            info.association_mappings
                .values()
                .filter(|assoc| assoc.is_many_to_many_owning_side())
                .find_map(|assoc| {
                    assoc.join_table.as_ref().and_then(|join_table| {
                        join_table
                            .join_columns
                            .iter()
                            .find(|join_column| join_column.name == column_name)
                            .map(|join_column| {
                                (
                                    join_column.referenced_column_name.clone(),
                                    assoc.target_entity.clone(),
                                )
                            })
                    })
                })
        });
        (target, info.name.clone())
    };

    if let Some((target_column_name, target_entity)) = target {
        let target_class = em.get_class_metadata(&target_entity)?;
        return get_type_of_column(&target_column_name, &target_class, em);
    }

    Err(Thrown::new(
        "RuntimeException",
        format!("Could not resolve type of column \"{column_name}\" of class \"{class_name}\""),
        0,
    ))
}

/// `HierarchyDiscriminatorResolver::resolveDiscriminatorsForClass()`:
/// the discriminator values of the class and its subclasses, as the keys
/// of an array (the values are `null`).
pub(crate) fn resolve_discriminators_for_class(
    root_class_metadata: &ClassMetadata,
    entity_manager: &dyn QueryContext,
) -> Result<PhpArray, Thrown> {
    let mut hierarchy_classes = root_class_metadata.read().sub_classes.clone();
    hierarchy_classes.push(root_class_metadata.read().name.clone());
    let mut discriminators = PhpArray::new();
    for class in hierarchy_classes {
        let current_metadata = entity_manager.get_class_metadata(&class)?;
        let current_discriminator = current_metadata.read().discriminator_value.clone();
        if let Some(key) = discriminator_key(&current_discriminator) {
            discriminators.insert(key, Value::Null);
        }
    }
    Ok(discriminators)
}

/// `$discriminators[$value]` for a non-null discriminator value.
fn discriminator_key(value: &Value) -> Option<Key> {
    match value {
        Value::Null => None,
        other => Key::from_value(other),
    }
}

const ABSTRACT_MYSQL_PLATFORM: &str = "Doctrine\\DBAL\\Platforms\\AbstractMySQLPlatform";
const POSTGRESQL_PLATFORM: &str = "Doctrine\\DBAL\\Platforms\\PostgreSQLPlatform";
const DB2_PLATFORM: &str = "Doctrine\\DBAL\\Platforms\\DB2Platform";
const SQLITE_PLATFORM: &str = "Doctrine\\DBAL\\Platforms\\SQLitePlatform";
const SQLSERVER_PLATFORM: &str = "Doctrine\\DBAL\\Platforms\\SQLServerPlatform";

/// `LockSqlHelper::getReadLockSQL()`.
pub(crate) fn get_read_lock_sql(platform: &dyn AbstractPlatform) -> &'static str {
    if platform_is_a(platform, ABSTRACT_MYSQL_PLATFORM) {
        return "LOCK IN SHARE MODE";
    }
    if platform_is_a(platform, POSTGRESQL_PLATFORM) {
        return "FOR SHARE";
    }
    get_write_lock_sql(platform)
}

/// `LockSqlHelper::getWriteLockSQL()`.
pub(crate) fn get_write_lock_sql(platform: &dyn AbstractPlatform) -> &'static str {
    if platform_is_a(platform, DB2_PLATFORM) {
        return "WITH RR USE AND KEEP UPDATE LOCKS";
    }
    if platform_is_a(platform, SQLITE_PLATFORM) || platform_is_a(platform, SQLSERVER_PLATFORM) {
        return "";
    }
    "FOR UPDATE"
}

/// `JoinedSubclassPersister::getOwningTable($fieldName)`: the table of the
/// class that declares the field or association.
pub(crate) fn get_owning_table(
    class: &ClassMetadata,
    field_name: &str,
    em: &dyn QueryContext,
) -> Result<String, Thrown> {
    let inherited = {
        let info = class.read();
        info.association_mappings
            .get(field_name)
            .and_then(|a| a.inherited.clone())
            .or_else(|| {
                info.field_mappings
                    .get(field_name)
                    .and_then(|f| f.inherited.clone())
            })
    };
    match inherited {
        Some(inherited) => em.get_class_metadata(&inherited)?.read().get_table_name(),
        None => class.read().get_table_name(),
    }
}
