//! CursorWalkerTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on a `Query` of the functional test entity manager (`tests/common/orm.rs`,
//! `SQLite`) with the DQL test models, as upstream's `createQuery()` gives
//! one.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/CursorWalkerTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::query::Query;
use php2rust_doctrine_orm::query::context::{HINT_CUSTOM_TREE_WALKERS, Hint};
use php2rust_doctrine_orm::query::exec::SqlStatements;
use php2rust_doctrine_orm::tools::pagination::CursorWalker;

fn create_query(dql: &str) -> Query {
    models::register_query_models();
    let em = orm::entity_manager(orm::connection(), None);
    let query = em.create_query(dql);
    query.set_hint(
        HINT_CUSTOM_TREE_WALKERS,
        Hint::TreeWalkers(vec![CursorWalker::CLASS]),
    );
    query
}

fn hints(query: &Query, reverse: bool, parameters: &[(&str, Value)]) {
    query.set_hint(
        CursorWalker::HINT_CURSOR_REVERSE,
        Hint::Value(Value::Bool(reverse)),
    );
    let mut array = PhpArray::new();
    for (key, value) in parameters {
        array.insert(*key, value.clone());
    }
    query.set_hint(
        CursorWalker::HINT_CURSOR_PARAMETERS,
        Hint::Value(Value::Array(array)),
    );
}

fn sql(query: &Query) -> String {
    match query.get_sql().unwrap() {
        SqlStatements::One(sql) => sql,
        SqlStatements::Many(_) => panic!("one statement"),
    }
}

#[test]
fn testThrowsExceptionWithoutOrderBy() {
    let query = create_query(r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p");
    let e = query.get_sql().unwrap_err();
    assert_eq!("LogicException", e.class);
    assert_eq!(
        "No ORDER BY clause found. Cursor pagination requires a deterministic sort order.",
        e.message.to_string()
    );
}

#[test]
fn testBasicQueryWithOrderBy() {
    let query = create_query(
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p ORDER BY p.id ASC",
    );
    hints(&query, false, &[]);
    assert_eq!(
        "SELECT b0_.id AS id_0, b0_.author_id AS author_id_1, b0_.category_id AS category_id_2 FROM BlogPost b0_ ORDER BY b0_.id ASC",
        sql(&query)
    );
}

#[test]
fn testQueryWithReversedOrderBy() {
    let query = create_query(
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p ORDER BY p.id ASC",
    );
    hints(&query, true, &[]);
    assert_eq!(
        "SELECT b0_.id AS id_0, b0_.author_id AS author_id_1, b0_.category_id AS category_id_2 FROM BlogPost b0_ ORDER BY b0_.id DESC",
        sql(&query)
    );
}

#[test]
fn testQueryWithMultipleOrderByColumnsReversed() {
    let query = create_query(
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p ORDER BY p.title ASC, p.id DESC",
    );
    hints(&query, true, &[]);
    assert_eq!(
        "SELECT m0_.id AS id_0, m0_.title AS title_1, m0_.author_id AS author_id_2, m0_.category_id AS category_id_3 FROM MyBlogPost m0_ ORDER BY m0_.title DESC, m0_.id ASC",
        sql(&query)
    );
}

#[test]
fn testCursorConditionSingleColumnAsc() {
    let query = create_query(
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p ORDER BY p.id ASC",
    );
    hints(&query, false, &[("p.id", Value::Int(10))]);
    assert_eq!(
        "SELECT b0_.id AS id_0, b0_.author_id AS author_id_1, b0_.category_id AS category_id_2 FROM BlogPost b0_ WHERE (b0_.id > ?) ORDER BY b0_.id ASC",
        sql(&query)
    );
}

#[test]
fn testCursorConditionSingleColumnDesc() {
    let query = create_query(
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p ORDER BY p.id DESC",
    );
    hints(&query, false, &[("p.id", Value::Int(10))]);
    assert_eq!(
        "SELECT b0_.id AS id_0, b0_.author_id AS author_id_1, b0_.category_id AS category_id_2 FROM BlogPost b0_ WHERE (b0_.id < ?) ORDER BY b0_.id DESC",
        sql(&query)
    );
}

#[test]
fn testCursorConditionWithExistingWhere() {
    let query = create_query(
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p WHERE p.id > 5 ORDER BY p.id ASC",
    );
    hints(&query, false, &[("p.id", Value::Int(10))]);
    assert_eq!(
        "SELECT b0_.id AS id_0, b0_.author_id AS author_id_1, b0_.category_id AS category_id_2 FROM BlogPost b0_ WHERE b0_.id > 5 AND (b0_.id > ?) ORDER BY b0_.id ASC",
        sql(&query)
    );
}

#[test]
fn testCursorConditionDoesNotOverwriteAParameterOfThePaginatedQuery() {
    let query = create_query(
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p WHERE p.id <= :p_id_0 ORDER BY p.id ASC",
    );
    query.set_parameter("p_id_0", Value::Int(5), None);
    hints(&query, false, &[("p.id", Value::Int(10))]);
    assert_eq!(
        "SELECT b0_.id AS id_0, b0_.author_id AS author_id_1, b0_.category_id AS category_id_2 FROM BlogPost b0_ WHERE b0_.id <= ? AND (b0_.id > ?) ORDER BY b0_.id ASC",
        sql(&query)
    );
    assert!(Value::Int(5).identical(query.get_parameter("p_id_0").unwrap().get_value()));
    assert!(Value::Int(10).identical(query.get_parameter("dctrn_cursor_0").unwrap().get_value()));
}

#[test]
fn testQueryWithJoinAndOrderByJoinedEntity() {
    let query = create_query(
        r"SELECT p, a FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p JOIN p.author a ORDER BY a.name ASC, p.id ASC",
    );
    hints(&query, false, &[]);
    assert_eq!(
        "SELECT b0_.id AS id_0, a1_.id AS id_1, a1_.name AS name_2, b0_.author_id AS author_id_3, b0_.category_id AS category_id_4 FROM BlogPost b0_ INNER JOIN Author a1_ ON b0_.author_id = a1_.id ORDER BY a1_.name ASC, b0_.id ASC",
        sql(&query)
    );
}

#[test]
fn testCursorConditionMultipleColumnsDesc() {
    let query = create_query(
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p ORDER BY p.title DESC, p.id DESC",
    );
    hints(
        &query,
        false,
        &[("p.title", Value::from("Test")), ("p.id", Value::Int(10))],
    );
    assert_eq!(
        "SELECT m0_.id AS id_0, m0_.title AS title_1, m0_.author_id AS author_id_2, m0_.category_id AS category_id_3 FROM MyBlogPost m0_ WHERE (m0_.title < ? OR (m0_.title = ? AND (m0_.id < ?))) ORDER BY m0_.title DESC, m0_.id DESC",
        sql(&query)
    );
}

#[test]
fn testCursorConditionMultipleColumnsAsc() {
    let query = create_query(
        r"SELECT a FROM Doctrine\Tests\ORM\Tools\Pagination\Author a ORDER BY a.name ASC, a.id ASC",
    );
    hints(
        &query,
        false,
        &[("a.name", Value::from("John")), ("a.id", Value::Int(5))],
    );
    assert_eq!(
        "SELECT a0_.id AS id_0, a0_.name AS name_1 FROM Author a0_ WHERE (a0_.name > ? OR (a0_.name = ? AND (a0_.id > ?))) ORDER BY a0_.name ASC, a0_.id ASC",
        sql(&query)
    );
}

#[test]
fn testCursorConditionThreeColumnsDesc() {
    let query = create_query(
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\Person p ORDER BY p.biography DESC, p.name DESC, p.id DESC",
    );
    hints(
        &query,
        false,
        &[
            ("p.biography", Value::from("2019-11-04")),
            ("p.name", Value::from("test@example.com")),
            ("p.id", Value::Int(529)),
        ],
    );
    assert_eq!(
        "SELECT p0_.id AS id_0, p0_.name AS name_1, p0_.biography AS biography_2 FROM Person p0_ WHERE (p0_.biography < ? OR (p0_.biography = ? AND (p0_.name < ? OR (p0_.name = ? AND (p0_.id < ?))))) ORDER BY p0_.biography DESC, p0_.name DESC, p0_.id DESC",
        sql(&query)
    );
}

#[test]
fn testNoExceptionWithToManyJoinWhenQueryProducesDuplicatesIsTrue() {
    let query = create_query(
        r"SELECT u, g FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g ORDER BY u.id ASC",
    );
    hints(&query, false, &[]);
    query.set_hint(
        CursorWalker::HINT_QUERY_PRODUCES_DUPLICATES,
        Hint::Value(Value::Bool(true)),
    );
    let sql = sql(&query);
    assert!(sql.contains("JOIN"));
}

#[test]
fn testNoExceptionWithToOneJoinWhenQueryProducesDuplicatesIsFalse() {
    let query = create_query(
        r"SELECT p, a FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p JOIN p.author a ORDER BY a.name ASC, p.id ASC",
    );
    hints(&query, false, &[]);
    query.set_hint(
        CursorWalker::HINT_QUERY_PRODUCES_DUPLICATES,
        Hint::Value(Value::Bool(false)),
    );
    let sql = sql(&query);
    assert!(sql.contains("JOIN"));
}
