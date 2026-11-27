//! Ticket/GH8061Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; upstream's `OrmTestCase` with `getTestEntityManager()`).
//! `GH8061Type` is a Rust type; the regular expression is matched by hand.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH8061Test.php

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

use php_value::{PhpArray, PhpString};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::Type;
use php2rust_doctrine_orm::query::exec::SqlStatements;

use ticket::*;

/// `GH8061Type extends Type`.
#[derive(Debug)]
struct GH8061Type;

impl Type for GH8061Type {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH8061Type"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }

    fn convert_to_php_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        PhpString::from(format!(
            "DatabaseFunction({})",
            String::from_utf8_lossy(sql_expr)
        ))
    }
}

/// `/SELECT DatabaseFunction\(\w+\.field\) AS /`.
fn matches(sql: &str) -> bool {
    sql.match_indices("SELECT DatabaseFunction(")
        .any(|(start, prefix)| {
            let rest = &sql[start + prefix.len()..];
            let word = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            word > 0 && rest[word..].starts_with(".field) AS ")
        })
}

#[tokio::test]
async fn testConvertToPHPValueSQLForNewObjectExpression() {
    ticket::add_type("GH8061Type", Arc::new(GH8061Type));
    let dql = format!(
        "SELECT NEW {}(e.field) FROM {} e",
        class("GH8061Class"),
        class("GH8061Entity")
    );
    let entity_manager = orm::entity_manager(orm::connection(), None);
    let query = entity_manager.create_query(&dql);

    let SqlStatements::One(sql) = query.get_sql().unwrap() else {
        panic!("one statement")
    };
    assert!(matches(&sql), "{sql}");
}
