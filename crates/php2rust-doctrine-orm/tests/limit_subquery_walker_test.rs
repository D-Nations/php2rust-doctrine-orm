//! LimitSubqueryWalkerTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on the test entity manager (`tests/common/dql.rs`, `SQLite`).
//! `clone $query` of a new query is a new query of the same DQL.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/LimitSubqueryWalkerTest.php

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

use php_value::Value;
use php2rust_doctrine_orm::query::context::{HINT_CUSTOM_TREE_WALKERS, Hint};
use php2rust_doctrine_orm::tools::pagination::{HINT_ENABLE_DISTINCT, LimitSubqueryWalker};

use dql::{TestEntityManager, TestQuery};

fn limit_query<'em>(em: &'em TestEntityManager, dql: &str) -> TestQuery<'em> {
    let limit_query = em.create_query(dql);
    limit_query.hint(
        HINT_CUSTOM_TREE_WALKERS,
        Hint::TreeWalkers(vec![LimitSubqueryWalker::CLASS]),
    );
    limit_query
}

#[test]
fn testLimitSubquery() {
    let em = TestEntityManager::new();
    let limit_query = limit_query(
        &em,
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN p.category c JOIN p.author a",
    );

    assert_eq!(
        "SELECT DISTINCT m0_.id AS id_0 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON m0_.category_id = c1_.id INNER JOIN Author a2_ ON m0_.author_id = a2_.id",
        limit_query.get_single_sql().unwrap()
    );
}

#[test]
fn testHintCanDisableDistinct() {
    let em = TestEntityManager::new();
    let limit_query = limit_query(
        &em,
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN p.category c JOIN p.author a",
    );
    limit_query.hint(HINT_ENABLE_DISTINCT, Hint::Value(Value::Bool(false)));

    assert_eq!(
        "SELECT m0_.id AS id_0 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON m0_.category_id = c1_.id INNER JOIN Author a2_ ON m0_.author_id = a2_.id",
        limit_query.get_single_sql().unwrap()
    );
}

#[test]
fn testLimitSubqueryWithSort() {
    let em = TestEntityManager::new();
    let limit_query = limit_query(
        &em,
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN p.category c JOIN p.author a ORDER BY p.title",
    );

    assert_eq!(
        "SELECT DISTINCT m0_.id AS id_0, m0_.title AS title_1 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON m0_.category_id = c1_.id INNER JOIN Author a2_ ON m0_.author_id = a2_.id ORDER BY m0_.title ASC",
        limit_query.get_single_sql().unwrap()
    );
}

#[test]
fn testLimitSubqueryWithSortFunction() {
    let em = TestEntityManager::new();
    let limit_query = limit_query(
        &em,
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN p.category c GROUP BY p.id ORDER BY COUNT(c.id)",
    );

    assert_eq!(
        "SELECT DISTINCT m0_.id AS id_0 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON m0_.category_id = c1_.id GROUP BY m0_.id ORDER BY COUNT(c1_.id) ASC",
        limit_query.get_single_sql().unwrap()
    );
}

#[test]
fn testCountQueryMixedResultsWithName() {
    let em = TestEntityManager::new();
    let limit_query = limit_query(
        &em,
        r"SELECT a, sum(a.name) as foo FROM Doctrine\Tests\ORM\Tools\Pagination\Author a",
    );

    assert_eq!(
        "SELECT DISTINCT a0_.id AS id_0 FROM Author a0_",
        limit_query.get_single_sql().unwrap()
    );
}

#[test]
fn testAggQueryMixedResultsWithNameAndSort() {
    let em = TestEntityManager::new();
    let limit_query = limit_query(
        &em,
        r"SELECT a, sum(a.name) as foo FROM Doctrine\Tests\ORM\Tools\Pagination\Author a ORDER BY foo DESC",
    );

    assert_eq!(
        "SELECT DISTINCT a0_.id AS id_0, sum(a0_.name) AS sclr_1 FROM Author a0_ ORDER BY sclr_1 DESC",
        limit_query.get_single_sql().unwrap()
    );
}

#[test]
fn testAggQueryMultipleMixedResultsWithSort() {
    let em = TestEntityManager::new();
    let limit_query = limit_query(
        &em,
        r"SELECT a, sum(a.name) as foo, (SELECT count(subA.id) FROM Doctrine\Tests\ORM\Tools\Pagination\Author subA WHERE subA.id = a.id ) as bar FROM Doctrine\Tests\ORM\Tools\Pagination\Author a ORDER BY foo DESC, bar ASC",
    );

    assert_eq!(
        "SELECT DISTINCT a0_.id AS id_0, sum(a0_.name) AS sclr_1, (SELECT count(a1_.id) AS sclr_3 FROM Author a1_ WHERE a1_.id = a0_.id) AS sclr_2 FROM Author a0_ ORDER BY sclr_1 DESC, sclr_2 ASC",
        limit_query.get_single_sql().unwrap()
    );
}

#[test]
fn testLimitSubqueryWithSortOnAssociation() {
    let em = TestEntityManager::new();
    let limit_query = limit_query(
        &em,
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p ORDER BY p.author",
    );

    assert_eq!(
        "SELECT DISTINCT m0_.id AS id_0, m0_.author_id AS sclr_1 FROM MyBlogPost m0_ ORDER BY m0_.author_id ASC",
        limit_query.get_single_sql().unwrap()
    );
}

/// Arbitrary Join
#[test]
fn testLimitSubqueryWithArbitraryJoin() {
    let em = TestEntityManager::new();
    let limit_query = limit_query(
        &em,
        r"SELECT p, c FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN Doctrine\Tests\ORM\Tools\Pagination\Category c ON p.category = c",
    );

    assert_eq!(
        "SELECT DISTINCT m0_.id AS id_0 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON (m0_.category_id = c1_.id)",
        limit_query.get_single_sql().unwrap()
    );
}

#[test]
fn testLimitSubqueryWithSortWithArbitraryJoin() {
    let em = TestEntityManager::new();
    let limit_query = limit_query(
        &em,
        r"SELECT p, c FROM Doctrine\Tests\ORM\Tools\Pagination\MyBlogPost p JOIN Doctrine\Tests\ORM\Tools\Pagination\Category c ON p.category = c ORDER BY p.title",
    );

    assert_eq!(
        "SELECT DISTINCT m0_.id AS id_0, m0_.title AS title_1 FROM MyBlogPost m0_ INNER JOIN Category c1_ ON (m0_.category_id = c1_.id) ORDER BY m0_.title ASC",
        limit_query.get_single_sql().unwrap()
    );
}
