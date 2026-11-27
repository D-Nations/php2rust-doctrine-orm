//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OneToOneEagerLoadingTest.php
//!
//! On `SQLite` (`tests/common/orm.rs`) with the query log
//! (`tests/common/query_log.rs`). The models are recorded lines of
//! `tests/data/models.json`; their constructors and setters are the
//! functions below.

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
#[path = "common/query_log.rs"]
mod query_log;

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_dbal::ParameterTypes;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{get, new_object, s, same_value, set};
use query_log::QueryLog;

const TRAIN: &str = "Doctrine\\Tests\\ORM\\Functional\\Train";
const TRAIN_DRIVER: &str = "Doctrine\\Tests\\ORM\\Functional\\TrainDriver";
const TRAIN_OWNER: &str = "Doctrine\\Tests\\ORM\\Functional\\TrainOwner";
const WAGGON: &str = "Doctrine\\Tests\\ORM\\Functional\\Waggon";
const TRAIN_ORDER: &str = "Doctrine\\Tests\\ORM\\Functional\\TrainOrder";

/// `setUp()`.
async fn set_up() -> (Arc<EntityManager>, Arc<QueryLog>) {
    let (em, log) =
        query_log::set_up_logged(&[TRAIN, TRAIN_DRIVER, TRAIN_OWNER, WAGGON, TRAIN_ORDER]).await;
    // `$this->getQueryLog()->enable()`
    log.reset_and_enable();
    (em, log)
}

/// `new Train($owner)`.
fn train(owner: &Object) -> Object {
    let train = new_object(TRAIN);
    set(
        &train,
        "waggons",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set_owner(&train, owner);
    train
}

/// `$train->setDriver($driver)`.
fn set_driver(train: &Object, driver: &Object) {
    set(train, "driver", Value::Object(driver.clone()));
    set(driver, "train", Value::Object(train.clone()));
}

/// `$train->setOwner($owner)`.
fn set_owner(train: &Object, owner: &Object) {
    set(train, "owner", Value::Object(owner.clone()));
    set(owner, "train", Value::Object(train.clone()));
}

/// `$train->addWaggon($waggon)`.
fn add_waggon(train: &Object, waggon: &Object) {
    set(waggon, "train", Value::Object(train.clone()));
    coll::add(&get(train, "waggons"), Value::Object(waggon.clone())).unwrap();
}

/// `new TrainDriver($name)`.
fn train_driver(name: &str) -> Object {
    let driver = new_object(TRAIN_DRIVER);
    set(&driver, "name", s(name));
    driver
}

/// `new TrainOwner($name)`.
fn train_owner(name: &str) -> Object {
    let owner = new_object(TRAIN_OWNER);
    set(&owner, "name", s(name));
    owner
}

/// `new TrainOrder($train)`.
fn train_order(train: &Object) -> Object {
    let order = new_object(TRAIN_ORDER);
    set(&order, "train", Value::Object(train.clone()));
    order
}

/// `$this->assertQueryCount($expectedCount)`.
#[track_caller]
fn assert_query_count(log: &QueryLog, expected_count: usize) {
    assert_eq!(expected_count, log.queries().len(), "{:#?}", log.queries());
}

/// `$this->assertSQLEquals($expectedSql, $actualSql)`.
#[track_caller]
fn assert_sql_equals(expected_sql: &str, actual_sql: &str) {
    assert_eq!(
        expected_sql.to_lowercase(),
        actual_sql.to_lowercase(),
        "Lowercase comparison of SQL statements failed."
    );
}

/// `$this->getLastLoggedQuery($index)['sql']`.
fn get_last_logged_query(log: &QueryLog, index: usize) -> String {
    let queries = log.queries();
    queries[queries.len() - 1 - index].clone()
}

async fn find(em: &Arc<EntityManager>, class: &str, id: Value) -> Option<Object> {
    em.find(class, id).await.unwrap()
}

#[tokio::test]
async fn testEagerLoadOneToOneOwningSide() {
    let (em, log) = set_up().await;
    let train = train(&train_owner("Alexander"));
    let driver = train_driver("Benjamin");
    let waggon = new_object(WAGGON);

    set_driver(&train, &driver);
    add_waggon(&train, &waggon);

    em.persist(&train).unwrap(); // cascades
    em.flush().await.unwrap();
    em.clear().unwrap();

    log.reset_and_enable();

    let train = find(&em, TRAIN, get(&train, "id")).await.unwrap();
    assert!(!em.is_uninitialized_object(&get(&train, "driver")));
    assert_eq!(
        s("Benjamin"),
        get(&orm::obj(&get(&train, "driver")), "name")
    );

    assert_query_count(&log, 1);
}

#[tokio::test]
async fn testEagerLoadOneToOneNullOwningSide() {
    let (em, log) = set_up().await;
    let train = train(&train_owner("Alexander"));

    em.persist(&train).unwrap(); // cascades
    em.flush().await.unwrap();
    em.clear().unwrap();

    log.reset_and_enable();

    let train = find(&em, TRAIN, get(&train, "id")).await.unwrap();
    assert!(get(&train, "driver").is_null());

    assert_query_count(&log, 1);
}

#[tokio::test]
async fn testEagerLoadOneToOneInverseSide() {
    let (em, log) = set_up().await;
    let owner = train_owner("Alexander");
    let train = train(&owner);

    em.persist(&train).unwrap(); // cascades
    em.flush().await.unwrap();
    em.clear().unwrap();

    log.reset_and_enable();

    let owner = find(&em, TRAIN_OWNER, get(&owner, "id")).await.unwrap();
    assert!(!em.is_uninitialized_object(&get(&owner, "train")));
    assert_eq!(TRAIN, orm::obj(&get(&owner, "train")).class());

    assert_query_count(&log, 1);
}

#[tokio::test]
async fn testEagerLoadOneToOneNullInverseSide() {
    let (em, log) = set_up().await;
    let driver = train_driver("Dagny Taggert");

    em.persist(&driver).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert!(get(&driver, "train").is_null());

    log.reset_and_enable();

    let driver = find(&em, TRAIN_DRIVER, get(&driver, "id")).await.unwrap();
    assert!(get(&driver, "train").is_null());

    assert_query_count(&log, 1);
}

#[tokio::test]
async fn testEagerLoadManyToOne() {
    let (em, _log) = set_up().await;
    let train = train(&train_owner("Alexander"));
    let waggon = new_object(WAGGON);
    add_waggon(&train, &waggon);

    em.persist(&train).unwrap(); // cascades
    em.flush().await.unwrap();
    em.clear().unwrap();

    let waggon = find(&em, WAGGON, get(&waggon, "id")).await.unwrap();
    assert!(!em.is_uninitialized_object(&get(&waggon, "train")));
    assert_eq!(TRAIN, orm::obj(&get(&waggon, "train")).class());
}

#[tokio::test]
async fn testEagerLoadWithNullableColumnsGeneratesLeftJoinOnBothSides() {
    let (em, _log) = set_up().await;
    let train = train(&train_owner("Alexander"));
    let driver = train_driver("Benjamin");

    em.persist(&train).unwrap();
    em.persist(&driver).unwrap();
    em.flush().await.unwrap();
    let train_id = get(&train, "id");
    let driver_id = get(&driver, "id");
    em.clear().unwrap();

    let train = find(&em, TRAIN, get(&train, "id")).await;
    assert!(
        train.is_some(),
        "It should be possible to find the train even though it has no driver"
    );
    assert!(train_id.identical(&get(&train.unwrap(), "id")));

    em.clear().unwrap();
    let driver = find(&em, TRAIN_DRIVER, get(&driver, "id")).await;
    assert!(
        driver.is_some(),
        "It should be possible to find the driver even though they drive no train"
    );
    assert!(driver_id.identical(&get(&driver.unwrap(), "id")));
}

#[tokio::test]
async fn testEagerLoadWithNonNullableColumnsGeneratesInnerJoinOnOwningSide() {
    let (em, log) = set_up().await;
    let waggon = new_object(WAGGON);

    // It should have a train
    let train = train(&train_owner("Alexander"));
    add_waggon(&train, &waggon);

    em.persist(&train).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    find(&em, WAGGON, get(&waggon, "id")).await.unwrap();

    // The last query is the eager loading of the owner of the train
    assert_sql_equals(
        "SELECT t0.id AS id_1, t0.name AS name_2, t3.id AS id_4, t3.driver_id AS driver_id_5, t3.owner_id AS owner_id_6 FROM TrainOwner t0 LEFT JOIN Train t3 ON t3.owner_id = t0.id WHERE t0.id IN (?)",
        &get_last_logged_query(&log, 0),
    );

    // The one before is the fetching of the waggon and train
    assert_sql_equals(
        "SELECT t0.id AS id_1, t0.train_id AS train_id_2, t3.id AS id_4, t3.driver_id AS driver_id_5, t3.owner_id AS owner_id_6 FROM Waggon t0 INNER JOIN Train t3 ON t0.train_id = t3.id WHERE t0.id = ?",
        &get_last_logged_query(&log, 1),
    );
}

#[tokio::test]
async fn testEagerLoadWithNonNullableColumnsGeneratesLeftJoinOnNonOwningSide() {
    let (em, _log) = set_up().await;
    let owner = train_owner("Alexander");
    em.persist(&owner).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let owner = find(&em, TRAIN_OWNER, get(&owner, "id")).await;
    assert!(
        owner.is_some(),
        "An owner without a train should be able to exist."
    );
}

#[tokio::test]
async fn testEagerLoadingDoesNotBreakRefresh() {
    let (em, _log) = set_up().await;
    let train = train(&train_owner("Johannes"));
    let order = train_order(&train);
    em.persist(&train).unwrap();
    em.persist(&order).unwrap();
    em.flush().await.unwrap();

    em.get_connection()
        .execute_statement(
            "UPDATE TrainOrder SET train_id = NULL",
            &PhpArray::new(),
            &ParameterTypes::new(),
        )
        .await
        .unwrap();

    assert!(same_value(&get(&order, "train"), &train));
    em.refresh(&order).await.unwrap();
    assert!(
        get(&order, "train").is_null(),
        "Train reference was not refreshed to NULL."
    );
}
