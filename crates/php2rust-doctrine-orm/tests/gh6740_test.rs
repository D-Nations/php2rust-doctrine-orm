//! Ticket/GH6740Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the ecommerce models).
//!
//! `Criteria::ASC` is not defined in collections 3: `Criteria::create()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6740Test.php

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

use std::sync::Arc;

use php_value::Value;
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_collections::expr::Comparison;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

struct Fixture {
    em: Arc<EntityManager>,
    product_id: Value,
    first_category_id: Value,
    second_category_id: Value,
}

async fn set_up() -> Fixture {
    let em = orm::set_up(&ECOMMERCE_MODELS).await;

    let product = ecommerce_product("First Product");

    let first_category = ecommerce_category("Business");
    let second_category = ecommerce_category("Home");

    add_category(&product, &first_category).await;
    add_category(&product, &second_category).await;

    em.persist(&product).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    Fixture {
        em,
        product_id: get(&product, "id"),
        first_category_id: get(&first_category, "id"),
        second_category_id: get(&second_category, "id"),
    }
}

impl Fixture {
    /// `count($product->getCategories()->matching($criteria))`.
    async fn matching_count(&self, expr: Comparison) -> usize {
        let product = self
            .em
            .find(ECOMMERCE_PRODUCT, self.product_id.clone())
            .await
            .unwrap()
            .unwrap();
        let mut criteria = Criteria::create();
        criteria.r#where(expr);

        let matched = coll::matching(&get(&product, "categories"), &criteria)
            .await
            .unwrap();
        coll::count(&matched).await.unwrap()
    }
}

#[tokio::test]
async fn testCollectionFilteringLteOperator() {
    let f = set_up().await;
    let expr = Criteria::expr().lte("id", f.second_category_id.clone());
    assert_eq!(2, f.matching_count(expr).await);
}

#[tokio::test]
async fn testCollectionFilteringLtOperator() {
    let f = set_up().await;
    let expr = Criteria::expr().lt("id", f.second_category_id.clone());
    assert_eq!(1, f.matching_count(expr).await);
}

#[tokio::test]
async fn testCollectionFilteringGteOperator() {
    let f = set_up().await;
    let expr = Criteria::expr().gte("id", f.first_category_id.clone());
    assert_eq!(2, f.matching_count(expr).await);
}

#[tokio::test]
async fn testCollectionFilteringGtOperator() {
    let f = set_up().await;
    let expr = Criteria::expr().gt("id", f.first_category_id.clone());
    assert_eq!(1, f.matching_count(expr).await);
}

#[tokio::test]
async fn testCollectionFilteringEqualsOperator() {
    let f = set_up().await;
    let expr = Criteria::expr().eq("id", f.first_category_id.clone());
    assert_eq!(1, f.matching_count(expr).await);
}
