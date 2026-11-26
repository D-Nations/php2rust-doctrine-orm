//! Functional/IndexByAssociationTest.php on `SQLite` (`tests/common/orm.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/IndexByAssociationTest.php
//! Upstream: doctrine/orm tests/Tests/Models/StockExchange/Market.php tests/Tests/Models/StockExchange/Stock.php
//! Upstream: doctrine/orm tests/Tests/Models/StockExchange/Bond.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{get, new_object, obj, s, set};

const MARKET: &str = "Doctrine\\Tests\\Models\\StockExchange\\Market";
const STOCK: &str = "Doctrine\\Tests\\Models\\StockExchange\\Stock";
const BOND: &str = "Doctrine\\Tests\\Models\\StockExchange\\Bond";

/// `new Market($name)`.
fn market(name: &str) -> Object {
    let market = new_object(MARKET);
    set(&market, "name", s(name));
    set(
        &market,
        "stocks",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    market
}

/// `$this->stocks[$stock->getSymbol()] = $stock` (Market and Bond's
/// `addStock()`).
async fn add_stock(owner: &Object, stock: &Object) {
    let symbol = get(stock, "symbol");
    let key = symbol
        .to_php_string()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    match get(owner, "stocks") {
        Value::Null => {
            let mut stocks = PhpArray::new();
            stocks.insert(key.as_str(), Value::Object(stock.clone()));
            set(owner, "stocks", Value::Array(stocks));
        }
        Value::Array(mut stocks) => {
            stocks.insert(key.as_str(), Value::Object(stock.clone()));
            set(owner, "stocks", Value::Array(stocks));
        }
        stocks => coll::set(&stocks, key.as_str(), Value::Object(stock.clone()))
            .await
            .unwrap(),
    }
}

/// `new Stock($symbol, $price, $market)`.
async fn stock(symbol: &str, price: f64, market: &Object) -> Object {
    let stock = new_object(STOCK);
    set(&stock, "symbol", s(symbol));
    set(&stock, "market", Value::Object(market.clone()));
    // (string) $price
    set(
        &stock,
        "price",
        Value::String(Value::Float(price).to_php_string().unwrap()),
    );
    add_stock(market, &stock).await;
    stock
}

/// `new Bond($name)`.
fn bond(name: &str) -> Object {
    let bond = new_object(BOND);
    set(&bond, "name", s(name));
    bond
}

struct Fixture {
    em: Arc<EntityManager>,
    market_id: Value,
    bond_id: Value,
}

async fn set_up() -> Fixture {
    let em = orm::set_up(&[MARKET, STOCK, BOND]).await;

    let market = market("Some Exchange");
    let stock1 = stock("AAPL", 10.0, &market).await;
    let stock2 = stock("GOOG", 20.0, &market).await;

    let bond = bond("MyBond");
    add_stock(&bond, &stock1).await;
    add_stock(&bond, &stock2).await;

    em.persist(&market).unwrap();
    em.persist(&stock1).unwrap();
    em.persist(&stock2).unwrap();
    em.persist(&bond).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    Fixture {
        em,
        market_id: get(&market, "id"),
        bond_id: get(&bond, "id"),
    }
}

/// The asserts on `$owner->stocks` indexed by symbol.
async fn assert_indexed_by_symbol(owner: &Object) {
    let stocks = get(owner, "stocks");
    assert_eq!(coll::count(&stocks).await.unwrap(), 2);
    for symbol in ["AAPL", "GOOG"] {
        assert!(
            coll::contains_key(&stocks, symbol).await.unwrap(),
            "{symbol} symbol has to be key in indexed association."
        );
        let stock = obj(&coll::get(&stocks, symbol).await.unwrap());
        assert_eq!(get(&stock, "symbol"), s(symbol));
    }
}

async fn single_result(em: &Arc<EntityManager>, dql: &str, id: &Value) -> Object {
    let query = em.create_query(dql);
    query.set_parameter(1, id.clone(), None);
    obj(&query.get_single_result(None).await.unwrap())
}

#[tokio::test]
async fn testManyToOneFinder() {
    let f = set_up().await;
    let market =
        f.em.find(MARKET, f.market_id.clone())
            .await
            .unwrap()
            .unwrap();

    assert_indexed_by_symbol(&market).await;
}

#[tokio::test]
async fn testManyToOneDQL() {
    let f = set_up().await;
    let dql = "SELECT m, s FROM Doctrine\\Tests\\Models\\StockExchange\\Market m JOIN m.stocks s WHERE m.id = ?1";
    let market = single_result(&f.em, dql, &f.market_id).await;

    assert_indexed_by_symbol(&market).await;
}

#[tokio::test]
async fn testManyToMany() {
    let f = set_up().await;
    let bond = f.em.find(BOND, f.bond_id.clone()).await.unwrap().unwrap();

    assert_indexed_by_symbol(&bond).await;
}

#[tokio::test]
async fn testManytoManyDQL() {
    let f = set_up().await;
    let dql = "SELECT b, s FROM Doctrine\\Tests\\Models\\StockExchange\\Bond b JOIN b.stocks s WHERE b.id = ?1";
    let bond = single_result(&f.em, dql, &f.bond_id).await;

    assert_indexed_by_symbol(&bond).await;
}

#[tokio::test]
async fn testDqlOverrideIndexBy() {
    let f = set_up().await;
    let dql = "SELECT b, s FROM Doctrine\\Tests\\Models\\StockExchange\\Bond b JOIN b.stocks s INDEX BY s.id WHERE b.id = ?1";
    let bond = single_result(&f.em, dql, &f.bond_id).await;

    let stocks = get(&bond, "stocks");
    assert_eq!(coll::count(&stocks).await.unwrap(), 2);
    assert!(
        !coll::contains_key(&stocks, "AAPL").await.unwrap(),
        "AAPL symbol not exists in re-indexed association."
    );
    assert!(
        !coll::contains_key(&stocks, "GOOG").await.unwrap(),
        "GOOG symbol not exists in re-indexed association."
    );
}
