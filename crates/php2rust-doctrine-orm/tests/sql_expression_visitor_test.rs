//! SqlExpressionVisitorTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! PHPUnit's mock of `BasicEntityPersister` is an `EntityPersister` that
//! counts its `getSelectConditionStatementSQL()` calls.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/SqlExpressionVisitorTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use std::sync::atomic::{AtomicUsize, Ordering};

use php_value::{Object, Thrown, Value};
use php2rust_doctrine_collections::{Criteria, ExpressionBuilder as CriteriaBuilder};
use php2rust_doctrine_orm::mapping::{AssociationMapping, ClassMetadata};
use php2rust_doctrine_orm::persisters::{EntityPersister, SqlExpressionVisitor};
use php2rust_doctrine_orm::unit_of_work::Fut;

use php2rust_doctrine_collections::expr::ExpressionVisitor;

#[derive(Debug, Default)]
struct PersisterMock {
    select_condition_statement_sql_calls: AtomicUsize,
}

impl EntityPersister for PersisterMock {
    fn count_criteria<'a>(&'a self, _criteria: &'a Criteria) -> Fut<'a, i64> {
        Box::pin(async { Ok(0) })
    }

    fn load_criteria<'a>(&'a self, _criteria: &'a Criteria) -> Fut<'a, Vec<Value>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn exists<'a>(&'a self, _entity: &'a Object, _extra: Option<&'a Criteria>) -> Fut<'a, bool> {
        Box::pin(async { Ok(false) })
    }

    fn get_select_condition_statement_sql(
        &self,
        _field: &str,
        _value: &Value,
        _assoc: Option<&AssociationMapping>,
        _comparison: Option<&str>,
    ) -> Result<String, Thrown> {
        self.select_condition_statement_sql_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok("dummy expression".to_owned())
    }
}

#[test]
fn testWalkNotCompositeExpression() {
    let persister = PersisterMock::default();
    let class_metadata = ClassMetadata::new("Dummy", None, None);
    let mut visitor = SqlExpressionVisitor::new(&persister, &class_metadata);

    let cb = CriteriaBuilder::new();

    let expr = visitor
        .walk_composite_expression(&cb.not(cb.eq("foo", 1)).unwrap())
        .unwrap();

    assert_eq!(
        persister
            .select_condition_statement_sql_calls
            .load(Ordering::SeqCst),
        1
    );
    assert_eq!(expr, "NOT (dummy expression)");
}
