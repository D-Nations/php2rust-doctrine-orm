//! WhereInWalkerTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on the test entity manager (`tests/common/dql.rs`, `SQLite`). The
//! provider's rows run in one test, labelled.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/WhereInWalkerTest.php

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

use php_value::{Key, Value};
use php2rust_doctrine_orm::query::Parser;
use php2rust_doctrine_orm::query::context::{HINT_CUSTOM_TREE_WALKERS, Hint};
use php2rust_doctrine_orm::query::exec::SqlStatements;
use php2rust_doctrine_orm::tools::pagination::WhereInWalker;

use dql::TestEntityManager;

fn example_queries() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "no WHERE condition",
            r"SELECT u, g FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g",
            "SELECT u0_.id AS id_0, g1_.id AS id_1 FROM User u0_ INNER JOIN user_group u2_ ON u0_.id = u2_.user_id INNER JOIN groups g1_ ON g1_.id = u2_.group_id WHERE u0_.id IN (?)",
        ),
        (
            "mixed results with name",
            r"SELECT a, sum(a.name) as foo FROM Doctrine\Tests\ORM\Tools\Pagination\Author a",
            "SELECT a0_.id AS id_0, a0_.name AS name_1, sum(a0_.name) AS sclr_2 FROM Author a0_ WHERE a0_.id IN (?)",
        ),
        (
            "single WHERE condition",
            r"SELECT u, g FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g WHERE 1 = 1",
            "SELECT u0_.id AS id_0, g1_.id AS id_1 FROM User u0_ INNER JOIN user_group u2_ ON u0_.id = u2_.user_id INNER JOIN groups g1_ ON g1_.id = u2_.group_id WHERE 1 = 1 AND u0_.id IN (?)",
        ),
        (
            "multiple WHERE conditions with AND",
            r"SELECT u, g FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g WHERE 1 = 1 AND 2 = 2",
            "SELECT u0_.id AS id_0, g1_.id AS id_1 FROM User u0_ INNER JOIN user_group u2_ ON u0_.id = u2_.user_id INNER JOIN groups g1_ ON g1_.id = u2_.group_id WHERE 1 = 1 AND 2 = 2 AND u0_.id IN (?)",
        ),
        (
            "multiple WHERE conditions with OR",
            r"SELECT u, g FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g WHERE 1 = 1 OR 2 = 2",
            "SELECT u0_.id AS id_0, g1_.id AS id_1 FROM User u0_ INNER JOIN user_group u2_ ON u0_.id = u2_.user_id INNER JOIN groups g1_ ON g1_.id = u2_.group_id WHERE (1 = 1 OR 2 = 2) AND u0_.id IN (?)",
        ),
        (
            "multiple WHERE conditions with mixed clauses 1",
            r"SELECT u, g FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g WHERE (1 = 1 OR 2 = 2) AND 3 = 3",
            "SELECT u0_.id AS id_0, g1_.id AS id_1 FROM User u0_ INNER JOIN user_group u2_ ON u0_.id = u2_.user_id INNER JOIN groups g1_ ON g1_.id = u2_.group_id WHERE (1 = 1 OR 2 = 2) AND 3 = 3 AND u0_.id IN (?)",
        ),
        (
            "multiple WHERE conditions with mixed clauses 2",
            r"SELECT u, g FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g WHERE 1 = 1 AND 2 = 2 OR 3 = 3",
            "SELECT u0_.id AS id_0, g1_.id AS id_1 FROM User u0_ INNER JOIN user_group u2_ ON u0_.id = u2_.user_id INNER JOIN groups g1_ ON g1_.id = u2_.group_id WHERE (1 = 1 AND 2 = 2 OR 3 = 3) AND u0_.id IN (?)",
        ),
        (
            "WHERE NOT condition",
            r"SELECT u, g FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g WHERE NOT 1 = 2",
            "SELECT u0_.id AS id_0, g1_.id AS id_1 FROM User u0_ INNER JOIN user_group u2_ ON u0_.id = u2_.user_id INNER JOIN groups g1_ ON g1_.id = u2_.group_id WHERE (NOT 1 = 2) AND u0_.id IN (?)",
        ),
        (
            "arbitary join with no WHERE",
            r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p JOIN Doctrine\Tests\ORM\Tools\Pagination\Category c ON p.category = c",
            "SELECT b0_.id AS id_0, b0_.author_id AS author_id_1, b0_.category_id AS category_id_2 FROM BlogPost b0_ INNER JOIN Category c1_ ON (b0_.category_id = c1_.id) WHERE b0_.id IN (?)",
        ),
        (
            "arbitary join with single WHERE",
            r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p JOIN Doctrine\Tests\ORM\Tools\Pagination\Category c ON p.category = c WHERE 1 = 1",
            "SELECT b0_.id AS id_0, b0_.author_id AS author_id_1, b0_.category_id AS category_id_2 FROM BlogPost b0_ INNER JOIN Category c1_ ON (b0_.category_id = c1_.id) WHERE 1 = 1 AND b0_.id IN (?)",
        ),
    ]
}

#[test]
fn testDqlQueryTransformation() {
    let em = TestEntityManager::new();
    for (label, dql, expected_sql) in example_queries() {
        let query = em.create_query(dql);
        query.hint(
            HINT_CUSTOM_TREE_WALKERS,
            Hint::TreeWalkers(vec![WhereInWalker::CLASS]),
        );
        query.hint(
            WhereInWalker::HINT_PAGINATOR_HAS_IDS,
            Hint::Value(Value::Bool(true)),
        );

        let result = Parser::new(&query).unwrap().parse().unwrap();
        let executor = result.borrow().prepare_sql_executor(&query).unwrap();

        assert_eq!(
            &SqlStatements::One(expected_sql.to_owned()),
            executor.get_sql_statements(),
            "{label}"
        );
        assert_eq!(
            Some(vec![0]),
            result
                .borrow()
                .get_sql_parameter_positions(&Key::from(WhereInWalker::PAGINATOR_ID_ALIAS)),
            "{label}"
        );
    }
}
