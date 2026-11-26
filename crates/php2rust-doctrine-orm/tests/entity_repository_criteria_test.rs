//! EntityRepositoryCriteriaTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! `new DateTime('today')` is midnight UTC (the tests' time zone);
//! `$dates[0]` is `get(0)`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/EntityRepositoryCriteriaTest.php

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

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_orm::collection_object::{
    self as coll, ArrayCollectionObject, CollectionRef,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

const DATE_TIME_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\DateTimeModel";
const TWEET: &str = "Doctrine\\Tests\\Models\\Tweet\\Tweet";
const TWEET_USER: &str = "Doctrine\\Tests\\Models\\Tweet\\User";
const TWEET_USER_LIST: &str = "Doctrine\\Tests\\Models\\Tweet\\UserList";

/// `useModelSet('generic')` and `useModelSet('tweet')`: the generic
/// models these tests use (the other ones are not recorded).
async fn set_up_models() -> Arc<EntityManager> {
    set_up(&[DATE_TIME_MODEL, TWEET_USER, TWEET, TWEET_USER_LIST]).await
}

/// `new DateTime('today')`, `'tomorrow'`, `'yesterday'`: midnight UTC,
/// `days` from today.
fn date(days: i64) -> Value {
    let now = DateTimeImmutable::now(TimeZone::Utc).timestamp();
    let today = now - now.rem_euclid(86_400);
    DateTime::object(DateTimeImmutable::from_timestamp(
        today + days * 86_400,
        0,
        TimeZone::Utc,
    ))
}

fn date_time_model(datetime: &Value, date: &Value, time: &Value) -> Object {
    let model = new_object(DATE_TIME_MODEL);
    set(&model, "datetime", datetime.clone());
    set(&model, "date", date.clone());
    set(&model, "time", time.clone());
    model
}

async fn load_fixture(em: &EntityManager) {
    let today = date(0);
    em.persist(&date_time_model(&today, &today, &today))
        .unwrap();

    let tomorrow = date(1);
    em.persist(&date_time_model(&tomorrow, &tomorrow, &tomorrow))
        .unwrap();

    let yesterday = date(-1);
    em.persist(&date_time_model(&yesterday, &yesterday, &yesterday))
        .unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
}

fn criteria_where(
    expression: impl Into<php2rust_doctrine_collections::expr::Expression>,
) -> Criteria {
    let mut criteria = Criteria::create();
    criteria.r#where(expression);
    criteria
}

fn matching(em: &Arc<EntityManager>, criteria: &Criteria) -> Value {
    em.get_repository(DATE_TIME_MODEL)
        .unwrap()
        .matching(criteria)
        .unwrap()
        .value()
}

async fn count(collection: &Value) -> usize {
    coll::count(collection).await.unwrap()
}

#[tokio::test]
async fn testLteDateComparison() {
    let em = set_up_models().await;
    load_fixture(&em).await;

    let dates = matching(
        &em,
        &criteria_where(Criteria::expr().lte("datetime", date(0))),
    );

    assert_eq!(count(&dates).await, 2);
}

async fn load_null_field_fixtures(em: &EntityManager) {
    let today = date(0);
    em.persist(&date_time_model(&today, &today, &Value::Null))
        .unwrap();

    let tomorrow = date(1);
    em.persist(&date_time_model(&tomorrow, &tomorrow, &tomorrow))
        .unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
}

#[tokio::test]
async fn testIsNullComparison() {
    let em = set_up_models().await;
    load_null_field_fixtures(&em).await;

    let dates = matching(&em, &criteria_where(Criteria::expr().is_null("time")));

    assert_eq!(count(&dates).await, 1);
}

#[tokio::test]
async fn testEqNullComparison() {
    let em = set_up_models().await;
    load_null_field_fixtures(&em).await;

    let dates = matching(
        &em,
        &criteria_where(Criteria::expr().eq("time", Value::Null)),
    );

    assert_eq!(count(&dates).await, 1);
}

#[tokio::test]
async fn testNotEqNullComparison() {
    let em = set_up_models().await;
    load_null_field_fixtures(&em).await;

    let dates = matching(
        &em,
        &criteria_where(Criteria::expr().neq("time", Value::Null)),
    );

    assert_eq!(count(&dates).await, 1);
}

#[tokio::test]
async fn testCanCountWithoutLoadingCollection() {
    let em = set_up_models().await;
    load_fixture(&em).await;
    let repository = em.get_repository(DATE_TIME_MODEL).unwrap();

    let dates = repository.matching(&Criteria::create()).unwrap();

    assert!(!dates.is_initialized());
    assert_eq!(dates.count().await.unwrap(), 3);
    assert!(!dates.is_initialized());

    // Test it can work even with a constraint
    let dates = repository
        .matching(&criteria_where(Criteria::expr().lte("datetime", date(0))))
        .unwrap();

    assert!(!dates.is_initialized());
    assert_eq!(dates.count().await.unwrap(), 2);
    assert!(!dates.is_initialized());

    // Trigger a loading, to make sure collection is initialized
    let _date = dates.get(&0.into()).await.unwrap();
    assert!(dates.is_initialized());
}

#[tokio::test]
async fn testCanContainsWithoutLoadingCollection() {
    let em = set_up_models().await;
    let user = new_object(TWEET_USER);
    set(
        &user,
        "tweets",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(
        &user,
        "userLists",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&user, "name", s("Marco"));
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let tweet = new_object(TWEET);
    set_obj(&tweet, "author", &user);
    set(&tweet, "content", s("Criteria is awesome"));
    em.persist(&tweet).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let mut criteria = Criteria::create();
    criteria
        .and_where(Criteria::expr().contains("content", s("Criteria")))
        .unwrap();

    let user = em
        .find(TWEET_USER, get(&user, "id"))
        .await
        .unwrap()
        .unwrap();
    let tweets = coll::matching(&get(&user, "tweets"), &criteria)
        .await
        .unwrap();

    let Some(CollectionRef::Lazy(tweets)) = CollectionRef::of(&tweets) else {
        panic!("a LazyCriteriaCollection expected");
    };
    assert!(!tweets.is_initialized());

    let tweet = Value::Object(tweet);
    tweets.contains(&tweet).await.unwrap();
    assert!(tweets.contains(&tweet).await.unwrap());

    assert!(!tweets.is_initialized());
}
