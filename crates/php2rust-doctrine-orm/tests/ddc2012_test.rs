//! Ticket/DDC2012Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `DDC2012TsVectorType` is a Rust type whose `$calls` count
//! the calls by method; the platform argument is not kept).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2012Test.php

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

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use php_value::{PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type};

use orm::*;
use ticket::*;

/// `DDC2012TsVectorType::$calls`: the values (or SQL expressions) by
/// method.
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

/// `DDC2012TsVectorType extends Type`.
#[derive(Debug)]
struct DDC2012TsVectorType;

impl DDC2012TsVectorType {
    const MYTYPE: &str = "tsvector";
}

impl Type for DDC2012TsVectorType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC2012TsVectorType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let value = match value {
            Value::Array(words) => {
                let words: Vec<String> = words
                    .values()
                    .map(|word| {
                        word.to_php_string()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    })
                    .collect();
                Value::from(words.join(" ").as_str())
            }
            other => other.clone(),
        };
        record("convertToDatabaseValue", value.clone());
        Ok(value)
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        record("convertToPHPValue", value.clone());
        let text = value
            .to_php_string()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        Ok(Value::Array(PhpArray::list(
            text.split(' ').map(Value::from),
        )))
    }

    fn convert_to_database_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        record(
            "convertToDatabaseValueSQL",
            Value::String(PhpString::from(sql_expr)),
        );
        // changed to upper expression to keep the test compatible with other Databases
        //sprintf('to_tsvector(%s)', $sqlExpr);
        PhpString::from(format!("UPPER({})", String::from_utf8_lossy(sql_expr)))
    }
}

fn words(words: &[&str]) -> Value {
    Value::Array(PhpArray::list(words.iter().map(|word| s(word))))
}

#[tokio::test]
async fn testIssue() {
    ticket::add_type(DDC2012TsVectorType::MYTYPE, Arc::new(DDC2012TsVectorType));
    *CALLS.lock().unwrap() = Some(HashMap::new());
    let em = ticket::set_up(&["DDC2012Item", "DDC2012ItemPerson"]).await;

    let item = new("DDC2012ItemPerson");
    set(&item, "tsv", words(&["word1", "word2", "word3"]));

    em.persist(&item).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let item = em
        .find(item.class(), get(&item, "id"))
        .await
        .unwrap()
        .expect("the item");

    assert_eq!(Some(1), calls("convertToDatabaseValueSQL"));
    assert_eq!(Some(1), calls("convertToDatabaseValue"));
    assert_eq!(Some(1), calls("convertToPHPValue"));

    assert_instance_of("DDC2012Item", &Value::Object(item.clone()));
    assert_loose_eq(&words(&["word1", "word2", "word3"]), &get(&item, "tsv"));

    set(&item, "tsv", words(&["word1", "word2"]));

    em.persist(&item).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let item = em
        .find(item.class(), get(&item, "id"))
        .await
        .unwrap()
        .expect("the item");

    assert_eq!(Some(2), calls("convertToDatabaseValueSQL"));
    assert_eq!(Some(2), calls("convertToDatabaseValue"));
    assert_eq!(Some(2), calls("convertToPHPValue"));

    assert_instance_of("DDC2012Item", &Value::Object(item.clone()));
    assert_loose_eq(&words(&["word1", "word2"]), &get(&item, "tsv"));
}
