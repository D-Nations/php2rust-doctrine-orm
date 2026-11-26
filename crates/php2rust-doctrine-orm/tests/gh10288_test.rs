//! Ticket/GH10288Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models and `Models/GH10288`; `GH10288PeopleType` is a Rust type).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10288Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use std::sync::Arc;

use php_value::{Object, PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type};
use php2rust_doctrine_orm::abstract_query::{HYDRATE_ARRAY, HYDRATE_OBJECT};

use orm::*;
use ticket::*;

const GH10288_PEOPLE: &str = "Doctrine\\Tests\\Models\\GH10288\\GH10288People";

/// `GH10288People::from($value)`.
fn people_from(value: &Value) -> Value {
    models::register();
    let class = php2rust_doctrine_persistence::php_reflection::class_info(GH10288_PEOPLE)
        .expect("the enum is declared");
    let case = class
        .cases()
        .iter()
        .find(|case| case.property(b"value").is_some_and(|v| v.identical(value)))
        .cloned()
        .unwrap_or_else(|| panic!("{value:?} is not a valid backing value"));
    Value::Object(case)
}

/// `GH10288PeopleType extends StringType`.
#[derive(Debug)]
struct GH10288PeopleType;

impl GH10288PeopleType {
    const NAME: &str = "GH10288PeopleType";
}

impl Type for GH10288PeopleType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH10288PeopleType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let value = match value {
            Value::Object(object) if object.instance_of(GH10288_PEOPLE) => value.clone(),
            other => people_from(other),
        };
        let Value::Object(case) = value else {
            unreachable!("a case")
        };
        Ok(case.property(b"value").unwrap_or(Value::Null))
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(people_from(value))
    }
}

async fn set_up() -> Arc<php2rust_doctrine_orm::entity_manager::EntityManager> {
    ticket::add_type(GH10288PeopleType::NAME, Arc::new(GH10288PeopleType));
    ticket::set_up(&[
        "GH10288PersonWithEnumType",
        "GH10288BossWithEnumType",
        "GH10288EmployeeWithEnumType",
        "GH10288PersonCustomEnumType",
        "GH10288BossCustomEnumType",
        "GH10288EmployeeCustomEnumType",
    ])
    .await
}

async fn perform_enum_discriminator_test(
    em: &Arc<php2rust_doctrine_orm::entity_manager::EntityManager>,
    boss: &Object,
    employee: &Object,
    person_type: &str,
) {
    set(boss, "bossId", Value::Int(1));

    em.persist(boss).unwrap();
    em.persist(employee).unwrap();
    em.flush().await.unwrap();
    let boss_id = get(boss, "id");

    em.clear().unwrap();

    // Using DQL here to make sure that we'll use ObjectHydrator instead of SimpleObjectHydrator
    let mut qb = em.create_query_builder();
    qb.select(vec![s("person")])
        .unwrap()
        .from(&class(person_type), "person", None)
        .unwrap()
        .r#where(vec![s("person.name = :name")])
        .unwrap()
        .set_max_results(Some(1))
        .unwrap();
    let query = qb.get_query().unwrap();

    query.set_parameter("name", s("John"), None);
    assert_loose_eq(
        &Value::Object(boss.clone()),
        &query
            .get_one_or_null_result(Some(HYDRATE_OBJECT))
            .await
            .unwrap(),
    );
    let array = query
        .get_one_or_null_result(Some(HYDRATE_ARRAY))
        .await
        .unwrap();
    assert_loose_eq(&ticket::case(GH10288_PEOPLE, "BOSS"), &at(&array, "discr"));

    query.set_parameter("name", s("Bob"), None);
    assert_loose_eq(
        &Value::Object(employee.clone()),
        &query
            .get_one_or_null_result(Some(HYDRATE_OBJECT))
            .await
            .unwrap(),
    );
    let array = query
        .get_one_or_null_result(Some(HYDRATE_ARRAY))
        .await
        .unwrap();
    assert_loose_eq(
        &ticket::case(GH10288_PEOPLE, "EMPLOYEE"),
        &at(&array, "discr"),
    );

    em.clear().unwrap();

    // test SimpleObjectHydrator
    let boss_fetched = em
        .find(&class(person_type), boss_id)
        .await
        .unwrap()
        .expect("the boss");
    assert_loose_eq(&Value::Object(boss.clone()), &Value::Object(boss_fetched));
}

/// `new $class($name)`.
fn person(short: &str, name: &str) -> Object {
    let person = new(short);
    set(&person, "name", s(name));
    person
}

#[tokio::test]
async fn testEnumDiscriminatorWithEnumType() {
    let em = set_up().await;
    let boss = person("GH10288BossWithEnumType", "John");
    let employee = person("GH10288EmployeeWithEnumType", "Bob");
    perform_enum_discriminator_test(&em, &boss, &employee, "GH10288PersonWithEnumType").await;
}

#[tokio::test]
async fn testEnumDiscriminatorWithCustomEnumType() {
    let em = set_up().await;
    let boss = person("GH10288BossCustomEnumType", "John");
    let employee = person("GH10288EmployeeCustomEnumType", "Bob");
    perform_enum_discriminator_test(&em, &boss, &employee, "GH10288PersonCustomEnumType").await;
}
