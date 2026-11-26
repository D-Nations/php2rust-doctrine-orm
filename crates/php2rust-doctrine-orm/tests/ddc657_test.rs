//! Ticket/DDC657Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the generic models).
//! `new DateTimeZone('America/Sao_Paulo')` is its fixed offset on the
//! fixture dates (`-02:00`, summer time in January and February 2010).
//! `testTicketResult` is pending: it formats with `G`, which the runtime's
//! `DateTime::format()` prints as a literal `G`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC657Test.php

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

use php_value::Value;
use php_value::datetime::{DateTime, DateTimeImmutable, Fields, TimeZone};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

const BOOLEAN_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\BooleanModel";
const DATE_TIME_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\DateTimeModel";
const DECIMAL_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\DecimalModel";

/// `America/Sao_Paulo` on the fixture dates.
const SAO_PAULO: TimeZone = TimeZone::Offset(-7200);

/// `new DateTime('Y-m-d H:i:s', $timezone)`.
fn date_time(
    (year, month, day): (i64, i64, i64),
    (hour, minute, second): (i64, i64, i64),
) -> Value {
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
        SAO_PAULO,
    ))
}

async fn set_up() -> Arc<EntityManager> {
    // useModelSet('generic')
    let em = orm::set_up(&[BOOLEAN_MODEL, DATE_TIME_MODEL, DECIMAL_MODEL]).await;
    load_fixtures(&em).await;
    em
}

async fn load_fixtures(em: &EntityManager) {
    let date_time1 = new_object(DATE_TIME_MODEL);
    let date_time2 = new_object(DATE_TIME_MODEL);

    set(&date_time1, "date", date_time((2010, 1, 1), (0, 0, 0)));
    set(&date_time1, "time", date_time((2010, 1, 1), (11, 11, 11)));
    set(
        &date_time1,
        "datetime",
        date_time((2010, 1, 1), (11, 11, 11)),
    );

    set(&date_time2, "date", date_time((2010, 2, 2), (0, 0, 0)));
    set(&date_time2, "time", date_time((2010, 2, 2), (12, 12, 12)));
    set(
        &date_time2,
        "datetime",
        date_time((2010, 2, 2), (12, 12, 12)),
    );

    em.persist(&date_time1).unwrap();
    em.persist(&date_time2).unwrap();

    em.flush().await.unwrap();
}

/// `self::assertInstanceOf(DateTime::class, $value)`.
fn assert_date_time(value: &Value) {
    assert_eq!("DateTime", obj(value).class(), "{value:?}");
}

/// `self::assertStringContainsString($needle, $value)`.
fn assert_contains(needle: &str, value: &Value) {
    let haystack = value
        .to_php_string()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(
        haystack.contains(needle),
        "{haystack:?} does not contain {needle:?}"
    );
}

#[tokio::test]
async fn testEntitySingleResult() {
    let em = set_up().await;
    let query = em.create_query(&format!("SELECT d FROM {DATE_TIME_MODEL} d"));
    query.set_max_results(Some(1));
    let datetime = obj(&query.get_single_result(None).await.unwrap());

    assert_eq!(DATE_TIME_MODEL, datetime.class());
    assert_date_time(&get(&datetime, "datetime"));
    assert_date_time(&get(&datetime, "time"));
    assert_date_time(&get(&datetime, "date"));
}

#[tokio::test]
async fn testScalarResult() {
    let em = set_up().await;
    let query = em.create_query(&format!(
        "SELECT d.id, d.time, d.date, d.datetime FROM {DATE_TIME_MODEL} d ORDER BY d.date ASC"
    ));
    let result = list(&query.get_scalar_result().await.unwrap());

    assert_eq!(2, result.len());

    assert_contains("11:11:11", &at(&result[0], "time"));
    assert_contains("2010-01-01", &at(&result[0], "date"));
    assert_contains("2010-01-01 11:11:11", &at(&result[0], "datetime"));

    assert_contains("12:12:12", &at(&result[1], "time"));
    assert_contains("2010-02-02", &at(&result[1], "date"));
    assert_contains("2010-02-02 12:12:12", &at(&result[1], "datetime"));
}

#[tokio::test]
async fn testaTicketEntityArrayResult() {
    let em = set_up().await;
    let query = em.create_query(&format!(
        "SELECT d FROM {DATE_TIME_MODEL} d ORDER BY d.date ASC"
    ));
    let result = list(&query.get_array_result().await.unwrap());

    assert_eq!(2, result.len());

    for row in &result {
        assert_date_time(&at(row, "datetime"));
        assert_date_time(&at(row, "time"));
        assert_date_time(&at(row, "date"));
    }
}

#[tokio::test]
async fn testTicketSingleResult() {
    let em = set_up().await;
    let query = em.create_query(&format!(
        "SELECT d.id, d.time, d.date, d.datetime FROM {DATE_TIME_MODEL} d ORDER BY d.date ASC"
    ));
    query.set_max_results(Some(1));
    let datetime = query.get_single_result(None).await.unwrap();

    assert!(matches!(datetime, Value::Array(_)), "assertIsArray");
    assert_date_time(&at(&datetime, "datetime"));
    assert_date_time(&at(&datetime, "time"));
    assert_date_time(&at(&datetime, "date"));
}
