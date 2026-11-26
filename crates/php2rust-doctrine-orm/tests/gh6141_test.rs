//! Ticket/GH6141Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `GH6141PeopleType` and `GH6141People` are Rust types).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6141Test.php

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

use php_value::{Object, PhpArray, PhpObject, PhpString, Thrown, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type};
use php2rust_doctrine_orm::abstract_query::{HYDRATE_ARRAY, HYDRATE_OBJECT};

use orm::*;
use ticket::*;

/// `GH6141People implements Stringable`.
#[derive(Debug)]
struct GH6141People {
    value: PhpString,
}

impl GH6141People {
    const CLASS: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH6141People";
    const BOSS: &str = "boss";
    const EMPLOYEE: &str = "employee";

    /// `GH6141People::get($value)`.
    fn get(value: &PhpString) -> Result<Value, Thrown> {
        if !Self::is_valid(value) {
            return Err(Thrown::new("InvalidArgumentException", "", 0));
        }

        let people: Object = Arc::new(Self {
            value: value.clone(),
        });
        Ok(Value::Object(people))
    }

    fn is_valid(valid: &PhpString) -> bool {
        [Self::BOSS, Self::EMPLOYEE]
            .iter()
            .any(|value| value.as_bytes() == valid.as_bytes())
    }
}

impl PhpObject for GH6141People {
    fn class(&self) -> &str {
        Self::CLASS
    }

    fn properties(&self) -> PhpArray {
        let mut properties = PhpArray::new();
        properties.insert("value", Value::String(self.value.clone()));
        properties
    }

    fn to_php_string(&self) -> Option<PhpString> {
        Some(self.value.clone())
    }
}

/// `GH6141PeopleType extends StringType`.
#[derive(Debug)]
struct GH6141PeopleType;

impl GH6141PeopleType {
    const NAME: &str = "gh6141people";
}

impl Type for GH6141PeopleType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH6141PeopleType"
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
            Value::Object(object) if object.class() == GH6141People::CLASS => value.clone(),
            other => GH6141People::get(&other.to_php_string().unwrap_or_default())
                .map_err(ConversionError::Thrown)?,
        };
        Ok(Value::String(
            value.to_php_string_or_stringable().unwrap_or_default(),
        ))
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        GH6141People::get(&value.to_php_string().unwrap_or_default())
            .map_err(ConversionError::Thrown)
    }
}

/// `new $class($name)`.
fn person(short: &str, name: &str) -> Object {
    let person = new(short);
    set(&person, "name", s(name));
    person
}

#[tokio::test]
async fn testEnumDiscriminatorsShouldBeConvertedToString() {
    ticket::add_type(GH6141PeopleType::NAME, Arc::new(GH6141PeopleType));
    let em = ticket::set_up(&["GH6141Person", "GH6141Boss", "GH6141Employee"]).await;

    let boss = person("GH6141Boss", "John");
    let employee = person("GH6141Employee", "Bob");

    em.persist(&boss).unwrap();
    em.persist(&employee).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    // Using DQL here to make sure that we'll use ObjectHydrator instead of SimpleObjectHydrator
    let mut qb = em.create_query_builder();
    qb.select(vec![s("person")])
        .unwrap()
        .from(&class("GH6141Person"), "person", None)
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
    assert_loose_eq(
        &GH6141People::get(&PhpString::from(GH6141People::BOSS)).unwrap(),
        &at(&array, "discr"),
    );

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
        &GH6141People::get(&PhpString::from(GH6141People::EMPLOYEE)).unwrap(),
        &at(&array, "discr"),
    );
}
