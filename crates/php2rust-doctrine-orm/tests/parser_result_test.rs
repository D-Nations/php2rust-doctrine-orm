//! Port of `ParserResultTest`. The stub executor of `testSetGetSqlExecutor`
//! is a `SingleSelectExecutor` of one statement; `assertSame()` compares
//! the executor's kind and statements.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/ParserResultTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/dql.rs"]
mod dql;
#[path = "common/models.rs"]
mod models;

use php_value::{Key, PhpArray, Value};
use php2rust_doctrine_orm::query::ParserResult;
use php2rust_doctrine_orm::query::exec::{ExecutorKind, SqlExecutor, SqlStatements};
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;

use dql::{ExpectDeprecation, thrown};

#[test]
fn testGetRsm() {
    let parser_result = ParserResult::new();
    // `getResultSetMapping()` is typed: it is a `ResultSetMapping`.
    let _ = parser_result.get_result_set_mapping();
}

#[test]
fn testItThrowsWhenAttemptingToAccessTheExecutorBeforeItIsSet() {
    let parser_result = ParserResult::new();
    let error = parser_result.get_sql_executor().unwrap_err();
    assert!(
        thrown_is_a(&error, "LogicException"),
        "{:?}",
        thrown(&error)
    );
    assert_eq!(
        "Executor not set yet. Call Doctrine\\ORM\\Query\\ParserResult::setSqlExecutor() first.",
        thrown(&error).1
    );
}

#[test]
fn testSetGetSqlExecutor() {
    let _deprecation = ExpectDeprecation::new("https://github.com/doctrine/orm/pull/11188");
    let mut parser_result = ParserResult::new();
    let executor = SqlExecutor::new(
        ExecutorKind::SingleSelect,
        SqlStatements::One("SELECT 1".to_owned()),
    );
    parser_result.set_sql_executor(executor.clone());
    assert_eq!(&executor, parser_result.get_sql_executor().unwrap());
}

#[test]
fn testGetSqlParameterPosition() {
    let mut parser_result = ParserResult::new();
    parser_result.add_parameter_mapping(Key::Int(1), 1);
    parser_result.add_parameter_mapping(Key::Int(1), 2);
    assert_eq!(
        Some(vec![1, 2]),
        parser_result.get_sql_parameter_positions(&Key::Int(1))
    );
}

#[test]
fn testGetParameterMappings() {
    let mut parser_result = ParserResult::new();
    // `getParameterMappings()` is typed: it is an array.
    assert!(parser_result.get_parameter_mappings().is_empty());

    parser_result.add_parameter_mapping(Key::Int(1), 1);
    parser_result.add_parameter_mapping(Key::Int(1), 2);
    let expected: PhpArray = [(
        1,
        Value::Array(PhpArray::list([Value::Int(1), Value::Int(2)])),
    )]
    .into_iter()
    .collect();
    assert_eq!(&expected, parser_result.get_parameter_mappings());
}
