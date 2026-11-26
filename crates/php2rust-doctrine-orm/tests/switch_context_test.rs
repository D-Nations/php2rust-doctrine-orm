//! Functional/Ticket/SwitchContextWithFilter/SwitchContextTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (`tests/common/orm.rs`). `getPrimaryInsurances()` and
//! `getSecondaryInsurances()` filter the collection's values by class.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/SwitchContextTest.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/AbstractTestCase.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/Entity/Insurance.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/Entity/Patient.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/Entity/PatientInsurance.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/Entity/Practice.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/Entity/PrimaryPatInsurance.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/Entity/SecondaryPatInsurance.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/SQLFilter/PracticeContextSQLFilter.php

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

use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::ClassMetadataInfo;
use php2rust_doctrine_orm::query::{FilterConstraint, SqlFilter, register_filter_class};

use orm::*;

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\Entity\\";
const PRACTICE_CONTEXT_SQL_FILTER: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\SQLFilter\\PracticeContextSQLFilter";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// `PracticeContextSQLFilter extends SQLFilter`.
struct PracticeContextSqlFilter;

impl FilterConstraint for PracticeContextSqlFilter {
    fn add_filter_constraint(
        &self,
        filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        // `getParameter()` returns the quoted value, never `null`.
        if !filter.has_parameter("practiceId") {
            return Ok(String::new());
        }

        if target_entity.name == class("Insurance") {
            return Ok(format!(
                "{target_table_alias}.{} = {}",
                target_entity
                    .association_mappings
                    .get("practice")
                    .unwrap()
                    .join_columns[0]
                    .name,
                filter.get_parameter("practiceId")?
            ));
        }

        Ok(String::new())
    }
}

async fn set_up() -> (Arc<EntityManager>, Arc<SqlFilter>) {
    register_filter_class(
        PRACTICE_CONTEXT_SQL_FILTER,
        Arc::new(|em| {
            SqlFilter::new(
                em,
                PRACTICE_CONTEXT_SQL_FILTER,
                Box::new(PracticeContextSqlFilter),
            )
        }),
    );
    let em = orm::set_up(&[
        &class("Practice"),
        &class("Patient"),
        &class("PatientInsurance"),
        &class("PrimaryPatInsurance"),
        &class("SecondaryPatInsurance"),
        &class("Insurance"),
    ])
    .await;
    em.get_configuration()
        .add_filter(PRACTICE_CONTEXT_SQL_FILTER, PRACTICE_CONTEXT_SQL_FILTER);
    let sql_filter = em
        .get_filters()
        .enable(PRACTICE_CONTEXT_SQL_FILTER)
        .unwrap();
    (em, sql_filter)
}

/// `new Practice($name)`.
fn practice(name: &str) -> Object {
    let practice = new_object(&class("Practice"));
    set(&practice, "name", s(name));
    practice
}

/// `new Insurance($practice, $name)`.
fn insurance(practice: &Object, name: &str) -> Object {
    let insurance = new_object(&class("Insurance"));
    set_obj(&insurance, "practice", practice);
    set(&insurance, "name", s(name));
    insurance
}

/// `new Patient($name)`.
fn patient(name: &str) -> Object {
    let patient = new_object(&class("Patient"));
    set(&patient, "name", s(name));
    set(
        &patient,
        "insurances",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    patient
}

/// `$patient->insurances[] = new $class($patient, $insurance)`.
fn add_insurance(patient: &Object, class_name: &str, insurance: &Object) {
    let patient_insurance = new_object(&class(class_name));
    set_obj(&patient_insurance, "patient", patient);
    set_obj(&patient_insurance, "insurance", insurance);
    append(patient, "insurances", Value::Object(patient_insurance));
}

/// `$patient->getPrimaryInsurances()->first()` (or the secondary ones).
async fn first_insurance(patient: &Object, class_name: &str) -> Object {
    let values = coll::get_values(&get(patient, "insurances")).await.unwrap();
    values
        .into_iter()
        .map(|value| obj(&value))
        .find(|insurance| insurance.instance_of(&class(class_name)))
        .expect("an insurance of the class")
}

async fn insurance_name(patient: &Object, class_name: &str) -> Value {
    let insurance = get_object(&first_insurance(patient, class_name).await, "insurance").unwrap();
    get(&insurance, "name")
}

async fn fixture_generate(em: &Arc<EntityManager>) -> (Object, Object) {
    let practice_a = practice("Practice A");
    let practice_b = practice("Practice B");
    let insurance_aetna = insurance(&practice_a, "Aetna in Practice A");
    let insurance_b_humana = insurance(&practice_b, "Humana in Practice B");
    let insurance_b_custom = insurance(&practice_b, "Custom in Practice B");

    let patient_egor = patient("Egor");
    add_insurance(&patient_egor, "PrimaryPatInsurance", &insurance_aetna);
    add_insurance(&patient_egor, "PrimaryPatInsurance", &insurance_b_humana);

    let patient_gena = patient("Gena");
    add_insurance(&patient_gena, "PrimaryPatInsurance", &insurance_b_humana);
    add_insurance(&patient_gena, "SecondaryPatInsurance", &insurance_b_custom);

    for entity in [
        &practice_a,
        &practice_b,
        &insurance_aetna,
        &insurance_b_humana,
        &insurance_b_custom,
        &patient_egor,
        &patient_gena,
    ] {
        em.persist(entity).unwrap();
    }
    em.flush().await.unwrap();
    em.clear().unwrap();

    (
        em.get_reference(&class("Patient"), get(&patient_egor, "id"))
            .await
            .unwrap()
            .unwrap(),
        em.get_reference(&class("Patient"), get(&patient_gena, "id"))
            .await
            .unwrap()
            .unwrap(),
    )
}

/// `clearCachedData(...$entities)`.
async fn clear_cached_data(em: &EntityManager, entities: &[&Object]) {
    for entity in entities {
        em.refresh(entity).await.unwrap();
    }
}

async fn find_practice(em: &Arc<EntityManager>, name: &str) -> Object {
    let mut criteria = PhpArray::new();
    criteria.insert("name", s(name));
    em.get_repository(&class("Practice"))
        .unwrap()
        .find_one_by(&criteria, None)
        .await
        .unwrap()
        .expect("a practice")
}

#[tokio::test]
async fn testSwitchContext() {
    let (em, sql_filter) = set_up().await;
    let (patient_egor, patent_gena) = fixture_generate(&em).await;

    let practice_a = find_practice(&em, "Practice A").await;
    let practice_b = find_practice(&em, "Practice B").await;

    // switchPracticeContext($practiceA, ...)
    sql_filter
        .set_parameter("practiceId", get(&practice_a, "id"), None)
        .unwrap();
    clear_cached_data(&em, &[&patent_gena, &patient_egor]).await;
    assert_eq!(
        1,
        coll::count(&get(&patient_egor, "insurances"))
            .await
            .unwrap()
    );
    first_insurance(&patient_egor, "PrimaryPatInsurance").await;
    assert_loose_eq(
        &s("Aetna in Practice A"),
        &insurance_name(&patient_egor, "PrimaryPatInsurance").await,
    );
    assert_eq!(
        0,
        coll::count(&get(&patent_gena, "insurances")).await.unwrap()
    );
    sql_filter
        .set_parameter("practiceId", Value::Null, None)
        .unwrap();

    // switchPracticeContext($practiceB, ...)
    sql_filter
        .set_parameter("practiceId", get(&practice_b, "id"), None)
        .unwrap();
    clear_cached_data(&em, &[&patent_gena, &patient_egor]).await;
    assert_eq!(
        1,
        coll::count(&get(&patient_egor, "insurances"))
            .await
            .unwrap()
    );
    first_insurance(&patient_egor, "PrimaryPatInsurance").await;
    assert_loose_eq(
        &s("Humana in Practice B"),
        &insurance_name(&patient_egor, "PrimaryPatInsurance").await,
    );
    assert_eq!(
        2,
        coll::count(&get(&patent_gena, "insurances")).await.unwrap()
    );
    first_insurance(&patent_gena, "PrimaryPatInsurance").await;
    first_insurance(&patent_gena, "SecondaryPatInsurance").await;
    assert_loose_eq(
        &s("Humana in Practice B"),
        &insurance_name(&patent_gena, "PrimaryPatInsurance").await,
    );
    assert_loose_eq(
        &s("Custom in Practice B"),
        &insurance_name(&patent_gena, "SecondaryPatInsurance").await,
    );
    sql_filter
        .set_parameter("practiceId", Value::Null, None)
        .unwrap();
}
