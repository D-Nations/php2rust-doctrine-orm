//! Ticket/DDC3719Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the company models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3719Test.php

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
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_orm::collection_object as coll;

use company::{COMPANY_FLEX_CONTRACT, company_contract, company_manager, mark_completed};
use orm::{append, get};

#[tokio::test]
async fn testCriteriaOnNotOwningSide() {
    let em = set_up().await;
    let manager = company_manager("Gandalf", 666, "Marketing", "Boss");
    em.persist(&manager).unwrap();

    let contract_a = company_contract(COMPANY_FLEX_CONTRACT);
    mark_completed(&contract_a);
    append(&contract_a, "managers", Value::Object(manager.clone()));
    em.persist(&contract_a).unwrap();

    let contract_b = company_contract(COMPANY_FLEX_CONTRACT);
    append(&contract_b, "managers", Value::Object(manager.clone()));
    em.persist(&contract_b).unwrap();

    em.flush().await.unwrap();
    em.refresh(&manager).await.unwrap();

    let contracts = get(&manager, "managedContracts");
    assert_eq!(2, coll::count(&contracts).await.unwrap());

    // `Criteria::ASC` is not defined in collections 3: `Criteria::create()`.
    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("completed", Value::Bool(true)));

    let completed_contracts = coll::matching(&contracts, &criteria).await.unwrap();
    assert_eq!(1, coll::count(&completed_contracts).await.unwrap());
}
