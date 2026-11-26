//! Ticket/DDC949Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the generic models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC949Test.php

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

use php_value::{PhpArray, Value};

use orm::*;

const BOOLEAN_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\BooleanModel";
const DATE_TIME_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\DateTimeModel";
const DECIMAL_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\DecimalModel";

#[tokio::test]
async fn testBooleanThroughRepository() {
    // useModelSet('generic')
    let em = orm::set_up(&[BOOLEAN_MODEL, DATE_TIME_MODEL, DECIMAL_MODEL]).await;

    let r#true = new_object(BOOLEAN_MODEL);
    set(&r#true, "booleanField", Value::Bool(true));

    let r#false = new_object(BOOLEAN_MODEL);
    set(&r#false, "booleanField", Value::Bool(false));

    em.persist(&r#true).unwrap();
    em.persist(&r#false).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let repository = em.get_repository(BOOLEAN_MODEL).unwrap();
    let find_one_by = |value: bool| {
        let mut criteria = PhpArray::new();
        criteria.insert("booleanField", Value::Bool(value));
        let repository = repository.clone();
        async move { repository.find_one_by(&criteria, None).await.unwrap() }
    };
    let r#true = find_one_by(true).await.expect("True model not found");
    let r#false = find_one_by(false).await.expect("False model not found");

    assert_eq!(BOOLEAN_MODEL, r#true.class(), "True model not found");
    assert_eq!(
        Value::Bool(true),
        get(&r#true, "booleanField"),
        "True Boolean Model should be true."
    );

    assert_eq!(BOOLEAN_MODEL, r#false.class(), "False model not found");
    assert_eq!(
        Value::Bool(false),
        get(&r#false, "booleanField"),
        "False Boolean Model should be false."
    );
}
