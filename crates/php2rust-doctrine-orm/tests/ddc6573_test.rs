//! Ticket/DDC6573Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the Models/DDC6573 models).
//!
//! The `NEW` classes' constructors are registered with
//! `register_new_object_class()`. A data provider's rows run in one
//! test. `tearDown()`'s DELETE is left out: every test has a fresh
//! database.
//!
//! Upstream: doctrine/orm tests/Tests/Models/DDC6573/DDC6573Money.php tests/Tests/Models/DDC6573/DDC6573Currency.php
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC6573Test.php

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::internal::hydration::register_new_object_class;
use php2rust_doctrine_orm::query::context::{HYDRATE_ARRAY, HYDRATE_OBJECT};

use orm::*;

const ITEM: &str = "Doctrine\\Tests\\Models\\DDC6573\\DDC6573Item";
const MONEY: &str = "Doctrine\\Tests\\Models\\DDC6573\\DDC6573Money";
const CURRENCY: &str = "Doctrine\\Tests\\Models\\DDC6573\\DDC6573Currency";

/// `new DDC6573Currency($code)`.
fn currency(code: Value) -> Object {
    let currency = new_object(CURRENCY);
    set(&currency, "code", code);
    currency
}

/// `new DDC6573Money($amount, $currency)`.
fn money(amount: Value, currency: Value) -> Object {
    let money = new_object(MONEY);
    set(&money, "amount", amount);
    set(&money, "currency", currency);
    money
}

/// `new DDC6573Item($name, $price)`.
fn item(name: &str, price: &Object) -> Object {
    let item = new_object(ITEM);
    set(&item, "name", s(name));
    set(&item, "priceAmount", get(price, "amount"));
    set(
        &item,
        "priceCurrency",
        get(&obj(&get(price, "currency")), "code"),
    );
    item
}

/// `DDC6573Item::getPrice()`.
fn get_price(item: &Object) -> Object {
    money(
        get(item, "priceAmount"),
        Value::Object(currency(get(item, "priceCurrency"))),
    )
}

/// `assertEquals()` of two `DDC6573Currency` objects.
fn assert_currency_equals(expected: &Object, actual: &Value) {
    assert_eq!(get(expected, "code"), get(&obj(actual), "code"));
}

/// `assertEquals()` of two `DDC6573Money` objects.
fn assert_money_equals(expected: &Object, actual: &Value) {
    let actual = obj(actual);
    assert_eq!(get(expected, "amount"), get(&actual, "amount"));
    assert_currency_equals(&obj(&get(expected, "currency")), &get(&actual, "currency"));
}

async fn set_up() -> (Arc<EntityManager>, Vec<Object>) {
    register_new_object_class(
        MONEY,
        Arc::new(|args| {
            let mut args = args.into_iter();
            let amount = args.next().unwrap_or_default();
            let currency = args.next().unwrap_or_default();
            Ok(Value::Object(money(amount, currency)))
        }),
    );
    register_new_object_class(
        CURRENCY,
        Arc::new(|args| {
            let code = args.into_iter().next().unwrap_or_default();
            Ok(Value::Object(currency(code)))
        }),
    );

    let em = orm::set_up(&[ITEM]).await;

    let item1 = item(
        "Plate",
        &money(Value::Int(5), Value::Object(currency(s("GBP")))),
    );
    let item2 = item(
        "Iron",
        &money(Value::Int(50), Value::Object(currency(s("EUR")))),
    );
    let item3 = item(
        "Teapot",
        &money(Value::Int(10), Value::Object(currency(s("GBP")))),
    );

    em.persist(&item1).unwrap();
    em.persist(&item2).unwrap();
    em.persist(&item3).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    (em, vec![item1, item2, item3])
}

/// `provideDataForHydrationMode()`.
const HYDRATION_MODES: [i64; 2] = [HYDRATE_ARRAY, HYDRATE_OBJECT];

#[tokio::test]
async fn testShouldSupportsMultipleNewOperator() {
    for hydration_mode in HYDRATION_MODES {
        let (em, fixtures) = set_up().await;
        let dql = "
            SELECT
                new Doctrine\\Tests\\Models\\DDC6573\\DDC6573Money(
                    i.priceAmount,
                    new Doctrine\\Tests\\Models\\DDC6573\\DDC6573Currency(i.priceCurrency)
                )
            FROM
                Doctrine\\Tests\\Models\\DDC6573\\DDC6573Item i
            ORDER BY
                i.priceAmount ASC";

        let query = em.create_query(dql);
        let result = list(&query.get_result(hydration_mode).await.unwrap());

        assert_eq!(3, result.len());

        for row in &result {
            assert!(matches!(row, Value::Object(o) if o.instance_of(MONEY)));
        }

        assert_money_equals(&get_price(&fixtures[0]), &result[0]);
        assert_money_equals(&get_price(&fixtures[2]), &result[1]);
        assert_money_equals(&get_price(&fixtures[1]), &result[2]);
    }
}

#[tokio::test]
async fn testShouldSupportsBasicUsage() {
    for hydration_mode in HYDRATION_MODES {
        let (em, fixtures) = set_up().await;
        let dql = "
            SELECT
                new Doctrine\\Tests\\Models\\DDC6573\\DDC6573Currency(
                    i.priceCurrency
                )
            FROM
                Doctrine\\Tests\\Models\\DDC6573\\DDC6573Item i
            ORDER BY
                i.priceAmount";

        let query = em.create_query(dql);
        let result = list(&query.get_result(hydration_mode).await.unwrap());

        assert_eq!(3, result.len());

        for row in &result {
            assert!(matches!(row, Value::Object(o) if o.instance_of(CURRENCY)));
        }

        let currency_of = |item: &Object| obj(&get(&get_price(item), "currency"));
        assert_currency_equals(&currency_of(&fixtures[0]), &result[0]);
        assert_currency_equals(&currency_of(&fixtures[1]), &result[2]);
        assert_currency_equals(&currency_of(&fixtures[2]), &result[1]);
    }
}
