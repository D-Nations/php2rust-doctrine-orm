//! Ticket/DDC331Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the company models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC331Test.php

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

use std::sync::Arc;

use php2rust_doctrine_orm::entity_manager::EntityManager;

async fn set_up() -> Arc<EntityManager> {
    company::register_listeners();
    orm::set_up(&company::COMPANY_MODELS).await
}

#[tokio::test]
async fn testSelectFieldOnRootEntity() {
    let em = set_up().await;
    let q =
        em.create_query("SELECT e.name FROM Doctrine\\Tests\\Models\\Company\\CompanyEmployee e");
    assert_eq!(
        "SELECT c0_.name AS name_0 FROM company_employees c1_ INNER JOIN company_persons c0_ ON c1_.id = c0_.id LEFT JOIN company_managers c2_ ON c1_.id = c2_.id".to_lowercase(),
        q.get_single_sql().unwrap().to_lowercase(),
    );
}
