//! LimitSubqueryOutputWalkerTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on the test entity manager (`tests/common/dql.rs`, `SQLite`), and on
//! DBAL's `MySQLPlatform`, `PostgreSQLPlatform` and `OraclePlatform` where
//! upstream's test creates the entity manager with that platform.
//! `testSubqueryClonedCompletely` runs on a `Query` of the functional test
//! entity manager (`tests/common/orm.rs`); the walker's `getQuery()` is
//! `get_clone_query()`. `testParsingQueryWithDifferentLimitOffsetValuesTakesOnlyOneCacheEntry`
//! runs on one too, as its query cache is the `Query`'s.
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
use std::sync::Arc;

use php_value::Value;
use php2rust_doctrine_orm::cache::pool::ArrayAdapter;
use php2rust_doctrine_orm::query::ParserResult;
use php2rust_doctrine_orm::query::context::{HINT_CUSTOM_OUTPUT_WALKER, HYDRATE_ARRAY, Hint};
use php2rust_doctrine_orm::query::context::{QueryComponents, QueryContext};
use php2rust_doctrine_orm::tools::pagination::LimitSubqueryOutputWalker;

use php2rust_doctrine_dbal::platforms::{
    AbstractPlatform, MySQLPlatform, OraclePlatform, PostgreSQLPlatform,
};

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
    assert_query_sql_on(&TestEntityManager::new(), expected_sql, dql);
}

/// `createTestEntityManagerWithPlatform($platform)`.
fn with_platform(platform: impl AbstractPlatform + 'static) -> TestEntityManager {
    TestEntityManager::with_platform(Box::new(platform))
}

fn mysql() -> TestEntityManager {
    with_platform(MySQLPlatform::new())
}

fn postgresql() -> TestEntityManager {
    with_platform(PostgreSQLPlatform::new())
}

fn oracle() -> TestEntityManager {
    with_platform(OraclePlatform::new())
}

/// `assertQuerySql()` on `$this->entityManager`.
fn assert_query_sql_on(entity_manager: &TestEntityManager, expected_sql: &str, dql: &str) {
    let sql = entity_manager
        .get_database_platform()
        .modify_limit_query(expected_sql, Some(20), 10)
        .unwrap();
    let query = create_query(entity_manager, dql);

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
fn testParsingQueryWithDifferentLimitOffsetValuesTakesOnlyOneCacheEntry() {
    let em = orm::entity_manager(orm::connection(), None);
    let query_cache = Arc::new(ArrayAdapter::new());
    em.get_configuration().set_query_cache(query_cache.clone());

    let query = em.create_query(
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN p.category c JOIN p.author a",
    );
    query.set_hint(
        HINT_CUSTOM_OUTPUT_WALKER,
        Hint::OutputWalker(LimitSubqueryOutputWalker::CLASS),
    );
    query.set_first_result(10);
    query.set_max_results(Some(20));

    assert_eq!(
        "SELECT DISTINCT id_0 FROM (SELECT m0_.id AS id_0, m0_.title AS title_1, c1_.id AS id_2, a2_.id AS id_3, a2_.name AS name_4, m0_.author_id AS author_id_5, m0_.category_id AS category_id_6 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON m0_.category_id = c1_.id INNER JOIN Author a2_ ON m0_.author_id = a2_.id) dctrn_result LIMIT 20 OFFSET 10",
        query.get_single_sql().unwrap(),
    );

    query.set_first_result(30).set_max_results(Some(40));

    assert_eq!(
        "SELECT DISTINCT id_0 FROM (SELECT m0_.id AS id_0, m0_.title AS title_1, c1_.id AS id_2, a2_.id AS id_3, a2_.name AS name_4, m0_.author_id AS author_id_5, m0_.category_id AS category_id_6 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON m0_.category_id = c1_.id INNER JOIN Author a2_ ON m0_.author_id = a2_.id) dctrn_result LIMIT 40 OFFSET 30",
        query.get_single_sql().unwrap(),
    );

    assert_eq!(1, query_cache.get_values().len());
}

#[test]
fn testLimitSubquery() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT m0_.id AS id_0, m0_.title AS title_1, c1_.id AS id_2, a2_.id AS id_3, a2_.name AS name_4, m0_.author_id AS author_id_5, m0_.category_id AS category_id_6 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON m0_.category_id = c1_.id INNER JOIN Author a2_ ON m0_.author_id = a2_.id) dctrn_result",
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN p.category c JOIN p.author a",
    );
}

#[test]
fn testLimitSubqueryWithSortPg() {
    assert_query_sql_on(
        &postgresql(),
        "SELECT DISTINCT id_0, MIN(sclr_5) AS dctrn_minrownum FROM (SELECT m0_.id AS id_0, m0_.title AS title_1, c1_.id AS id_2, a2_.id AS id_3, a2_.name AS name_4, ROW_NUMBER() OVER(ORDER BY m0_.title ASC) AS sclr_5, m0_.author_id AS author_id_6, m0_.category_id AS category_id_7 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON m0_.category_id = c1_.id INNER JOIN Author a2_ ON m0_.author_id = a2_.id) dctrn_result GROUP BY id_0 ORDER BY dctrn_minrownum ASC",
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN p.category c JOIN p.author a ORDER BY p.title",
    );
}

#[test]
fn testLimitSubqueryWithScalarSortPg() {
    assert_query_sql_on(
        &postgresql(),
        "SELECT DISTINCT id_1, MIN(sclr_3) AS dctrn_minrownum FROM (SELECT COUNT(g0_.id) AS sclr_0, u1_.id AS id_1, g0_.id AS id_2, ROW_NUMBER() OVER(ORDER BY COUNT(g0_.id) ASC) AS sclr_3 FROM User u1_ INNER JOIN user_group u2_ ON u1_.id = u2_.user_id INNER JOIN groups g0_ ON g0_.id = u2_.group_id) dctrn_result GROUP BY id_1 ORDER BY dctrn_minrownum ASC",
        r"SELECT u, g, COUNT(g.id) AS g_quantity FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g ORDER BY g_quantity",
    );
}

#[test]
fn testLimitSubqueryWithMixedSortPg() {
    assert_query_sql_on(
        &postgresql(),
        "SELECT DISTINCT id_1, MIN(sclr_3) AS dctrn_minrownum FROM (SELECT COUNT(g0_.id) AS sclr_0, u1_.id AS id_1, g0_.id AS id_2, ROW_NUMBER() OVER(ORDER BY COUNT(g0_.id) ASC, u1_.id DESC) AS sclr_3 FROM User u1_ INNER JOIN user_group u2_ ON u1_.id = u2_.user_id INNER JOIN groups g0_ ON g0_.id = u2_.group_id) dctrn_result GROUP BY id_1 ORDER BY dctrn_minrownum ASC",
        r"SELECT u, g, COUNT(g.id) AS g_quantity FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g ORDER BY g_quantity, u.id DESC",
    );
}

#[test]
fn testLimitSubqueryWithHiddenScalarSortPg() {
    assert_query_sql_on(
        &postgresql(),
        "SELECT DISTINCT id_1, MIN(sclr_3) AS dctrn_minrownum FROM (SELECT COUNT(g0_.id) AS sclr_0, u1_.id AS id_1, g0_.id AS id_2, ROW_NUMBER() OVER(ORDER BY COUNT(g0_.id) ASC, u1_.id DESC) AS sclr_3 FROM User u1_ INNER JOIN user_group u2_ ON u1_.id = u2_.user_id INNER JOIN groups g0_ ON g0_.id = u2_.group_id) dctrn_result GROUP BY id_1 ORDER BY dctrn_minrownum ASC",
        r"SELECT u, g, COUNT(g.id) AS hidden g_quantity FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g ORDER BY g_quantity, u.id DESC",
    );
}

#[test]
fn testLimitSubqueryPg() {
    // `$this->testLimitSubquery()` on the PostgreSQL entity manager.
    assert_query_sql_on(
        &postgresql(),
        "SELECT DISTINCT id_0 FROM (SELECT m0_.id AS id_0, m0_.title AS title_1, c1_.id AS id_2, a2_.id AS id_3, a2_.name AS name_4, m0_.author_id AS author_id_5, m0_.category_id AS category_id_6 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON m0_.category_id = c1_.id INNER JOIN Author a2_ ON m0_.author_id = a2_.id) dctrn_result",
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN p.category c JOIN p.author a",
    );
}

#[test]
fn testLimitSubqueryWithSortOracle() {
    assert_query_sql_on(
        &oracle(),
        "SELECT DISTINCT ID_0, MIN(SCLR_5) AS dctrn_minrownum FROM (SELECT m0_.id AS ID_0, m0_.title AS TITLE_1, c1_.id AS ID_2, a2_.id AS ID_3, a2_.name AS NAME_4, ROW_NUMBER() OVER(ORDER BY m0_.title ASC) AS SCLR_5, m0_.author_id AS AUTHOR_ID_6, m0_.category_id AS CATEGORY_ID_7 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON m0_.category_id = c1_.id INNER JOIN Author a2_ ON m0_.author_id = a2_.id) dctrn_result GROUP BY ID_0 ORDER BY dctrn_minrownum ASC",
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN p.category c JOIN p.author a ORDER BY p.title",
    );
}

#[test]
fn testLimitSubqueryWithScalarSortOracle() {
    assert_query_sql_on(
        &oracle(),
        "SELECT DISTINCT ID_1, MIN(SCLR_3) AS dctrn_minrownum FROM (SELECT COUNT(g0_.id) AS SCLR_0, u1_.id AS ID_1, g0_.id AS ID_2, ROW_NUMBER() OVER(ORDER BY COUNT(g0_.id) ASC) AS SCLR_3 FROM User u1_ INNER JOIN user_group u2_ ON u1_.id = u2_.user_id INNER JOIN groups g0_ ON g0_.id = u2_.group_id) dctrn_result GROUP BY ID_1 ORDER BY dctrn_minrownum ASC",
        r"SELECT u, g, COUNT(g.id) AS g_quantity FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g ORDER BY g_quantity",
    );
}

#[test]
fn testLimitSubqueryWithMixedSortOracle() {
    assert_query_sql_on(
        &oracle(),
        "SELECT DISTINCT ID_1, MIN(SCLR_3) AS dctrn_minrownum FROM (SELECT COUNT(g0_.id) AS SCLR_0, u1_.id AS ID_1, g0_.id AS ID_2, ROW_NUMBER() OVER(ORDER BY COUNT(g0_.id) ASC, u1_.id DESC) AS SCLR_3 FROM User u1_ INNER JOIN user_group u2_ ON u1_.id = u2_.user_id INNER JOIN groups g0_ ON g0_.id = u2_.group_id) dctrn_result GROUP BY ID_1 ORDER BY dctrn_minrownum ASC",
        r"SELECT u, g, COUNT(g.id) AS g_quantity FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g ORDER BY g_quantity, u.id DESC",
    );
}

#[test]
fn testLimitSubqueryOracle() {
    assert_query_sql_on(
        &oracle(),
        "SELECT DISTINCT ID_0 FROM (SELECT m0_.id AS ID_0, m0_.title AS TITLE_1, c1_.id AS ID_2, a2_.id AS ID_3, a2_.name AS NAME_4, m0_.author_id AS AUTHOR_ID_5, m0_.category_id AS CATEGORY_ID_6 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON m0_.category_id = c1_.id INNER JOIN Author a2_ ON m0_.author_id = a2_.id) dctrn_result",
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN p.category c JOIN p.author a",
    );
}

#[test]
fn testCountQueryWithComplexScalarOrderByItemOracle() {
    assert_query_sql_on(
        &oracle(),
        "SELECT DISTINCT ID_0, MIN(SCLR_5) AS dctrn_minrownum FROM (SELECT a0_.id AS ID_0, a0_.image AS IMAGE_1, a0_.imageHeight AS IMAGEHEIGHT_2, a0_.imageWidth AS IMAGEWIDTH_3, a0_.imageAltDesc AS IMAGEALTDESC_4, ROW_NUMBER() OVER(ORDER BY a0_.imageHeight * a0_.imageWidth DESC) AS SCLR_5, a0_.user_id AS USER_ID_6 FROM Avatar a0_) dctrn_result GROUP BY ID_0 ORDER BY dctrn_minrownum ASC",
        r"SELECT a FROM Doctrine\Tests\ORM\Tools\Pagination\Avatar a ORDER BY a.imageHeight * a.imageWidth DESC",
    );
}

#[test]
fn testLimitSubqueryWithOrderByAndSubSelectInWhereClausePgSql() {
    assert_query_sql_on(
        &postgresql(),
        "SELECT DISTINCT id_0, MIN(sclr_1) AS dctrn_minrownum FROM (SELECT b0_.id AS id_0, ROW_NUMBER() OVER(ORDER BY b0_.id DESC) AS sclr_1, b0_.author_id AS author_id_2, b0_.category_id AS category_id_3 FROM BlogPost b0_ WHERE ((SELECT COUNT(b1_.id) AS sclr_4 FROM BlogPost b1_) = 1)) dctrn_result GROUP BY id_0 ORDER BY dctrn_minrownum ASC",
        r"SELECT b FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost b WHERE  ((SELECT COUNT(simple.id) FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost simple) = 1) ORDER BY b.id DESC",
    );
}

/// Tests order by on a subselect expression invoking `RowNumberOverFunction` (postgres).
#[test]
fn testLimitSubqueryOrderBySubSelectOrderByExpressionPg() {
    assert_query_sql_on(
        &postgresql(),
        "SELECT DISTINCT id_0, MIN(sclr_4) AS dctrn_minrownum FROM (SELECT a0_.id AS id_0, a0_.name AS name_1, (SELECT MIN(m1_.title) AS sclr_3 FROM MyBlogPost m1_ WHERE m1_.author_id = a0_.id) AS sclr_2, ROW_NUMBER() OVER(ORDER BY (SELECT MIN(m1_.title) AS sclr_5 FROM MyBlogPost m1_ WHERE m1_.author_id = a0_.id) DESC) AS sclr_4 FROM Author a0_) dctrn_result GROUP BY id_0 ORDER BY dctrn_minrownum ASC",
        r"SELECT a, ( SELECT MIN(bp.title) FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost bp WHERE bp.author = a ) AS HIDDEN first_blog_post FROM Doctrine\Tests\ORM\Tools\Pagination\Author a ORDER BY first_blog_post DESC",
    );
}

/// Tests order by on a subselect expression invoking `RowNumberOverFunction` (oracle).
#[test]
fn testLimitSubqueryOrderBySubSelectOrderByExpressionOracle() {
    assert_query_sql_on(
        &oracle(),
        "SELECT DISTINCT ID_0, MIN(SCLR_4) AS dctrn_minrownum FROM (SELECT a0_.id AS ID_0, a0_.name AS NAME_1, (SELECT MIN(m1_.title) AS SCLR_3 FROM MyBlogPost m1_ WHERE m1_.author_id = a0_.id) AS SCLR_2, ROW_NUMBER() OVER(ORDER BY (SELECT MIN(m1_.title) AS SCLR_5 FROM MyBlogPost m1_ WHERE m1_.author_id = a0_.id) DESC) AS SCLR_4 FROM Author a0_) dctrn_result GROUP BY ID_0 ORDER BY dctrn_minrownum ASC",
        r"SELECT a, ( SELECT MIN(bp.title) FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost bp WHERE bp.author = a ) AS HIDDEN first_blog_post FROM Doctrine\Tests\ORM\Tools\Pagination\Author a ORDER BY first_blog_post DESC",
    );
}

#[test]
fn testCountQueryMixedResultsWithName() {
    assert_query_sql(
        "SELECT DISTINCT id_0 FROM (SELECT a0_.id AS id_0, a0_.name AS name_1, sum(a0_.name) AS sclr_2 FROM Author a0_) dctrn_result",
        r"SELECT a, sum(a.name) as foo FROM Doctrine\Tests\ORM\Tools\Pagination\Author a",
    );
}

/// DDC-3336
#[test]
fn testCountQueryWithArithmeticOrderByCondition() {
    assert_query_sql_on(
        &mysql(),
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, (1 - 1000) * 1 FROM (SELECT a0_.id AS id_0, a0_.name AS name_1 FROM Author a0_) dctrn_result_inner ORDER BY (1 - 1000) * 1 DESC) dctrn_result",
        r"SELECT a FROM Doctrine\Tests\ORM\Tools\Pagination\Author a ORDER BY (1 - 1000) * 1 DESC",
    );
}

#[test]
fn testCountQueryWithComplexScalarOrderByItemWithoutJoin() {
    assert_query_sql_on(
        &mysql(),
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, imageHeight_2 * imageWidth_3 FROM (SELECT a0_.id AS id_0, a0_.image AS image_1, a0_.imageHeight AS imageHeight_2, a0_.imageWidth AS imageWidth_3, a0_.imageAltDesc AS imageAltDesc_4, a0_.user_id AS user_id_5 FROM Avatar a0_) dctrn_result_inner ORDER BY imageHeight_2 * imageWidth_3 DESC) dctrn_result",
        r"SELECT a FROM Doctrine\Tests\ORM\Tools\Pagination\Avatar a ORDER BY a.imageHeight * a.imageWidth DESC",
    );
}

#[test]
fn testCountQueryWithComplexScalarOrderByItemJoinedWithoutPartial() {
    assert_query_sql_on(
        &mysql(),
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, imageHeight_3 * imageWidth_4 FROM (SELECT u0_.id AS id_0, a1_.id AS id_1, a1_.image AS image_2, a1_.imageHeight AS imageHeight_3, a1_.imageWidth AS imageWidth_4, a1_.imageAltDesc AS imageAltDesc_5, a1_.user_id AS user_id_6 FROM User u0_ INNER JOIN Avatar a1_ ON u0_.id = a1_.user_id) dctrn_result_inner ORDER BY imageHeight_3 * imageWidth_4 DESC) dctrn_result",
        r"SELECT u FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.avatar a ORDER BY a.imageHeight * a.imageWidth DESC",
    );
}

#[test]
fn testCountQueryWithComplexScalarOrderByItemJoinedWithPartial() {
    let entity_manager = mysql();

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

#[test]
fn testLimitSubqueryWithColumnWithSortDirectionInName() {
    assert_query_sql_on(
        &mysql(),
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, imageAltDesc_4 FROM (SELECT a0_.id AS id_0, a0_.image AS image_1, a0_.imageHeight AS imageHeight_2, a0_.imageWidth AS imageWidth_3, a0_.imageAltDesc AS imageAltDesc_4, a0_.user_id AS user_id_5 FROM Avatar a0_) dctrn_result_inner ORDER BY imageAltDesc_4 DESC) dctrn_result",
        r"SELECT a FROM Doctrine\Tests\ORM\Tools\Pagination\Avatar a ORDER BY a.imageAltDesc DESC",
    );
}

#[test]
fn testLimitSubqueryWithOrderByAndSubSelectInWhereClauseMySql() {
    assert_query_sql_on(
        &mysql(),
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0 FROM (SELECT b0_.id AS id_0, b0_.author_id AS author_id_1, b0_.category_id AS category_id_2 FROM BlogPost b0_ WHERE ((SELECT COUNT(b1_.id) AS sclr_3 FROM BlogPost b1_) = 1)) dctrn_result_inner ORDER BY id_0 DESC) dctrn_result",
        r"SELECT b FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost b WHERE  ((SELECT COUNT(simple.id) FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost simple) = 1) ORDER BY b.id DESC",
    );
}

/// This tests ordering by property that has the 'declared' field.
#[test]
fn testLimitSubqueryOrderByFieldFromMappedSuperclass() {
    assert_query_sql_on(
        &mysql(),
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0 FROM (SELECT b0_.id AS id_0, b0_.name AS name_1 FROM Banner b0_) dctrn_result_inner ORDER BY id_0 DESC) dctrn_result",
        r"SELECT b FROM Doctrine\Tests\ORM\Tools\Pagination\Banner b ORDER BY b.id DESC",
    );
}

/// Tests order by on a subselect expression (mysql).
#[test]
fn testLimitSubqueryOrderBySubSelectOrderByExpression() {
    assert_query_sql_on(
        &mysql(),
        "SELECT DISTINCT id_0 FROM (SELECT DISTINCT id_0, sclr_2 FROM (SELECT a0_.id AS id_0, a0_.name AS name_1, (SELECT MIN(m1_.title) AS sclr_3 FROM MyBlogPost m1_ WHERE m1_.author_id = a0_.id) AS sclr_2 FROM Author a0_) dctrn_result_inner ORDER BY sclr_2 DESC) dctrn_result",
        r"SELECT a, ( SELECT MIN(bp.title) FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost bp WHERE bp.author = a ) AS HIDDEN first_blog_post FROM Doctrine\Tests\ORM\Tools\Pagination\Author a ORDER BY first_blog_post DESC",
    );
}
