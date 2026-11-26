//! Ticket/DDC1041Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the company models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1041Test.php

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

use php_value::Value;

use company::{COMPANY_FIX_CONTRACT, COMPANY_FLEX_CONTRACT, company_contract};
use orm::{get, set};

#[tokio::test]
async fn testGrabWrongSubtypeReturnsNull() {
    let em = set_up().await;
    let fix = company_contract(COMPANY_FIX_CONTRACT);
    set(&fix, "fixPrice", Value::Int(2000));

    em.persist(&fix).unwrap();
    em.flush().await.unwrap();

    let id = get(&fix, "id");

    assert!(
        em.find(COMPANY_FLEX_CONTRACT, id.clone())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        em.get_reference(COMPANY_FLEX_CONTRACT, id)
            .await
            .unwrap()
            .is_none()
    );
}
