//! Ticket/DDC2655Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2655Test.php

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

use php_value::Value;
use php2rust_doctrine_orm::abstract_query::HYDRATE_SINGLE_SCALAR;

#[tokio::test]
async fn testSingleScalarOneOrNullResult() {
    let em = orm::set_up(&orm::CMS_MODELS).await;

    let query = em.create_query(
        "SELECT u.name FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u.username = 'happy_doctrine_user'",
    );
    assert_eq!(
        Value::Null,
        query
            .get_one_or_null_result(Some(HYDRATE_SINGLE_SCALAR))
            .await
            .unwrap()
    );
}
