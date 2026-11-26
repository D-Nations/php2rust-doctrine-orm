//! Upstream: doctrine/orm tests/Tests/ORM/Hydration/SimpleObjectHydratorTest.php
//! Upstream: doctrine/orm tests/Tests/DbalTypes/GH8565EmployeePayloadType.php
//! Upstream: doctrine/orm tests/Tests/DbalTypes/GH8565ManagerPayloadType.php
//!
//! Translated line by line over the PHP value helpers of
//! `common/php_test.rs`.

#![allow(non_snake_case)]
#![allow(
    unused_mut,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::redundant_clone,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/php_test.rs"]
mod php_test;

use std::sync::Arc;

use php_value::{PhpArray, PhpString, Thrown, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type, TypeArgument, add_type, has_type};
use php2rust_doctrine_orm::abstract_query::HYDRATE_SIMPLEOBJECT;
use php2rust_doctrine_orm::query::ResultSetMapping;

use php_test::*;

/// The entity manager, on a database with the schema of `CmsAddress`, so
/// that the query of its inverse one-to-one association finds no rows
/// (as the mocked connection of upstream).
async fn entity_manager() -> Arc<php2rust_doctrine_orm::entity_manager::EntityManager> {
    orm::set_up(&orm::CMS_MODELS).await
}

/// `GH8565EmployeePayloadType`: a `JsonType` that needs a field.
#[derive(Debug)]
struct Gh8565EmployeePayloadType;

impl Type for Gh8565EmployeePayloadType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\GH8565EmployeePayloadType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_json_type_declaration_sql(column)
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        match value {
            Value::Array(array) => match array.get(&"GH8565EmployeePayloadRequiredField".into()) {
                Some(field) if !matches!(field, Value::Null) => Ok(field.clone()),
                _ => Err(employee_payload_error()),
            },
            _ => Err(employee_payload_error()),
        }
    }
}

fn employee_payload_error() -> ConversionError {
    Thrown::new(
        "Exception",
        "GH8565EmployeePayloadType cannot be initialized without required field",
        0,
    )
    .into()
}

/// `GH8565ManagerPayloadType`: a `JsonType` that keeps the value.
#[derive(Debug)]
struct Gh8565ManagerPayloadType;

impl Type for Gh8565ManagerPayloadType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\GH8565ManagerPayloadType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_json_type_declaration_sql(column)
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(value.clone())
    }
}

/// `DBALType::addType(GH8565EmployeePayloadType::NAME, ...)` and the
/// manager type, once per process.
fn register_gh8565_types() {
    if !has_type("GH8565EmployeePayloadType") {
        let _ = add_type(
            "GH8565EmployeePayloadType",
            TypeArgument::Instance(Arc::new(Gh8565EmployeePayloadType)),
        );
    }
    if !has_type("GH8565ManagerPayloadType") {
        let _ = add_type(
            "GH8565ManagerPayloadType",
            TypeArgument::Instance(Arc::new(Gh8565ManagerPayloadType)),
        );
    }
}

#[tokio::test]
async fn testMissingDiscriminatorColumnException() {
    let em = entity_manager().await;
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\Company\\CompanyPerson"), s("p")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("p"), s("p__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("p"), s("p__name"), s("name")],
    );
    rsm_call(
        &mut rsm,
        "addMetaResult",
        &[
            s("p "),
            s("discr"),
            s("discr"),
            Value::Bool(false),
            s("string"),
        ],
    );
    rsm_call(&mut rsm, "setDiscriminatorColumn", &[s("p"), s("discr")]);
    let mut result_set = list_of(vec![array_of(vec![
        (s("u__id"), s("1")),
        (s("u__name"), s("Fabio B. Silva")),
    ])]);
    let e = php_hydrate_all(&em, HYDRATE_SIMPLEOBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap_err();
    assert_eq!(
        s(&e.class),
        s("Doctrine\\ORM\\Internal\\Hydration\\HydrationException")
    );
    assert_eq!(
        s(&orm::message(&e)),
        s(
            "The discriminator column \"discr\" is missing for \"Doctrine\\Tests\\Models\\Company\\CompanyPerson\" using the DQL alias \"p\"."
        )
    );
}

#[tokio::test]
async fn testExtraFieldInResultSetShouldBeIgnore() {
    let em = entity_manager().await;
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\CMS\\CmsAddress"), s("a")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("a"), s("a__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("a"), s("a__city"), s("city")],
    );
    let mut result_set = list_of(vec![array_of(vec![
        (s("a__id"), s("1")),
        (s("a__city"), s("Cracow")),
        (s("doctrine_rownum"), s("1")),
    ])]);
    let expected_entity = orm::new_object("Doctrine\\Tests\\Models\\CMS\\CmsAddress");
    orm::set(&expected_entity, "id", Value::Int(1));
    orm::set(&expected_entity, "city", s("Cracow"));
    let expected_entity = Value::Object(expected_entity);
    let result = php_hydrate_all(&em, HYDRATE_SIMPLEOBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(
        &idx(&result.clone(), &Value::Int(0)),
        &expected_entity.clone(),
    );
}

#[tokio::test]
async fn testInvalidDiscriminatorValueException() {
    let em = entity_manager().await;
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\Company\\CompanyPerson"), s("p")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("p"), s("p__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("p"), s("p__name"), s("name")],
    );
    rsm_call(
        &mut rsm,
        "addMetaResult",
        &[
            s("p"),
            s("discr"),
            s("discr"),
            Value::Bool(false),
            s("string"),
        ],
    );
    rsm_call(&mut rsm, "setDiscriminatorColumn", &[s("p"), s("discr")]);
    let mut result_set = list_of(vec![array_of(vec![
        (s("p__id"), s("1")),
        (s("p__name"), s("Fabio B. Silva")),
        (s("discr"), s("subworker")),
    ])]);
    let e = php_hydrate_all(&em, HYDRATE_SIMPLEOBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap_err();
    assert_eq!(
        s(&e.class),
        s("Doctrine\\ORM\\Internal\\Hydration\\HydrationException")
    );
    assert_eq!(
        s(&orm::message(&e)),
        s(
            "The discriminator value \"subworker\" is invalid. It must be one of \"person\", \"manager\", \"employee\"."
        )
    );
}

#[tokio::test]
async fn testNullValueShouldNotOverwriteFieldWithSameNameInJoinedInheritance() {
    let em = entity_manager().await;
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\Issue5989\\Issue5989Person"),
            s("p"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("p"), s("p__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[
            s("p"),
            s("m__tags"),
            s("tags"),
            s("Doctrine\\Tests\\Models\\Issue5989\\Issue5989Manager"),
        ],
    );
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[
            s("p"),
            s("e__tags"),
            s("tags"),
            s("Doctrine\\Tests\\Models\\Issue5989\\Issue5989Employee"),
        ],
    );
    rsm_call(
        &mut rsm,
        "addMetaResult",
        &[
            s("p"),
            s("discr"),
            s("discr"),
            Value::Bool(false),
            s("string"),
        ],
    );
    let mut result_set = list_of(vec![array_of(vec![
        (s("p__id"), s("1")),
        (s("m__tags"), s("tag1,tag2")),
        (s("e__tags"), Value::Null),
        (s("discr"), s("manager")),
    ])]);
    let expected_entity = orm::new_object("Doctrine\\Tests\\Models\\Issue5989\\Issue5989Manager");
    orm::set(&expected_entity, "id", Value::Int(1));
    orm::set(
        &expected_entity,
        "tags",
        list_of(vec![s("tag1"), s("tag2")]),
    );
    let expected_entity = Value::Object(expected_entity);
    let result = php_hydrate_all(&em, HYDRATE_SIMPLEOBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(
        &idx(&result.clone(), &Value::Int(0)),
        &expected_entity.clone(),
    );
}

#[tokio::test]
async fn testWrongValuesShouldNotBeConvertedToPhpValue() {
    let em = entity_manager().await;
    register_gh8565_types();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\GH8565\\GH8565Person"), s("p")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("p"), s("p__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[
            s("p"),
            s("m__type"),
            s("type"),
            s("Doctrine\\Tests\\Models\\GH8565\\GH8565Manager"),
        ],
    );
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[
            s("p"),
            s("e__type"),
            s("type"),
            s("Doctrine\\Tests\\Models\\GH8565\\GH8565Employee"),
        ],
    );
    rsm_call(
        &mut rsm,
        "addMetaResult",
        &[
            s("p"),
            s("discr"),
            s("discr"),
            Value::Bool(false),
            s("string"),
        ],
    );
    rsm_call(&mut rsm, "setDiscriminatorColumn", &[s("p"), s("type")]);
    let mut result_set = list_of(vec![array_of(vec![
        (s("p__id"), s("1")),
        (s("m__type"), s("type field")),
        (s("e__type"), s("type field")),
        (s("e__tags"), Value::Null),
        (s("discr"), s("manager")),
    ])]);
    let expected_entity = orm::new_object("Doctrine\\Tests\\Models\\GH8565\\GH8565Manager");
    orm::set(&expected_entity, "id", Value::Int(1));
    orm::set(&expected_entity, "type", s("type field"));
    let expected_entity = Value::Object(expected_entity);
    let result = php_hydrate_all(&em, HYDRATE_SIMPLEOBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(
        &idx(&result.clone(), &Value::Int(0)),
        &expected_entity.clone(),
    );
}

#[tokio::test]
async fn testNotListedValueInEnumArray() {
    let em = entity_manager().await;
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\Enums\\Scale"), s("s")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("s"), s("s__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("s"), s("s__supported_units"), s("supportedUnits")],
    );
    rsm_call(
        &mut rsm,
        "addEnumResult",
        &[
            s("s__supported_units"),
            s("Doctrine\\Tests\\Models\\Enums\\Unit"),
        ],
    );
    let mut result_set = list_of(vec![array_of(vec![
        (s("s__id"), Value::Int(1)),
        (s("s__supported_units"), s("g,m,unknown_case")),
    ])]);
    let e = php_hydrate_all(&em, HYDRATE_SIMPLEOBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap_err();
    assert_eq!(s(&e.class), s("Doctrine\\ORM\\Mapping\\MappingException"));
    // expectExceptionMessage(): the message contains it.
    assert!(
        orm::message(&e).contains(
            "Case \"unknown_case\" is not listed in enum \"Doctrine\\Tests\\Models\\Enums\\Unit\""
        ),
        "{e:?}"
    );
}
