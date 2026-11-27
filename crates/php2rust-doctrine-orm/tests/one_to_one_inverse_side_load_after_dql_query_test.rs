//! Port of `OneToOneInverseSideLoadAfterDqlQueryTest`, on `SQLite` with the statements logged
//! (`tests/common/query_log.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OneToOneInverseSideLoadAfterDqlQueryTest.php

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
#[path = "common/query_log.rs"]
mod query_log;

use php_value::Value;

use orm::{get_object, new_object, s, set};
use query_log::QueryLog;

const NS: &str = "Doctrine\\Tests\\Models\\OneToOneInverseSideLoad\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// `$this->assertSQLEquals($expectedSql, $actualSql)`.
#[track_caller]
fn assert_sql_equals(expected_sql: &str, actual_sql: &str) {
    assert_eq!(
        expected_sql.to_lowercase(),
        actual_sql.to_lowercase(),
        "Lowercase comparison of SQL statements failed."
    );
}

/// `$this->getLastLoggedQuery($index)['sql']`.
fn get_last_logged_query(log: &QueryLog, index: usize) -> String {
    let queries = log.queries();
    queries[queries.len() - 1 - index].clone()
}

#[tokio::test]
async fn testInverseSideOneToOneLoadedAfterDqlQuery() {
    let (em, log) = query_log::set_up_logged(&[&class("OwningSide"), &class("InverseSide")]).await;
    log.reset_and_enable();

    let owner = new_object(&class("OwningSide"));
    let inverse = new_object(&class("InverseSide"));

    set(&owner, "id", s("owner"));
    set(&inverse, "id", s("inverse"));
    set(&owner, "inverse", Value::Object(inverse.clone()));
    set(&inverse, "owning", Value::Object(owner.clone()));

    em.persist(&owner).unwrap();
    em.persist(&inverse).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let mut qb = em.create_query_builder();
    qb.select(vec![s("inverse")])
        .unwrap()
        .from(&class("InverseSide"), "inverse", None)
        .unwrap()
        .and_where(vec![s("inverse.id = :id")])
        .unwrap()
        .set_parameter("id", s("inverse"), None);
    let fetched_inverse = orm::obj(
        &qb.get_query()
            .unwrap()
            .get_single_result(None)
            .await
            .unwrap(),
    );

    assert_eq!(fetched_inverse.class(), class("InverseSide"));
    assert_eq!(
        get_object(&fetched_inverse, "owning").unwrap().class(),
        class("OwningSide")
    );

    assert_sql_equals(
        "select o0_.id as id_0 from one_to_one_inverse_side_load_inverse o0_ where o0_.id = ?",
        &get_last_logged_query(&log, 1),
    );

    assert_sql_equals(
        "select t0.id as id_1, t0.inverse as inverse_2 from one_to_one_inverse_side_load_owning t0 WHERE t0.inverse = ?",
        &get_last_logged_query(&log, 0),
    );
}
