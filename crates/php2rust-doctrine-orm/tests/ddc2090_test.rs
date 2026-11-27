//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2090Test.php
//!
//! On `SQLite` with the `company` model set (`tests/common/orm.rs`); the
//! functional test case's query cache is a new `ArrayAdapter`. The
//! `DateTime('yesterday')` start dates are overwritten by the updates, so
//! they are any date here.

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

use php_value::datetime::{DateTime, DateTimeImmutable, Fields, TimeZone};
use php_value::{Object, Value};
use php2rust_doctrine_orm::cache::pool::ArrayAdapter;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::Parameter;

use company::*;
use orm::*;

fn date_time(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64) -> Value {
    DateTime::object(DateTimeImmutable::from_fields(
        Fields {
            year,
            month,
            day,
            hour,
            minute,
            second,
            microsecond: 0,
        },
        TimeZone::Utc,
    ))
}

/// `new DateTime('yesterday')`.
fn yesterday() -> Value {
    let today = DateTimeImmutable::now(TimeZone::Utc).local_fields();
    date_time(today.year, today.month, today.day - 1, 0, 0, 0)
}

/// `$qb->update(CompanyEmployee::class, 'e')->set(...)->where(...)
/// ->setParameters(...)->getQuery()->useQueryCache(true)->execute()`.
async fn update(
    em: &Arc<EntityManager>,
    placeholders: [&str; 3],
    names: [&str; 3],
    employee: &Object,
    date: &Value,
    salary: i64,
) {
    let mut qb = em.create_query_builder();
    qb.update(Some(COMPANY_EMPLOYEE), Some("e")).unwrap();
    qb.set("e.startDate", s(placeholders[1])).unwrap();
    qb.set("e.salary", s(placeholders[2])).unwrap();
    qb.r#where(vec![s(&format!("e = {}", placeholders[0]))])
        .unwrap();
    qb.set_parameters(vec![
        Parameter::new(names[0], Value::Object(employee.clone()), None),
        Parameter::new(names[1], date.clone(), None),
        Parameter::new(names[2], salary, None),
    ]);
    let query = qb.get_query().unwrap();
    query.use_query_cache(true);
    query.execute(None, None).await.unwrap();
}

async fn assert_employees(
    em: &Arc<EntityManager>,
    employee1: &Object,
    employee2: &Object,
    date1: &Value,
    date2: &Value,
) {
    let e1 = em
        .find(COMPANY_EMPLOYEE, get(employee1, "id"))
        .await
        .unwrap()
        .unwrap();
    let e2 = em
        .find(COMPANY_EMPLOYEE, get(employee2, "id"))
        .await
        .unwrap()
        .unwrap();

    assert_loose_eq(&Value::Int(101), &get(&e1, "salary"));
    assert_loose_eq(&Value::Int(102), &get(&e2, "salary"));
    assert_loose_eq(date1, &get(&e1, "startDate"));
    assert_loose_eq(date2, &get(&e2, "startDate"));
}

#[tokio::test]
async fn testIssue() {
    register_listeners();
    let em = orm::set_up(&COMPANY_MODELS).await;
    em.get_configuration()
        .set_query_cache(Arc::new(ArrayAdapter::new()));

    let date1 = date_time(2011, 11, 11, 11, 11, 11);
    let date2 = date_time(2012, 12, 12, 12, 12, 12);
    let employee1 = company_employee("Fabio B. Silva", 100, "R&D");
    let employee2 = company_employee("Doctrine Bot", 100, "QA");
    set(&employee1, "startDate", yesterday());
    set(&employee1, "startDate", yesterday());

    em.persist(&employee1).unwrap();
    em.persist(&employee2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let named = [":e", ":date", ":salary"];
    let parameter_names = ["e", "date", "salary"];
    update(&em, named, parameter_names, &employee1, &date1, 101).await;
    update(&em, named, parameter_names, &employee2, &date2, 102).await;

    em.clear().unwrap();

    assert_employees(&em, &employee1, &employee2, &date1, &date2).await;

    let positional = ["?0", "?1", "?2"];
    let numbers = ["0", "1", "2"];
    update(&em, positional, numbers, &employee1, &date1, 101).await;
    update(&em, positional, numbers, &employee2, &date2, 102).await;

    em.clear().unwrap();

    assert_employees(&em, &employee1, &employee2, &date1, &date2).await;
}
