//! Ticket/DDC1335Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! The `UniqueConstraintViolationException` catch of `setUp()` has
//! nothing to catch: every test has a fresh database.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1335Test.php

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

use php_value::{Key, Object, PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

/// `new DDC1335User($email, $name, $numbers)`.
fn user(email: &str, name: &str, numbers: &[&str]) -> Object {
    let user = new("DDC1335User");
    set(&user, "email", s(email));
    set(&user, "name", s(name));
    set(
        &user,
        "phones",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    for number in numbers {
        // new DDC1335Phone($this, $number)
        let phone = new("DDC1335Phone");
        set_obj(&phone, "user", &user);
        set(&phone, "numericalValue", s(number));
        coll::add(&get(&user, "phones"), Value::Object(phone)).unwrap();
    }
    user
}

async fn set_up() -> Arc<EntityManager> {
    let em = ticket::set_up(&["DDC1335User", "DDC1335Phone"]).await;
    load_fixture(&em).await;
    em
}

async fn load_fixture(em: &EntityManager) {
    let p1 = ["11 xxxx-xxxx", "11 yyyy-yyyy", "11 zzzz-zzzz"];
    let p2 = ["22 xxxx-xxxx", "22 yyyy-yyyy", "22 zzzz-zzzz"];
    let p3 = ["33 xxxx-xxxx", "33 yyyy-yyyy", "33 zzzz-zzzz"];

    let u1 = user("foo@foo.com", "Foo", &p1);
    let u2 = user("bar@bar.com", "Bar", &p2);
    let u3 = user("foobar@foobar.com", "Foo Bar", &p3);

    em.persist(&u1).unwrap();
    em.persist(&u2).unwrap();
    em.persist(&u3).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
}

fn assert_array_has_key(key: impl Into<Key>, array: &Value) {
    let key = key.into();
    let Value::Array(array) = array else {
        panic!("{array:?} is not an array");
    };
    assert!(array.get(&key).is_some(), "no key {key:?}");
}

fn count(array: &Value) -> usize {
    list(array).len()
}

/// `$result[$email]->phones`.
fn phones(result: &Value, email: &str) -> Value {
    get(&obj(&at(result, email)), "phones")
}

async fn assert_phones_by_id(result: &Value) {
    for (email, ids) in [
        ("foo@foo.com", [1, 2, 3]),
        ("bar@bar.com", [4, 5, 6]),
        ("foobar@foobar.com", [7, 8, 9]),
    ] {
        let phones = phones(result, email);
        assert_eq!(3, coll::count(&phones).await.unwrap());
        let array = Value::Array(coll::to_array(&phones).await.unwrap());
        for id in ids {
            assert_array_has_key(id, &array);
        }
    }
}

#[tokio::test]
async fn testDql() {
    let em = set_up().await;
    let dql = format!("SELECT u FROM {NS}DDC1335User u INDEX BY u.id");
    let query = em.create_query(&dql);
    let result = query.get_result(HYDRATE_OBJECT).await.unwrap();

    assert_eq!(3, count(&result));
    assert_array_has_key(1, &result);
    assert_array_has_key(2, &result);
    assert_array_has_key(3, &result);

    let dql = format!(
        "SELECT u, p FROM {NS}DDC1335User u INDEX BY u.email INNER JOIN u.phones p INDEX BY p.id"
    );
    let query = em.create_query(&dql);
    let result = query.get_result(HYDRATE_OBJECT).await.unwrap();

    assert_eq!(3, count(&result));
    assert_array_has_key("foo@foo.com", &result);
    assert_array_has_key("bar@bar.com", &result);
    assert_array_has_key("foobar@foobar.com", &result);

    assert_phones_by_id(&result).await;
}

#[tokio::test]
async fn testTicket() {
    let em = set_up().await;
    let mut builder = em.create_query_builder();
    builder
        .select(vec![s("u")])
        .unwrap()
        .from(&class("DDC1335User"), "u", Some("u.id"))
        .unwrap();

    let dql = builder.get_query().unwrap().get_dql();
    let result = builder
        .get_query()
        .unwrap()
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    assert_eq!(3, count(&result));
    assert_array_has_key(1, &result);
    assert_array_has_key(2, &result);
    assert_array_has_key(3, &result);
    assert_eq!(
        Some(format!("SELECT u FROM {NS}DDC1335User u INDEX BY u.id")),
        dql
    );
}

#[tokio::test]
async fn testIndexByUnique() {
    let em = set_up().await;
    let mut builder = em.create_query_builder();
    builder
        .select(vec![s("u")])
        .unwrap()
        .from(&class("DDC1335User"), "u", Some("u.email"))
        .unwrap();

    let dql = builder.get_query().unwrap().get_dql();
    let result = builder
        .get_query()
        .unwrap()
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    assert_eq!(3, count(&result));
    assert_array_has_key("foo@foo.com", &result);
    assert_array_has_key("bar@bar.com", &result);
    assert_array_has_key("foobar@foobar.com", &result);
    assert_eq!(
        Some(format!("SELECT u FROM {NS}DDC1335User u INDEX BY u.email")),
        dql
    );
}

#[tokio::test]
async fn testIndexWithJoin() {
    let em = set_up().await;
    let mut builder = em.create_query_builder();
    builder
        .select(vec![s("u"), s("p")])
        .unwrap()
        .from(&class("DDC1335User"), "u", Some("u.email"))
        .unwrap()
        .join("u.phones", "p", None, None, Some("p.id"))
        .unwrap();

    let dql = builder.get_query().unwrap().get_dql();
    let result = builder
        .get_query()
        .unwrap()
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    assert_eq!(3, count(&result));
    assert_array_has_key("foo@foo.com", &result);
    assert_array_has_key("bar@bar.com", &result);
    assert_array_has_key("foobar@foobar.com", &result);

    assert_phones_by_id(&result).await;

    assert_eq!(
        Some(format!(
            "SELECT u, p FROM {NS}DDC1335User u INDEX BY u.email INNER JOIN u.phones p INDEX BY p.id"
        )),
        dql
    );
}
