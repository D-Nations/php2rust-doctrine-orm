//! Ticket/DDC2494Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models, with a logging connection (`tests/common/query_log.rs`);
//! `DDC2494TinyIntType` is a Rust type whose `$calls` count the calls by
//! method. `getId()` on the proxy reads its identifier, `getTemp()`
//! initializes it).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2494Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use php_value::{PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type};

use orm::*;
use ticket::*;

/// `DDC2494TinyIntType::$calls`: the returned values by method.
static CALLS: Mutex<Option<HashMap<&'static str, Vec<Value>>>> = Mutex::new(None);

fn record(function: &'static str, value: Value) {
    CALLS
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .entry(function)
        .or_default()
        .push(value);
}

fn calls(function: &str) -> Option<usize> {
    CALLS
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|calls| calls.get(function).map(Vec::len))
}

/// `DDC2494TinyIntType extends Type`.
#[derive(Debug)]
struct DDC2494TinyIntType;

impl Type for DDC2494TinyIntType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC2494TinyIntType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_small_int_type_declaration_sql(column)
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let r#return = Value::String(value.to_php_string().unwrap_or_default());
        record("convertToDatabaseValue", r#return.clone());
        Ok(r#return)
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let r#return = Value::Int(php_value::ops::to_int(value));
        record("convertToPHPValue", r#return.clone());
        Ok(r#return)
    }
}

#[tokio::test]
async fn testIssue() {
    *CALLS.lock().unwrap() = Some(HashMap::new());
    ticket::add_type("ddc2494_tinyint", Arc::new(DDC2494TinyIntType));
    let classes = [class("DDC2494Currency"), class("DDC2494Campaign")];
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    let (em, log) = query_log::set_up_logged(&classes).await;

    let currency = new("DDC2494Currency");
    set(&currency, "id", Value::Int(1));
    set(&currency, "temp", Value::Int(2));

    em.persist(&currency).unwrap();
    em.flush().await.unwrap();

    let campaign = new("DDC2494Campaign");
    set_obj(&campaign, "currency", &currency);

    em.persist(&campaign).unwrap();
    em.flush().await.unwrap();
    em.close();

    assert_eq!(Some(3), calls("convertToDatabaseValue"));

    let item = find(&em, "DDC2494Campaign", get(&campaign, "id"))
        .await
        .expect("the campaign");

    assert_instance_of("DDC2494Campaign", &Value::Object(item.clone()));
    let item_currency = get(&item, "currency");
    assert_instance_of("DDC2494Currency", &item_currency);

    log.reset_and_enable();

    assert!(is_uninitialized(&em, &item_currency));
    assert_eq!(Some(1), calls("convertToPHPValue"));

    assert!(matches!(get(&obj(&item_currency), "id"), Value::Int(_)));
    assert_eq!(Some(1), calls("convertToPHPValue"));
    assert!(is_uninitialized(&em, &item_currency));
    assert_eq!(0, log.queries().len());

    em.initialize_object(&obj(&item_currency)).await.unwrap();
    assert!(matches!(get(&obj(&item_currency), "temp"), Value::Int(_)));
    assert_eq!(Some(3), calls("convertToPHPValue"));
    assert!(!is_uninitialized(&em, &item_currency));
    assert_eq!(1, log.queries().len());
}
