//! LimitSubqueryOutputWalkerTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on the test entity manager (`tests/common/dql.rs`, `SQLite`). The DBAL
//! port has only `SQLitePlatform`: the tests on the `MySQLPlatform` run on
//! it, since neither platform has `ROW_NUMBER()` for the walker and both
//! write `LIMIT 20 OFFSET 10`, so upstream's SQL is the same; the tests on
//! the `PostgreSQLPlatform` and `OraclePlatform` (the `ROW_NUMBER()` SQL)
//! are pending.
//! `testSubqueryClonedCompletely` runs on a `Query` of the functional test
//! entity manager (`tests/common/orm.rs`); the walker's `getQuery()` is
//! `get_clone_query()`. `testParsingQueryWithDifferentLimitOffsetValuesTakesOnlyOneCacheEntry`
//! is pending: the port has no query cache.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/LimitSubqueryOutputWalkerTest.php

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
#[path = "common/orm.rs"]
mod orm;

use std::cell::RefCell;
use std::rc::Rc;

use php_value::Value;
use php2rust_doctrine_orm::query::ParserResult;
use php2rust_doctrine_orm::query::context::{HINT_CUSTOM_OUTPUT_WALKER, HYDRATE_ARRAY, Hint};
use php2rust_doctrine_orm::query::context::{QueryComponents, QueryContext};
use php2rust_doctrine_orm::tools::pagination::LimitSubqueryOutputWalker;

use dql::{TestEntityManager, TestQuery};

fn create_query<'em>(entity_manager: &'em TestEntityManager, dql: &str) -> TestQuery<'em> {
    let query = entity_manager.create_query(dql);
    query.hint(
        HINT_CUSTOM_OUTPUT_WALKER,
        Hint::OutputWalker(LimitSubqueryOutputWalker::CLASS),
    );
    query.set_first_result(10);
    query.set_max_results(Some(20));
    query
}

fn assert_query_sql(expected_sql: &str, dql: &str) {
    let entity_manager = TestEntityManager::new();
    let sql = entity_manager
        .get_database_platform()
        .modify_limit_query(expected_sql, Some(20), 10)
        .unwrap();
    let query = create_query(&entity_manager, dql);

    assert_eq!(sql, query.get_single_sql().unwrap());
}

#[test]
fn testSubqueryClonedCompletely() {
    let em = orm::entity_manager(orm::connection(), None);
    let query = em.create_query(r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p");
    query.set_hint(
        HINT_CUSTOM_OUTPUT_WALKER,
        Hint::OutputWalker(LimitSubqueryOutputWalker::CLASS),
    );
    query.set_first_result(10);
    query.set_max_results(Some(20));
    query.set_parameter("dummy-param", 123, None);
    query.set_hint("dummy-hint", Hint::Value(Value::from("dummy-value")));
    query.set_cacheable(true);

    let walker = LimitSubqueryOutputWalker::new(
        &query,
        Rc::new(RefCell::new(ParserResult::new())),
        QueryComponents::new(),
    );
    let clone = walker.get_clone_query().expect("a Query");

    assert!(!std::ptr::eq(std::ptr::from_ref(&query), clone));
    assert!(clone.has_hint("dummy-hint"));
    assert!(matches!(
        clone.get_hint("dummy-hint"),
        Hint::Value(value) if value.identical(&Value::from("dummy-value"))
    ));
    // `assertNotSame($query->getParameters(), ...)`: the parameters are
    // copied values.
    let param = clone.get_parameter("dummy-param").expect("a Parameter");
    assert!(param.get_value().identical(&Value::Int(123)));
    assert!(!clone.is_cacheable());
}

#[test]
fn testLimitSubquery() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT m0_.id AS id_0, m0_.title AS title_1, c1_.id AS id_2, a2_.id AS id_3, a2_.name AS name_4, m0_.author_id AS author_id_5, m0_.category_id AS category_id_6 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON m0_.category_id = c1_.id INNER JOIN Author a2_ ON m0_.author_id = a2_.id) dctrn_result",
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN p.category c JOIN p.author a",
    );
}

#[test]
fn testCountQueryMixedResultsWithName() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT a0_.id AS id_0, a0_.name AS name_1, sum(a0_.name) AS sclr_2 FROM Author a0_) dctrn_result",
        r"SELECT a, sum(a.name) as foo FROM Doctrine\Tests\ORM\Tools\Pagination\Author a",
    );
}

/// DDC-3336 (`MySQLPlatform` upstream).
#[test]
fn testCountQueryWithArithmeticOrderByCondition() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, (1 - 1000) * 1 FROM (SELECT a0_.id AS id_0, a0_.name AS name_1 FROM Author a0_) dctrn_result_inner ORDER BY (1 - 1000) * 1 DESC) dctrn_result",
        r"SELECT a FROM Doctrine\Tests\ORM\Tools\Pagination\Author a ORDER BY (1 - 1000) * 1 DESC",
    );
}

/// `MySQLPlatform` upstream.
#[test]
fn testCountQueryWithComplexScalarOrderByItemWithoutJoin() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, imageHeight_2 * imageWidth_3 FROM (SELECT a0_.id AS id_0, a0_.image AS image_1, a0_.imageHeight AS imageHeight_2, a0_.imageWidth AS imageWidth_3, a0_.imageAltDesc AS imageAltDesc_4, a0_.user_id AS user_id_5 FROM Avatar a0_) dctrn_result_inner ORDER BY imageHeight_2 * imageWidth_3 DESC) dctrn_result",
        r"SELECT a FROM Doctrine\Tests\ORM\Tools\Pagination\Avatar a ORDER BY a.imageHeight * a.imageWidth DESC",
    );
}

/// `MySQLPlatform` upstream.
#[test]
fn testCountQueryWithComplexScalarOrderByItemJoinedWithoutPartial() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, imageHeight_3 * imageWidth_4 FROM (SELECT u0_.id AS id_0, a1_.id AS id_1, a1_.image AS image_2, a1_.imageHeight AS imageHeight_3, a1_.imageWidth AS imageWidth_4, a1_.imageAltDesc AS imageAltDesc_5, a1_.user_id AS user_id_6 FROM User u0_ INNER JOIN Avatar a1_ ON u0_.id = a1_.user_id) dctrn_result_inner ORDER BY imageHeight_3 * imageWidth_4 DESC) dctrn_result",
        r"SELECT u FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.avatar a ORDER BY a.imageHeight * a.imageWidth DESC",
    );
}

/// `MySQLPlatform` upstream.
#[test]
fn testCountQueryWithComplexScalarOrderByItemJoinedWithPartial() {
    let entity_manager = TestEntityManager::new();

    let query = entity_manager.create_query(
        r"SELECT u, partial a.{id, imageAltDesc} FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.avatar a ORDER BY a.imageHeight * a.imageWidth DESC",
    );

    query.set_hydration_mode(HYDRATE_ARRAY);
    query.hint(
        HINT_CUSTOM_OUTPUT_WALKER,
        Hint::OutputWalker(LimitSubqueryOutputWalker::CLASS),
    );

    assert_eq!(
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, imageHeight_5 * imageWidth_6 FROM (SELECT u0_.id AS id_0, a1_.id AS id_1, a1_.imageAltDesc AS imageAltDesc_2, a1_.id AS id_3, a1_.image AS image_4, a1_.imageHeight AS imageHeight_5, a1_.imageWidth AS imageWidth_6, a1_.imageAltDesc AS imageAltDesc_7 FROM User u0_ INNER JOIN Avatar a1_ ON u0_.id = a1_.user_id) dctrn_result_inner ORDER BY imageHeight_5 * imageWidth_6 DESC) dctrn_result",
        query.get_single_sql().unwrap()
    );
}

/// DDC-3434
#[test]
fn testLimitSubqueryWithHiddenSelectionInOrderBy() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, name_2 FROM (SELECT a0_.id AS id_0, a0_.name AS name_1, a0_.name AS name_2 FROM Author a0_) dctrn_result_inner ORDER BY name_2 DESC) dctrn_result",
        r"SELECT a, a.name AS HIDDEN ord FROM Doctrine\Tests\ORM\Tools\Pagination\Author a ORDER BY ord DESC",
    );
}

#[test]
fn testLimitSubqueryWithOrderByInnerJoined() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, name_2 FROM (SELECT b0_.id AS id_0, a1_.id AS id_1, a1_.name AS name_2, b0_.author_id AS author_id_3, b0_.category_id AS category_id_4 FROM BlogPost b0_ INNER JOIN Author a1_ ON b0_.author_id = a1_.id) dctrn_result_inner ORDER BY name_2 ASC) dctrn_result",
        r"SELECT b FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost b JOIN b.author a ORDER BY a.name ASC",
    );
}

/// `MySQLPlatform` upstream.
#[test]
fn testLimitSubqueryWithColumnWithSortDirectionInName() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, imageAltDesc_4 FROM (SELECT a0_.id AS id_0, a0_.image AS image_1, a0_.imageHeight AS imageHeight_2, a0_.imageWidth AS imageWidth_3, a0_.imageAltDesc AS imageAltDesc_4, a0_.user_id AS user_id_5 FROM Avatar a0_) dctrn_result_inner ORDER BY imageAltDesc_4 DESC) dctrn_result",
        r"SELECT a FROM Doctrine\Tests\ORM\Tools\Pagination\Avatar a ORDER BY a.imageAltDesc DESC",
    );
}

/// `MySQLPlatform` upstream.
#[test]
fn testLimitSubqueryWithOrderByAndSubSelectInWhereClauseMySql() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0 FROM (SELECT b0_.id AS id_0, b0_.author_id AS author_id_1, b0_.category_id AS category_id_2 FROM BlogPost b0_ WHERE ((SELECT COUNT(b1_.id) AS sclr_3 FROM BlogPost b1_) = 1)) dctrn_result_inner ORDER BY id_0 DESC) dctrn_result",
        r"SELECT b FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost b WHERE  ((SELECT COUNT(simple.id) FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost simple) = 1) ORDER BY b.id DESC",
    );
}

/// This tests ordering by property that has the 'declared' field
/// (`MySQLPlatform` upstream).
#[test]
fn testLimitSubqueryOrderByFieldFromMappedSuperclass() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0 FROM (SELECT b0_.id AS id_0, b0_.name AS name_1 FROM Banner b0_) dctrn_result_inner ORDER BY id_0 DESC) dctrn_result",
        r"SELECT b FROM Doctrine\Tests\ORM\Tools\Pagination\Banner b ORDER BY b.id DESC",
    );
}

/// Tests order by on a subselect expression (mysql).
#[test]
fn testLimitSubqueryOrderBySubSelectOrderByExpression() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, sclr_2 FROM (SELECT a0_.id AS id_0, a0_.name AS name_1, (SELECT MIN(m1_.title) AS sclr_3 FROM MyBlogPost m1_ WHERE m1_.author_id = a0_.id) AS sclr_2 FROM Author a0_) dctrn_result_inner ORDER BY sclr_2 DESC) dctrn_result",
        r"SELECT a, ( SELECT MIN(bp.title) FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost bp WHERE bp.author = a ) AS HIDDEN first_blog_post FROM Doctrine\Tests\ORM\Tools\Pagination\Author a ORDER BY first_blog_post DESC",
    );
}
