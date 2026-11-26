//! Port of `SqlWalkerTest`: `getSQLTableAlias()` of the default
//! `SqlWalker` on the test entity manager (`tests/common/dql.rs`). The
//! data providers are loops over their cases.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/SqlWalkerTest.php

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

use std::cell::RefCell;
use std::rc::Rc;

use php2rust_doctrine_orm::query::context::QueryComponents;
use php2rust_doctrine_orm::query::sql_walker::DefaultSqlWalker;
use php2rust_doctrine_orm::query::{ParserResult, SqlWalker};

use dql::{TestEntityManager, TestQuery};

/// `getTableNamesAndSqlAliases()`.
const TABLE_NAMES_AND_SQL_ALIASES: [(&str, &str); 3] =
    [("aaaaa", "a0_"), ("table", "t0_"), ("çtable", "t0_")];

/// `getTableNames()`.
const TABLE_NAMES: [&str; 3] = ["aaaaa", "table", "çtable"];

/// `setUp()`: a new walker for each case.
fn with_sql_walker(f: impl FnOnce(&mut DefaultSqlWalker<'_>)) {
    let entity_manager = TestEntityManager::new();
    let query = TestQuery::new(&entity_manager, "");
    let mut sql_walker = DefaultSqlWalker::new(
        &query,
        Rc::new(RefCell::new(ParserResult::new())),
        QueryComponents::new(),
    );
    f(&mut sql_walker);
}

#[test]
fn testGetSQLTableAlias() {
    for (table_name, expected_alias) in TABLE_NAMES_AND_SQL_ALIASES {
        with_sql_walker(|sql_walker| {
            assert_eq!(
                expected_alias,
                sql_walker.get_sql_table_alias(table_name, "")
            );
        });
    }
}

#[test]
fn testGetSQLTableAliasIsSameForMultipleCalls() {
    for table_name in TABLE_NAMES {
        with_sql_walker(|sql_walker| {
            assert_eq!(
                sql_walker.get_sql_table_alias(table_name, ""),
                sql_walker.get_sql_table_alias(table_name, "")
            );
        });
    }
}
