//! CountWalkerTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on the test entity manager (`tests/common/dql.rs`, `SQLite`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/CountWalkerTest.php

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
use php2rust_doctrine_orm::tools::pagination::CountWalker;

use dql::{TestEntityManager, TestQuery, thrown};

fn count_query<'em>(
    em: &'em TestEntityManager,
    dql: &str,
    distinct: Option<bool>,
) -> TestQuery<'em> {
    let query = em.create_query(dql);
    query.hint(
        HINT_CUSTOM_TREE_WALKERS,
        Hint::TreeWalkers(vec![CountWalker::CLASS]),
    );
    if let Some(distinct) = distinct {
        query.hint(
            CountWalker::HINT_DISTINCT,
            Hint::Value(Value::Bool(distinct)),
        );
    }
    query
}

#[test]
fn testCountQuery() {
    let em = TestEntityManager::new();
    let query = count_query(
        &em,
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p JOIN p.category c JOIN p.author a",
        Some(true),
    );
    query.set_first_result(0).set_max_results(None);

    assert_eq!(
        "SELECT count(DISTINCT b0_.id) AS sclr_0 FROM BlogPost b0_ INNER JOIN Category c1_ ON b0_.category_id = c1_.id INNER JOIN Author a2_ ON b0_.author_id = a2_.id",
        query.get_single_sql().unwrap()
    );
}

#[test]
fn testCountQueryWithoutDistinctUsesCountStar() {
    let em = TestEntityManager::new();
    let query = count_query(
        &em,
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p JOIN p.category c JOIN p.author a",
        Some(false),
    );

    assert_eq!(
        "SELECT count(*) AS sclr_0 FROM BlogPost b0_ INNER JOIN Category c1_ ON b0_.category_id = c1_.id INNER JOIN Author a2_ ON b0_.author_id = a2_.id",
        query.get_single_sql().unwrap()
    );
}

#[test]
fn testCountQueryMixedResultsWithName() {
    let em = TestEntityManager::new();
    let query = count_query(
        &em,
        r"SELECT a, sum(a.name) as foo FROM Doctrine\Tests\ORM\Tools\Pagination\Author a",
        Some(true),
    );
    query.set_first_result(0).set_max_results(None);

    assert_eq!(
        "SELECT count(DISTINCT a0_.id) AS sclr_0 FROM Author a0_",
        query.get_single_sql().unwrap()
    );
}

#[test]
fn testCountQueryKeepsGroupBy() {
    let em = TestEntityManager::new();
    let query = count_query(
        &em,
        r"SELECT b FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost b GROUP BY b.id",
        Some(true),
    );
    query.set_first_result(0).set_max_results(None);

    assert_eq!(
        "SELECT count(DISTINCT b0_.id) AS sclr_0 FROM BlogPost b0_ GROUP BY b0_.id",
        query.get_single_sql().unwrap()
    );
}

#[test]
fn testCountQueryRemovesOrderBy() {
    let em = TestEntityManager::new();
    let query = count_query(
        &em,
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p JOIN p.category c JOIN p.author a ORDER BY a.name",
        Some(true),
    );
    query.set_first_result(0).set_max_results(None);

    assert_eq!(
        "SELECT count(DISTINCT b0_.id) AS sclr_0 FROM BlogPost b0_ INNER JOIN Category c1_ ON b0_.category_id = c1_.id INNER JOIN Author a2_ ON b0_.author_id = a2_.id",
        query.get_single_sql().unwrap()
    );
}

#[test]
fn testCountQueryRemovesLimits() {
    let em = TestEntityManager::new();
    let query = count_query(
        &em,
        r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p JOIN p.category c JOIN p.author a",
        Some(true),
    );
    query.set_first_result(0).set_max_results(None);

    assert_eq!(
        "SELECT count(DISTINCT b0_.id) AS sclr_0 FROM BlogPost b0_ INNER JOIN Category c1_ ON b0_.category_id = c1_.id INNER JOIN Author a2_ ON b0_.author_id = a2_.id",
        query.get_single_sql().unwrap()
    );
}

#[test]
fn testCountQueryHavingException() {
    let em = TestEntityManager::new();
    let query = count_query(
        &em,
        r"SELECT g, COUNT(u.id) AS userCount FROM Doctrine\Tests\Models\CMS\CmsGroup g LEFT JOIN g.users u GROUP BY g.id HAVING userCount > 0",
        None,
    );
    query.set_first_result(0).set_max_results(None);

    assert_eq!(
        (
            "RuntimeException".to_owned(),
            "Cannot count query that uses a HAVING clause. Use the output walkers for pagination"
                .to_owned()
        ),
        thrown(&query.get_single_sql().unwrap_err())
    );
}

/// Arbitrary Join
#[test]
fn testCountQueryWithArbitraryJoin() {
    let em = TestEntityManager::new();
    let query = count_query(
        &em,
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p LEFT JOIN Doctrine\Tests\ORM\Tools\Pagination\Category c ON p.category = c",
        Some(true),
    );
    query.set_first_result(0).set_max_results(None);

    assert_eq!(
        "SELECT count(DISTINCT b0_.id) AS sclr_0 FROM BlogPost b0_ LEFT JOIN Category c1_ ON (b0_.category_id = c1_.id)",
        query.get_single_sql().unwrap()
    );
}
