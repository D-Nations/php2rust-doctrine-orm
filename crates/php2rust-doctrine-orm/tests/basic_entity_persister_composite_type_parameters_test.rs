//! Port of `BasicEntityPersisterCompositeTypeParametersTest`. The entity
//! manager is a `SQLite` one (`tests/common/orm.rs`) in place of
//! `EntityManagerMock`; the types are compared by their names.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Persisters/BasicEntityPersisterCompositeTypeParametersTest.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_dbal::WrapperParameterType;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persisters::BasicEntityPersister;

use orm::{new_object, s, set};

const NS: &str = "Doctrine\\Tests\\Models\\GeoNames\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

fn set_up() -> (Arc<EntityManager>, BasicEntityPersister) {
    let em = orm::entity_manager(orm::connection(), None);

    em.get_class_metadata(&class("Country")).unwrap();
    em.get_class_metadata(&class("Admin1")).unwrap();
    em.get_class_metadata(&class("Admin1AlternateName"))
        .unwrap();

    let persister = BasicEntityPersister::new(
        &em,
        em.get_class_metadata(&class("Admin1AlternateName"))
            .unwrap(),
    );
    (em, persister)
}

/// `new Admin1(10, 'Rome', new Country('IT', 'Italy'))`.
fn rome() -> Object {
    let country = new_object(&class("Country"));
    set(&country, "id", s("IT"));
    set(&country, "name", s("Italy"));
    let admin1 = new_object(&class("Admin1"));
    set(&admin1, "id", Value::Int(10));
    set(&admin1, "name", s("Rome"));
    set(&admin1, "country", Value::Object(country));
    admin1
}

#[test]
fn testExpandParametersWillExpandCompositeEntityKeys() {
    let (em, persister) = set_up();
    let admin1 = rome();

    let mut criteria = PhpArray::new();
    criteria.insert("admin1", Value::Object(admin1));
    let (values, types) = persister.expand_parameters(&em, &criteria).unwrap();

    assert_eq!(
        vec![
            WrapperParameterType::from("integer"),
            WrapperParameterType::from("string")
        ],
        types
    );
    assert_eq!(vec![Value::Int(10), s("IT")], values);
}

#[test]
fn testExpandCriteriaParametersWillExpandCompositeEntityKeys() {
    let (em, persister) = set_up();
    let admin1 = rome();

    let mut criteria = Criteria::create();
    criteria
        .and_where(Criteria::expr().eq("admin1", Value::Object(admin1)))
        .unwrap();

    let (values, types) = persister
        .expand_criteria_parameters(&em, &criteria)
        .unwrap();

    assert_eq!(
        vec![
            WrapperParameterType::from("integer"),
            WrapperParameterType::from("string")
        ],
        types
    );
    assert_eq!(vec![Value::Int(10), s("IT")], values);
}
