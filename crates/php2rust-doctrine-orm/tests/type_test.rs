//! Functional/TypeTest.php on `SQLite` (`tests/common/orm.rs`).
//!
//! `new DateTimeZone('Europe/Berlin')` is its fixed offset on the test
//! dates (`+02:00`, summer time): php-value's `TimeZone` has no named
//! zones. `testArray` and `testObject` are n/a: they are skipped on DBAL 4
//! (no `ArrayType`, `ObjectType`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/TypeTest.php

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

use php_value::datetime::{DateTime, DateTimeImmutable, Fields, TimeZone};
use php_value::{PhpArray, Value, php_array};
use php2rust_doctrine_dbal::WrapperParameterType;
use php2rust_doctrine_dbal::types::Types;
use php2rust_doctrine_orm::collection_object::downcast;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{get, new_object, obj, s, set};

const BOOLEAN_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\BooleanModel";
const DATE_TIME_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\DateTimeModel";
const DECIMAL_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\DecimalModel";

/// `Europe/Berlin` in summer (CEST).
const BERLIN_SUMMER: TimeZone = TimeZone::Offset(7200);

async fn set_up() -> Arc<EntityManager> {
    // useModelSet('generic')
    orm::set_up(&[BOOLEAN_MODEL, DATE_TIME_MODEL, DECIMAL_MODEL]).await
}

/// `new DateTime('Y-m-d H:i:s', $zone)`.
fn date_time(
    (year, month, day): (i64, i64, i64),
    (hour, minute, second): (i64, i64, i64),
    zone: TimeZone,
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
        zone,
    ))
}

/// `assertInstanceOf(DateTime::class, $value)` and `$value->format($format)`.
fn format(value: &Value, format: &str) -> Value {
    let object = obj(value);
    assert_eq!(object.class(), "DateTime");
    s(&downcast::<DateTime>(&object).unwrap().format(format))
}

fn assert_same(expected: &Value, actual: &Value) {
    assert!(expected.identical(actual), "{expected:?} !== {actual:?}");
}

async fn single_result(em: &Arc<EntityManager>, dql: &str) -> php_value::Object {
    obj(&em.create_query(dql).get_single_result(None).await.unwrap())
}

async fn find_by(em: &Arc<EntityManager>, criteria: &PhpArray) -> Vec<php_value::Object> {
    em.get_repository(DATE_TIME_MODEL)
        .unwrap()
        .find_by(criteria, None, None, None)
        .await
        .unwrap()
}

#[tokio::test]
async fn testDecimal() {
    let em = set_up().await;
    let decimal = new_object(DECIMAL_MODEL);
    set(&decimal, "decimal", Value::Float(0.15));
    set(&decimal, "highScale", Value::Float(0.1515));

    em.persist(&decimal).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql = format!("SELECT d FROM {DECIMAL_MODEL} d");
    let decimal = single_result(&em, &dql).await;

    assert_same(&s("0.15"), &get(&decimal, "decimal"));
    assert_same(&s("0.1515"), &get(&decimal, "highScale"));
}

#[tokio::test]
async fn testBoolean() {
    let em = set_up().await;
    let bool_ = new_object(BOOLEAN_MODEL);
    set(&bool_, "booleanField", Value::Bool(true));

    em.persist(&bool_).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql = format!("SELECT b FROM {BOOLEAN_MODEL} b WHERE b.booleanField = true");
    let bool_ = single_result(&em, &dql).await;

    assert_same(&Value::Bool(true), &get(&bool_, "booleanField"));

    set(&bool_, "booleanField", Value::Bool(false));

    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql = format!("SELECT b FROM {BOOLEAN_MODEL} b WHERE b.booleanField = false");
    let bool_ = single_result(&em, &dql).await;

    assert_same(&Value::Bool(false), &get(&bool_, "booleanField"));
}

#[tokio::test]
async fn testDate() {
    let em = set_up().await;
    let date_time = new_object(DATE_TIME_MODEL);
    set(
        &date_time,
        "date",
        self::date_time((2009, 10, 1), (0, 0, 0), BERLIN_SUMMER),
    );

    em.persist(&date_time).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let date_time_db = em
        .find(DATE_TIME_MODEL, get(&date_time, "id"))
        .await
        .unwrap()
        .unwrap();

    assert_same(
        &s("2009-10-01"),
        &format(&get(&date_time_db, "date"), "Y-m-d"),
    );
}

#[tokio::test]
async fn testDateTime() {
    let em = set_up().await;
    let date_time = new_object(DATE_TIME_MODEL);
    set(
        &date_time,
        "datetime",
        self::date_time((2009, 10, 2), (20, 10, 52), BERLIN_SUMMER),
    );

    em.persist(&date_time).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let date_time_db = em
        .find(DATE_TIME_MODEL, get(&date_time, "id"))
        .await
        .unwrap()
        .unwrap();

    assert_same(
        &s("2009-10-02 20:10:52"),
        &format(&get(&date_time_db, "datetime"), "Y-m-d H:i:s"),
    );

    let articles = find_by(
        &em,
        &php_array! { "datetime" => DateTime::object(DateTimeImmutable::now(TimeZone::Utc)) },
    )
    .await;

    assert!(articles.is_empty());
}

#[tokio::test]
async fn testDqlQueryBindDateTimeInstance() {
    let em = set_up().await;
    let date = date_time((2009, 10, 2), (20, 10, 52), BERLIN_SUMMER);

    let date_time = new_object(DATE_TIME_MODEL);
    set(&date_time, "datetime", date.clone());

    em.persist(&date_time).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(
        "SELECT d FROM Doctrine\\Tests\\Models\\Generic\\DateTimeModel d WHERE d.datetime = ?1",
    );
    query.set_parameter(
        1,
        date,
        Some(WrapperParameterType::Name(Types::DATETIME_MUTABLE.into())),
    );
    let date_time_db = obj(&query.get_single_result(None).await.unwrap());

    assert_same(
        &s("2009-10-02 20:10:52"),
        &format(&get(&date_time_db, "datetime"), "Y-m-d H:i:s"),
    );
}

#[tokio::test]
async fn testDqlQueryBuilderBindDateTimeInstance() {
    let em = set_up().await;
    let date = date_time((2009, 10, 2), (20, 10, 52), BERLIN_SUMMER);

    let date_time = new_object(DATE_TIME_MODEL);
    set(&date_time, "datetime", date.clone());

    em.persist(&date_time).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let mut qb = em.create_query_builder();
    qb.select(vec![s("d")])
        .unwrap()
        .from(DATE_TIME_MODEL, "d", None)
        .unwrap()
        .r#where(vec![s("d.datetime = ?1")])
        .unwrap()
        .set_parameter(
            1,
            date,
            Some(WrapperParameterType::Name(Types::DATETIME_MUTABLE.into())),
        );
    let date_time_db = obj(&qb
        .get_query()
        .unwrap()
        .get_single_result(None)
        .await
        .unwrap());

    assert_same(
        &s("2009-10-02 20:10:52"),
        &format(&get(&date_time_db, "datetime"), "Y-m-d H:i:s"),
    );
}

#[tokio::test]
async fn testTime() {
    let em = set_up().await;
    let date_time = new_object(DATE_TIME_MODEL);
    set(
        &date_time,
        "time",
        self::date_time((2010, 1, 1), (19, 27, 20), TimeZone::Utc),
    );

    em.persist(&date_time).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let date_time_db = em
        .find(DATE_TIME_MODEL, get(&date_time, "id"))
        .await
        .unwrap()
        .unwrap();

    assert_same(
        &s("19:27:20"),
        &format(&get(&date_time_db, "time"), "H:i:s"),
    );
}

#[tokio::test]
async fn testFindByDateWithString() {
    let em = set_up().await;
    let date_time = new_object(DATE_TIME_MODEL);
    set(
        &date_time,
        "date",
        self::date_time((2009, 10, 1), (0, 0, 0), BERLIN_SUMMER),
    );

    em.persist(&date_time).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let found = find_by(&em, &php_array! { "date" => "2009-10-01" }).await;

    assert_eq!(found.len(), 1);
    assert_same(&s("2009-10-01"), &format(&get(&found[0], "date"), "Y-m-d"));
}

#[tokio::test]
async fn testFindByTimeWithString() {
    let em = set_up().await;
    let date_time = new_object(DATE_TIME_MODEL);
    set(
        &date_time,
        "time",
        self::date_time((2010, 1, 1), (19, 27, 20), TimeZone::Utc),
    );

    em.persist(&date_time).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let found = find_by(&em, &php_array! { "time" => "19:27:20" }).await;

    assert_eq!(found.len(), 1);
    assert_same(&s("19:27:20"), &format(&get(&found[0], "time"), "H:i:s"));
}
