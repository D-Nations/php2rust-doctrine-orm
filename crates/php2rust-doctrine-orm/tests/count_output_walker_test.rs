//! CountOutputWalkerTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on the test entity manager (`tests/common/dql.rs`, `SQLite`).
//! `testCountQueryOrderBySqlServer` runs on `SQLServerPlatform` only
//! (upstream skips it on any other platform, so on its `SQLite` test entity
//! manager): here it runs on a test entity manager with DBAL's
//! `SQLServerPlatform`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/CountOutputWalkerTest.php

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

use php2rust_doctrine_dbal::platforms::SQLServerPlatform;
use php2rust_doctrine_orm::query::context::{HINT_CUSTOM_OUTPUT_WALKER, Hint};
use php2rust_doctrine_orm::tools::pagination::CountOutputWalker;

use dql::TestEntityManager;

fn count_sql(dql: &str) -> String {
    let entity_manager = TestEntityManager::new();
    let query = entity_manager.create_query(dql);
    query.hint(
        HINT_CUSTOM_OUTPUT_WALKER,
        Hint::OutputWalker(CountOutputWalker::CLASS),
    );
    query.set_first_result(0).set_max_results(None);
    query.get_single_sql().unwrap()
}

#[test]
fn testCountQuery() {
    assert_eq!(
        "SELECT COUNT(*) AS dctrn_count FROM (SELECT DISTINCT id_0 FROM (SELECT b0_.id AS id_0, c1_.id AS id_1, a2_.id AS id_2, a2_.name AS name_3, b0_.author_id AS author_id_4, b0_.category_id AS category_id_5 FROM BlogPost b0_ INNER JOIN Category c1_ ON b0_.category_id = c1_.id INNER JOIN Author a2_ ON b0_.author_id = a2_.id) dctrn_result) dctrn_table",
        count_sql(
            r"SELECT p, c, a FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p JOIN p.category c JOIN p.author a"
        )
    );
}

#[test]
fn testCountQueryMixedResultsWithName() {
    assert_eq!(
        "SELECT COUNT(*) AS dctrn_count FROM (SELECT DISTINCT id_0 FROM (SELECT a0_.id AS id_0, a0_.name AS name_1, sum(a0_.name) AS sclr_2 FROM Author a0_) dctrn_result) dctrn_table",
        count_sql(
            r"SELECT a, sum(a.name) as foo FROM Doctrine\Tests\ORM\Tools\Pagination\Author a"
        )
    );
}

#[test]
fn testCountQueryGroupBy() {
    assert_eq!(
        "SELECT COUNT(*) AS dctrn_count FROM (SELECT p0_.name AS name_0 FROM Person p0_ GROUP BY p0_.name) dctrn_table",
        count_sql(
            r"SELECT p.name FROM Doctrine\Tests\ORM\Tools\Pagination\Person p GROUP BY p.name"
        )
    );
}

#[test]
fn testCountQueryHaving() {
    assert_eq!(
        "SELECT COUNT(*) AS dctrn_count FROM (SELECT count(u0_.id) AS sclr_0, g1_.id AS id_1, u0_.id AS id_2 FROM groups g1_ LEFT JOIN user_group u2_ ON g1_.id = u2_.group_id LEFT JOIN User u0_ ON u0_.id = u2_.user_id GROUP BY g1_.id HAVING sclr_0 > 0) dctrn_table",
        count_sql(
            r"SELECT g, u, count(u.id) AS userCount FROM Doctrine\Tests\ORM\Tools\Pagination\Group g LEFT JOIN g.users u GROUP BY g.id HAVING userCount > 0"
        )
    );
}

#[test]
fn testCountQueryOrderBySqlServer() {
    let entity_manager = TestEntityManager::with_platform(Box::new(SQLServerPlatform::new()));
    let query = entity_manager.create_query(
        r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p ORDER BY p.id",
    );
    query.hint(
        HINT_CUSTOM_OUTPUT_WALKER,
        Hint::OutputWalker(CountOutputWalker::CLASS),
    );
    query.set_first_result(0).set_max_results(None);

    assert_eq!(
        "SELECT COUNT(*) AS dctrn_count FROM (SELECT DISTINCT id_0 FROM (SELECT b0_.id AS id_0, b0_.author_id AS author_id_1, b0_.category_id AS category_id_2 FROM BlogPost b0_) dctrn_result) dctrn_table",
        query.get_single_sql().unwrap()
    );
}
