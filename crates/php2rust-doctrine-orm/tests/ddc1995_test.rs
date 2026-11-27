//! Ticket/DDC1995Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the `company` model set); the functional test case's query
//! cache is a new `ArrayAdapter`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1995Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/company.rs"]
mod company;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php_value::Value;
use php2rust_doctrine_orm::cache::pool::ArrayAdapter;

use company::*;
use orm::*;

#[tokio::test]
async fn testIssue() {
    register_listeners();
    let em = orm::set_up(&COMPANY_MODELS).await;

    let person = company_person("p1");
    let employee = company_employee("Foo", 1000, "bar");

    em.persist(&person).unwrap();
    em.persist(&employee).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql =
        "SELECT u FROM Doctrine\\Tests\\Models\\Company\\CompanyPerson u WHERE u INSTANCE OF ?1";
    let class = em.get_class_metadata(COMPANY_EMPLOYEE).unwrap();

    let query = em.create_query(dql);
    query.set_parameter(1, Value::Object(class), None);
    let result = list(&query.get_object_result().await.unwrap());

    assert_eq!(1, result.len());
    assert!(obj(&result[0]).instance_of(COMPANY_EMPLOYEE));
}

#[tokio::test]
async fn testQueryCache() {
    register_listeners();
    let em = orm::set_up(&COMPANY_MODELS).await;
    em.get_configuration()
        .set_query_cache(Arc::new(ArrayAdapter::new()));

    let person = company_person("p1");
    let employee = company_employee("Foo", 1000, "bar");

    em.persist(&person).unwrap();
    em.persist(&employee).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql =
        "SELECT u FROM Doctrine\\Tests\\Models\\Company\\CompanyPerson u WHERE u INSTANCE OF :type";
    let class1 = em.get_class_metadata(COMPANY_EMPLOYEE).unwrap();
    let class2 = em.get_class_metadata(COMPANY_PERSON).unwrap();

    let query1 = em.create_query(dql);
    query1.set_parameter("type", Value::Object(class1), None);
    query1.use_query_cache(true);
    let result1 = list(&query1.get_object_result().await.unwrap());

    let query2 = em.create_query(dql);
    query2.set_parameter("type", Value::Object(class2), None);
    query2.use_query_cache(true);
    let result2 = list(&query2.get_object_result().await.unwrap());

    assert_eq!(1, result1.len());
    assert_eq!(2, result2.len());

    assert!(obj(&result1[0]).instance_of(COMPANY_EMPLOYEE));
    for person in &result2 {
        assert!(obj(person).instance_of(COMPANY_PERSON));
    }
}
