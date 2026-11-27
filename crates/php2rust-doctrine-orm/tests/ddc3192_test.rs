//! Ticket/DDC3192Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `DDC3192CurrencyCode` is a Rust type).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3192Test.php

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

use php_value::{PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type};

use orm::*;
use ticket::*;

/// `DDC3192CurrencyCode extends Type`.
#[derive(Debug)]
struct DDC3192CurrencyCode;

impl DDC3192CurrencyCode {
    /// `self::$map`.
    const MAP: [(&str, i64); 1] = [("BYR", 974)];
}

impl Type for DDC3192CurrencyCode {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC3192CurrencyCode"
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
        let code = value.to_php_string().unwrap_or_default();
        let (_, number) = Self::MAP
            .iter()
            .find(|(key, _)| key.as_bytes() == code.as_bytes())
            .expect("a mapped currency code");
        Ok(Value::Int(*number))
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let number = php_value::ops::to_int(value);
        Ok(Self::MAP
            .iter()
            .find(|(_, mapped)| *mapped == number)
            .map_or(Value::Bool(false), |(key, _)| s(key)))
    }
}

#[tokio::test]
async fn testIssue() {
    assert!(
        !php2rust_doctrine_dbal::types::has_type("ddc3192_currency_code"),
        "Type ddc3192_currency_code exists for testing DDC-3192 only, but it has already been registered for some reason"
    );
    ticket::add_type("ddc3192_currency_code", Arc::new(DDC3192CurrencyCode));
    let em = ticket::set_up(&["DDC3192Currency", "DDC3192Transaction"]).await;

    let currency = new("DDC3192Currency");
    set(&currency, "code", s("BYR"));

    em.persist(&currency).unwrap();
    em.flush().await.unwrap();

    let amount = 50;
    let transaction = new("DDC3192Transaction");
    set(&transaction, "amount", Value::Int(amount));
    set_obj(&transaction, "currency", &currency);

    em.persist(&transaction).unwrap();
    em.flush().await.unwrap();
    em.close();

    let result_by_persister = find(&em, "DDC3192Transaction", get(&transaction, "id"))
        .await
        .expect("the transaction");

    // This works: DDC2494 makes persister set type mapping info to ResultSetMapping
    let currency = get_object(&result_by_persister, "currency").unwrap();
    assert_loose_eq(&s("BYR"), &get(&currency, "code"));

    em.close();

    let query = em.create_query(&format!(
        "SELECT t FROM {} t WHERE t.id = ?1",
        class("DDC3192Transaction")
    ));
    query.set_parameter(1, get(&transaction, "id"), None);

    let result_by_query = obj(&query.get_single_result(None).await.unwrap());

    // This is fixed here: before the fix it used to return 974.
    // because unlike the BasicEntityPersister, SQLWalker doesn't set type info
    let currency = get_object(&result_by_query, "currency").unwrap();
    assert_loose_eq(&s("BYR"), &get(&currency, "code"));
}
