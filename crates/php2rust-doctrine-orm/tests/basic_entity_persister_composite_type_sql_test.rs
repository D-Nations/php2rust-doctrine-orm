//! Port of `BasicEntityPersisterCompositeTypeSqlTest`. The entity manager
//! is a `SQLite` one (`tests/common/orm.rs`) in place of
//! `EntityManagerMock`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Persisters/BasicEntityPersisterCompositeTypeSqlTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php_value::{PhpArray, Value};
use php2rust_doctrine_collections::expr::Comparison;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::exception::CANT_USE_IN_OPERATOR_ON_COMPOSITE_KEYS;
use php2rust_doctrine_orm::mapping::AssociationMapping;
use php2rust_doctrine_orm::mapping::association_mapping::{AssociationClass, AssociationKind};
use php2rust_doctrine_orm::persisters::BasicEntityPersister;

const ADMIN1_ALTERNATE_NAME: &str = "Doctrine\\Tests\\Models\\GeoNames\\Admin1AlternateName";

struct Test {
    em: Arc<EntityManager>,
    persister: BasicEntityPersister,
    association_mapping: AssociationMapping,
}

fn set_up() -> Test {
    let em = orm::entity_manager(orm::connection(), None);
    let persister =
        BasicEntityPersister::new(&em, em.get_class_metadata(ADMIN1_ALTERNATE_NAME).unwrap());
    let association_mapping = AssociationMapping::new(
        AssociationClass::Final(AssociationKind::ManyToOne),
        "admin1",
        "Doctrine\\Tests\\ORM\\Persisters\\WhoCares",
        ADMIN1_ALTERNATE_NAME,
    );
    Test {
        em,
        persister,
        association_mapping,
    }
}

impl Test {
    fn statement(&self, value: &Value, comparison: &str) -> Result<String, php_value::Thrown> {
        self.persister.get_select_condition_statement_sql(
            &self.em,
            "admin1",
            value,
            Some(&self.association_mapping),
            Some(comparison),
        )
    }
}

#[test]
fn testSelectConditionStatementEq() {
    let t = set_up();
    let statement = t.statement(&Value::Int(1), Comparison::EQ).unwrap();
    assert_eq!("t0.admin1 = ? AND t0.country = ?", statement);
}

#[test]
fn testSelectConditionStatementEqNull() {
    let t = set_up();
    let statement = t.statement(&Value::Null, Comparison::IS).unwrap();
    assert_eq!("t0.admin1 IS NULL AND t0.country IS NULL", statement);
}

#[test]
fn testSelectConditionStatementNeqNull() {
    let t = set_up();
    let statement = t.statement(&Value::Null, Comparison::NEQ).unwrap();
    assert_eq!(
        "t0.admin1 IS NOT NULL AND t0.country IS NOT NULL",
        statement
    );
}

#[test]
fn testSelectConditionStatementIn() {
    let t = set_up();
    let error = t
        .statement(&Value::Array(PhpArray::new()), Comparison::IN)
        .unwrap_err();
    assert_eq!(error.class, CANT_USE_IN_OPERATOR_ON_COMPOSITE_KEYS);
}
