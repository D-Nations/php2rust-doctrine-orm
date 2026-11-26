//! `EntityPersister`: the interface of the entity persisters, as far as
//! the criteria paths (`SqlExpressionVisitor`, `LazyCriteriaCollection`)
//! use it. `BasicEntityPersister` is its implementation; the other
//! methods of the interface are the persister's own (the interface's
//! file is listed with `BasicEntityPersister`).
//!
//! Upstream: none

use std::fmt::Debug;

use php_value::{Object, Thrown, Value};
use php2rust_doctrine_collections::Criteria;

use crate::mapping::AssociationMapping;
use crate::unit_of_work::Fut;

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
