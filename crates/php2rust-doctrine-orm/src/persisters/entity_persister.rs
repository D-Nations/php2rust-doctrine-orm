//! `Doctrine\ORM\Persisters\Entity\EntityPersister`, in three parts:
//! `EntityPersister`, the methods the criteria paths
//! (`SqlExpressionVisitor`, `LazyCriteriaCollection`) use;
//! `CommitPersister`, the methods the unit of work's commit and
//! `getEntityState()` call, which `UnitOfWork::set_entity_persister()`
//! replaces for a class; and `EntityPersisterInterface`, the whole
//! interface over both, which the second-level cache's persisters wrap.
//! `BasicEntityPersister` implements all three.
//!
//! `exists()` is in both `EntityPersister` and `CommitPersister`; on an
//! `EntityPersisterInterface` it is called as `CommitPersister::exists()`.
//!
//! Upstream: doctrine/orm src/Persisters/Entity/EntityPersister.php

use std::fmt::Debug;
use std::sync::Arc;

use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_dbal::LockMode;

use crate::mapping::{AssociationMapping, ClassMetadata};
use crate::persistent_collection::PersistentCollection;
use crate::query::ResultSetMapping;
use crate::unit_of_work::{Fut, Hints};

use super::{Params, PersisterCriteria};

/// The criteria methods of `Doctrine\ORM\Persisters\Entity\EntityPersister`.
pub trait EntityPersister: Debug + Send + Sync {
    /// `count(Criteria)`: count entities filtered by a criteria.
    fn count_criteria<'a>(&'a self, criteria: &'a Criteria) -> Fut<'a, i64>;

    /// `loadCriteria($criteria)`: loads Entities matching the given
    /// Criteria object.
    fn load_criteria<'a>(&'a self, criteria: &'a Criteria) -> Fut<'a, Vec<Value>>;

    /// `exists($entity, $extraConditions)`: checks whether the given
    /// managed entity exists in the database.
    fn exists<'a>(
        &'a self,
        entity: &'a Object,
        extra_conditions: Option<&'a Criteria>,
    ) -> Fut<'a, bool>;

    /// `getSelectConditionStatementSQL()`: the conditional SQL fragment
    /// used in the WHERE clause when selecting entities in this persister.
    fn get_select_condition_statement_sql(
        &self,
        field: &str,
        value: &Value,
        assoc: Option<&AssociationMapping>,
        comparison: Option<&str>,
    ) -> Result<String, Thrown>;
}

/// The write methods of `Doctrine\ORM\Persisters\Entity\EntityPersister`
/// that the unit of work calls: `addInsert()`, `executeInserts()`,
/// `update()`, `delete()` and `exists()`.
pub trait CommitPersister: Debug + Send + Sync {
    /// `addInsert($entity)`: adds an entity to the queued insertions.
    fn add_insert(&self, entity: &Object);

    /// `executeInserts()`: executes all queued entity insertions.
    fn execute_inserts(&self) -> Fut<'_, ()>;

    /// `update($entity)`: updates a managed entity.
    fn update<'a>(&'a self, entity: &'a Object) -> Fut<'a, ()>;

    /// `delete($entity)`: deletes a managed entity.
    fn delete<'a>(&'a self, entity: &'a Object) -> Fut<'a, bool>;

    /// `exists($entity, $extraConditions)`.
    fn exists<'a>(
        &'a self,
        entity: &'a Object,
        extra_conditions: Option<&'a Criteria>,
    ) -> Fut<'a, bool>;
}

/// `Doctrine\ORM\Persisters\Entity\EntityPersister`: the whole
/// interface of an entity persister.
pub trait EntityPersisterInterface: EntityPersister + CommitPersister {
    fn get_class_metadata(&self) -> Arc<ClassMetadata>;

    /// Gets the `ResultSetMapping` used for hydration.
    fn get_result_set_mapping(&self) -> ResultSetMapping;

    /// Get all queued inserts.
    fn get_inserts(&self) -> Vec<Object>;

    /// Gets the SELECT SQL to select one or more entities by a set of
    /// field criteria.
    #[allow(
        clippy::too_many_arguments,
        reason = "getSelectSQL(), as PHP declares it"
    )]
    fn get_select_sql(
        &self,
        criteria: PersisterCriteria<'_>,
        assoc: Option<&AssociationMapping>,
        lock_mode: Option<LockMode>,
        limit: Option<i64>,
        offset: Option<i64>,
        order_by: Option<&PhpArray>,
    ) -> Result<String, Thrown>;

    /// Get the COUNT SQL to count entities (optionally filtered by a
    /// criteria).
    fn get_count_sql(&self, criteria: PersisterCriteria<'_>) -> Result<String, Thrown>;

    /// Gets the INSERT SQL used by the persister to persist a new entity.
    fn get_insert_sql(&self) -> Result<String, Thrown>;

    /// Expands the parameters from the given criteria and use the correct
    /// binding types if found.
    fn expand_parameters(&self, criteria: &PhpArray) -> Result<Params, Thrown>;

    /// Expands Criteria Parameters by walking the expressions and grabbing
    /// all parameters and types from it.
    fn expand_criteria_parameters(&self, criteria: &Criteria) -> Result<Params, Thrown>;

    /// Gets the name of the table that owns the column the given field is
    /// mapped to.
    fn get_owning_table(&self, field_name: &str) -> Result<String, Thrown>;

    /// Loads an entity by a list of field criteria.
    #[allow(clippy::too_many_arguments, reason = "load(), as PHP declares it")]
    fn load<'a>(
        &'a self,
        criteria: &'a PhpArray,
        entity: Option<&'a Object>,
        assoc: Option<&'a AssociationMapping>,
        hints: Hints,
        lock_mode: Option<LockMode>,
        limit: Option<i64>,
        order_by: Option<&'a PhpArray>,
    ) -> Fut<'a, Option<Object>>;

    /// Loads an entity by identifier.
    fn load_by_id<'a>(
        &'a self,
        identifier: &'a PhpArray,
        entity: Option<&'a Object>,
    ) -> Fut<'a, Option<Object>>;

    /// Loads a list of entities by a list of field criteria.
    fn load_all<'a>(
        &'a self,
        criteria: &'a PhpArray,
        order_by: Option<&'a PhpArray>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Fut<'a, Vec<Object>>;

    /// Loads an entity of this persister's mapped class as part of a
    /// single-valued association from another entity.
    fn load_one_to_one_entity<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        identifier: Option<&'a PhpArray>,
    ) -> Fut<'a, Option<Object>>;

    /// Refreshes a managed entity.
    fn refresh<'a>(
        &'a self,
        id: &'a PhpArray,
        entity: &'a Object,
        lock_mode: Option<LockMode>,
    ) -> Fut<'a, ()>;

    /// Count entities (optionally filtered by a criteria).
    fn count<'a>(&'a self, criteria: &'a PhpArray) -> Fut<'a, i64>;

    /// Locks all rows of this entity matching the given criteria with the
    /// specified pessimistic lock mode.
    fn lock<'a>(&'a self, criteria: &'a PhpArray, lock_mode: LockMode) -> Fut<'a, ()>;

    /// Returns an array with (sliced or full list) of elements in the
    /// specified collection.
    fn get_many_to_many_collection<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> Fut<'a, PhpArray>;

    /// Loads a collection of entities of a many-to-many association.
    fn load_many_to_many_collection<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        collection: &'a Arc<PersistentCollection>,
    ) -> Fut<'a, Vec<Object>>;

    /// Returns an array with (sliced or full list) of elements in the
    /// specified collection.
    fn get_one_to_many_collection<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> Fut<'a, PhpArray>;

    /// Loads a collection of entities in a one-to-many association.
    fn load_one_to_many_collection<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        collection: &'a Arc<PersistentCollection>,
    ) -> Fut<'a, Vec<Object>>;
}
