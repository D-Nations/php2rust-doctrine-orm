//! Port of `CustomTreeWalkersJoinTest`: the `CustomTreeWalkerJoin` tree
//! walker (`tests/common/custom_tree_walker_join.rs`) on the test entity
//! manager (`tests/common/dql.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/CustomTreeWalkersJoinTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/custom_tree_walker_join.rs"]
mod custom_tree_walker_join;
#[path = "common/dql.rs"]
mod dql;
#[path = "common/models.rs"]
mod models;

use php2rust_doctrine_orm::query::context::{HINT_CUSTOM_TREE_WALKERS, Hint};

use custom_tree_walker_join::CustomTreeWalkerJoin;
use dql::TestEntityManager;

fn assert_sql_generation(dql_to_be_tested: &str, sql_to_be_confirmed: &str) {
    let entity_manager = TestEntityManager::new();
    let query = entity_manager.create_query(dql_to_be_tested);
    query.hint(
        HINT_CUSTOM_TREE_WALKERS,
        Hint::TreeWalkers(vec![CustomTreeWalkerJoin::CLASS]),
    );

    assert_eq!(sql_to_be_confirmed, query.get_single_sql().unwrap());
}

#[test]
fn testAddsJoin() {
    assert_sql_generation(
        r"select u from Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c1_.id AS id_4, c1_.country AS country_5, c1_.zip AS zip_6, c1_.city AS city_7, c0_.email_id AS email_id_8, c1_.user_id AS user_id_9 FROM cms_users c0_ LEFT JOIN cms_addresses c1_ ON c0_.id = c1_.user_id",
    );
}

#[test]
fn testDoesNotAddJoin() {
    assert_sql_generation(
        r"select a from Doctrine\Tests\Models\CMS\CmsAddress a",
        "SELECT c0_.id AS id_0, c0_.country AS country_1, c0_.zip AS zip_2, c0_.city AS city_3, c0_.user_id AS user_id_4 FROM cms_addresses c0_",
    );
}
