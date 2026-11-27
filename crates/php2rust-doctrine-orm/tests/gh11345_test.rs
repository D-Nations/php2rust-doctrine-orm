//! Port of `GH11345Test`, on `SQLite` with the `company` model set and the
//! statements logged (`tests/common/query_log.rs`).
//!
//! Covers: <https://github.com/doctrine/orm/issues/11345>
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/GH11345Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/company.rs"]
mod company;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/query_log.rs"]
mod query_log;

use std::sync::Arc;

use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use company::{
    COMPANY_AUCTION, COMPANY_EMPLOYEE, COMPANY_MANAGER, COMPANY_MODELS, company_employee,
    company_event, company_manager,
};
use orm::{get, s, set};
use query_log::QueryLog;

struct Fixture {
    em: Arc<EntityManager>,
    log: Arc<QueryLog>,
    employee_id: Value,
    manager_id: Value,
    auction1_id: Value,
    auction2_id: Value,
    auction3_id: Value,
}

/// `setUp()`: the schema and `generateFixture()`.
async fn set_up() -> Fixture {
    let (em, log) = query_log::set_up_logged(&COMPANY_MODELS).await;

    let employee = company_employee("name", 50_000, "department");
    let manager = company_manager("manager name", 80_000, "management", "manager title");
    let auction1 = company_event(COMPANY_AUCTION, "lorem ipsum");
    let auction2 = company_event(COMPANY_AUCTION, "dolor sit");
    let auction3 = company_event(COMPANY_AUCTION, "dolor consectetur");

    for entity in [&employee, &manager, &auction1, &auction2, &auction3] {
        em.persist(entity).unwrap();
    }
    em.flush().await.unwrap();

    let fixture = Fixture {
        em: Arc::clone(&em),
        log,
        employee_id: get(&employee, "id"),
        manager_id: get(&manager, "id"),
        auction1_id: get(&auction1, "id"),
        auction2_id: get(&auction2, "id"),
        auction3_id: get(&auction3, "id"),
    };

    em.clear().unwrap();
    fixture
}

async fn find(em: &EntityManager, class: &str, id: &Value) -> Object {
    em.find(class, id.clone())
        .await
        .unwrap()
        .expect("Could not find expected entity")
}

/// The `UPDATE` statements of the log, with their bound values.
fn updates(log: &QueryLog) -> Vec<(String, Vec<Value>)> {
    log.entries()
        .into_iter()
        .filter(|(sql, _, _)| sql.to_uppercase().starts_with("UPDATE"))
        .map(|(sql, params, _)| (sql, params.iter().map(|(_, v)| v.clone()).collect()))
        .collect()
}

#[tokio::test]
async fn testUpdateSqlStatementsAreIssuedInAlphabeticOrderByEntityClass() {
    let f = set_up().await;
    // Load entities in an order that is not alphabetical
    let manager = find(&f.em, COMPANY_MANAGER, &f.manager_id).await;
    let employee = find(&f.em, COMPANY_EMPLOYEE, &f.employee_id).await;
    let auction = find(&f.em, COMPANY_AUCTION, &f.auction1_id).await;

    // Make some changes to the entities
    set(&manager, "title", s("new title"));
    set(&employee, "salary", Value::Int(100_000));
    set(&auction, "data", s("new data"));

    // Reset the log so only the flush() queries are captured.
    f.log.reset_and_enable();

    f.em.flush().await.unwrap();

    // Collect only UPDATE statements in execution order.
    let update_sqls: Vec<String> = updates(&f.log).into_iter().map(|(sql, _)| sql).collect();

    assert_eq!(update_sqls.len(), 3, "{update_sqls:#?}");

    // Assert that sqls were run in alphabetic order by the class names (which also results in the same order by the
    // table names)
    assert!(
        update_sqls[0].contains("company_auctions"),
        "{}",
        update_sqls[0]
    );
    assert!(
        update_sqls[1].contains("company_employees"),
        "{}",
        update_sqls[1]
    );
    assert!(
        update_sqls[2].contains("company_managers"),
        "{}",
        update_sqls[2]
    );
}

#[tokio::test]
async fn testUpdateSqlStatementsAreIssuedInAlphabeticOrderByEntityIdForSameClassName() {
    let f = set_up().await;
    // Load entities in an order that is not alphabetical
    let auction2 = find(&f.em, COMPANY_AUCTION, &f.auction2_id).await;
    let auction1 = find(&f.em, COMPANY_AUCTION, &f.auction1_id).await;
    let auction3 = find(&f.em, COMPANY_AUCTION, &f.auction3_id).await;

    // Make some changes to the entities
    set(&auction1, "data", s("new data"));
    set(&auction2, "data", s("new data"));
    set(&auction3, "data", s("new data"));

    // Reset the log so only the flush() queries are captured.
    f.log.reset_and_enable();

    f.em.flush().await.unwrap();

    // Collect only UPDATE statements in execution order.
    let updates = updates(&f.log);

    assert_eq!(updates.len(), 3, "{updates:#?}");

    // Assert that the params of each UPDATE contain the expected ID in ascending order.
    for (update, id) in updates
        .iter()
        .zip([&f.auction1_id, &f.auction2_id, &f.auction3_id])
    {
        assert!(
            update.1.iter().any(|v| v.identical(id)),
            "{update:?} has {id:?}"
        );
    }
}
