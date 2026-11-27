//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2224Test.php
//!
//! On `SQLite`, with the recorded `DDC2224Entity` model; `DDC2224Type` is a
//! Rust type. The `#[Depends('testIssue')]` test runs `testIssue` first
//! and takes its query.

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
use php2rust_doctrine_orm::cache::pool::ArrayAdapter;
use php2rust_doctrine_orm::query::dql_query::Query;

use orm::s;

/// `DDC2224Type extends Type`.
#[derive(Debug)]
struct DDC2224Type;

impl Type for DDC2224Type {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC2224Type"
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
        Ok(value.clone())
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(value.clone())
    }

    fn convert_to_database_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        let mut out = b"FUNCTION(".to_vec();
        out.extend_from_slice(sql_expr);
        out.push(b')');
        PhpString::from(out)
    }
}

/// `setUpBeforeClass()`.
fn set_up_before_class() {
    ticket::add_type("DDC2224Type", Arc::new(DDC2224Type));
}

/// `testIssue()`, which returns its query.
fn test_issue() -> (
    Arc<php2rust_doctrine_orm::entity_manager::EntityManager>,
    Query,
) {
    set_up_before_class();
    let em = orm::entity_manager(orm::connection(), None);
    let dql = format!(
        "SELECT e FROM {}DDC2224Entity e WHERE e.field = :field",
        ticket::NS
    );
    let query = em.create_query(&dql);
    query.set_query_cache(Some(Arc::new(ArrayAdapter::new())));

    query.set_parameter("field", s("test"), Some("DDC2224Type".into()));
    let sql = query.get_single_sql().unwrap();
    assert!(sql.ends_with(".field = FUNCTION(?)"), "{sql}");

    (em, query)
}

#[tokio::test]
async fn testIssue() {
    test_issue();
}

#[tokio::test]
async fn testCacheMissWhenTypeChanges() {
    let (_em, query) = test_issue();
    query.set_parameter("field", s("test"), Some("string".into()));
    let sql = query.get_single_sql().unwrap();
    assert!(sql.ends_with(".field = ?"), "{sql}");
}
