//! Port of tests/Tests/ORM/Persisters/BasicEntityPersisterCompositeTypeParametersTest.php.
//! `getTestEntityManager()` (`EntityManagerMock`) is an entity manager on
//! an in-memory `SQLite` database; the persister takes it with each call.
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

use php_value::{Object, Value, php_array};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_dbal::WrapperParameterType;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persisters::BasicEntityPersister;

use orm::{new_object, s, set, set_obj};

const COUNTRY: &str = "Doctrine\\Tests\\Models\\GeoNames\\Country";
const ADMIN1: &str = "Doctrine\\Tests\\Models\\GeoNames\\Admin1";
const ADMIN1_ALTERNATE_NAME: &str = "Doctrine\\Tests\\Models\\GeoNames\\Admin1AlternateName";

struct Test {
    persister: BasicEntityPersister,
    entity_manager: Arc<EntityManager>,
}

fn set_up() -> Test {
    let entity_manager = orm::entity_manager(orm::connection(), None);

    entity_manager.get_class_metadata(COUNTRY).unwrap();
    entity_manager.get_class_metadata(ADMIN1).unwrap();
    entity_manager
        .get_class_metadata(ADMIN1_ALTERNATE_NAME)
        .unwrap();

    let persister = BasicEntityPersister::new(
        &entity_manager,
        entity_manager
            .get_class_metadata(ADMIN1_ALTERNATE_NAME)
            .unwrap(),
    );
    Test {
        persister,
        entity_manager,
    }
}

/// `new Country('IT', 'Italy')` and `new Admin1(10, 'Rome', $country)`.
fn admin1() -> Object {
    let country = new_object(COUNTRY);
    set(&country, "id", s("IT"));
    set(&country, "name", s("Italy"));
    let admin1 = new_object(ADMIN1);
    set(&admin1, "id", Value::Int(10));
    set(&admin1, "name", s("Rome"));
    set_obj(&admin1, "country", &country);
    admin1
}

fn expected_types() -> Vec<WrapperParameterType> {
    vec![
        WrapperParameterType::Name("integer".into()),
        WrapperParameterType::Name("string".into()),
    ]
}

#[test]
fn testExpandParametersWillExpandCompositeEntityKeys() {
    let t = set_up();
    let admin1 = admin1();

    let (values, types) = t
        .persister
        .expand_parameters(
            &t.entity_manager,
            &php_array!["admin1" => Value::Object(admin1)],
        )
        .unwrap();

    assert_eq!(types, expected_types());
    assert_eq!(values, vec![Value::Int(10), s("IT")]);
}

#[test]
fn testExpandCriteriaParametersWillExpandCompositeEntityKeys() {
    let t = set_up();
    let admin1 = admin1();

    // `Criteria::ASC` is not defined (doctrine/collections 3).
    let mut criteria = Criteria::create();
    criteria
        .and_where(Criteria::expr().eq("admin1", Value::Object(admin1)))
        .unwrap();

    let (values, types) = t
        .persister
        .expand_criteria_parameters(&t.entity_manager, &criteria)
        .unwrap();

    assert_eq!(types, expected_types());
    assert_eq!(values, vec![Value::Int(10), s("IT")]);
}
