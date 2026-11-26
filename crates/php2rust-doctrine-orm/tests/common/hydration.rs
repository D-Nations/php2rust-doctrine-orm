//! `HydrationTestCase`: an entity manager (no schema: the hydrators read
//! the faked result set, not the database) and the faked rows.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Hydration/HydrationTestCase.php tests/Tests/Mocks/ArrayResultFactory.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]
#![allow(clippy::unwrap_used, reason = "test helpers; a panic is a test failure")]

use std::sync::Arc;

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::internal::hydration::{self, HydrationHints};
use php2rust_doctrine_orm::query::ResultSetMapping;
use php2rust_doctrine_orm::unit_of_work::Hints;

use super::orm;

/// `getTestEntityManager()`.
pub fn entity_manager() -> Arc<EntityManager> {
    orm::entity_manager(orm::connection(), None)
}

/// A faked result set: the rows as column/value pairs.
pub fn rows(rows: &[&[(&str, Value)]]) -> Vec<PhpArray> {
    rows.iter()
        .map(|row| {
            let mut array = PhpArray::new();
            for (column, value) in *row {
                array.insert(*column, value.clone());
            }
            array
        })
        .collect()
}

/// `$hydrator->hydrateAll($stmt, $rsm)` of the hydrator of a mode.
pub async fn hydrate_all(
    em: &Arc<EntityManager>,
    mode: i64,
    rows: Vec<PhpArray>,
    rsm: &ResultSetMapping,
) -> Result<Value, Thrown> {
    let mut hints = Hints::default();
    hydration::hydrate(em, mode, rows, rsm, &mut hints, HydrationHints::default()).await
}

/// `$hydrator->hydrateAll($stmt, $rsm, $hints)` with unit of work hints.
pub async fn hydrate_all_with(
    em: &Arc<EntityManager>,
    mode: i64,
    rows: Vec<PhpArray>,
    rsm: &ResultSetMapping,
    mut hints: Hints,
) -> Result<Value, Thrown> {
    hydration::hydrate(em, mode, rows, rsm, &mut hints, HydrationHints::default()).await
}

/// `$hydrator->toIterable($stmt, $rsm)`.
pub async fn to_iterable(
    em: &Arc<EntityManager>,
    mode: i64,
    rows: Vec<PhpArray>,
    rsm: &ResultSetMapping,
) -> Result<Vec<Value>, Thrown> {
    let mut hints = Hints::default();
    hydration::to_iterable(em, mode, rows, rsm, &mut hints, HydrationHints::default()).await
}
