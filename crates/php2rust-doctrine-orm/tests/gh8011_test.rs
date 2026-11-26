//! GH8011Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite`: ordering with arithmetic expressions. `skipIfPostgres()`
//! never skips (the platform is `SQLite`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/GH8011Test.php

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

use php_value::Value;
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use company::*;
use orm::*;

async fn set_up() -> Arc<EntityManager> {
    register_listeners();
    let em = orm::set_up(&COMPANY_MODELS).await;
    generate_fixture(&em).await;
    em
}

async fn generate_fixture(em: &Arc<EntityManager>) {
    let person1 = company_employee("Benjamin E.", 200_000, "IT");

    let person2 = company_employee("Guilherme B.", 400_000, "IT2");

    em.persist(&person1).unwrap();
    em.persist(&person2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
}

/// The names of the result's employees, in order.
async fn names(dql: &str) -> Vec<Value> {
    let em = set_up().await;
    let result = em
        .create_query(dql)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();
    list(&result).iter().map(|p| get(&obj(p), "name")).collect()
}

#[tokio::test]
async fn testOrderWithArithmeticExpressionWithSingleValuedPathExpression() {
    let dql =
        r"SELECT p FROM Doctrine\Tests\Models\Company\CompanyEmployee p ORDER BY p.id + p.id ASC";

    let result = names(dql).await;

    assert_eq!(2, result.len());
    assert_eq!(s("Benjamin E."), result[0]);
    assert_eq!(s("Guilherme B."), result[1]);
}

#[tokio::test]
async fn testOrderWithArithmeticExpressionWithLiteralAndSingleValuedPathExpression() {
    let dql =
        r"SELECT p FROM Doctrine\Tests\Models\Company\CompanyEmployee p ORDER BY 1 + p.id ASC";

    let result = names(dql).await;

    assert_eq!(2, result.len());
    assert_eq!(s("Benjamin E."), result[0]);
    assert_eq!(s("Guilherme B."), result[1]);
}

#[tokio::test]
async fn testOrderWithArithmeticExpressionWithLiteralAndSingleValuedPathExpression2() {
    let dql =
        r"SELECT p FROM Doctrine\Tests\Models\Company\CompanyEmployee p ORDER BY ((1 + p.id)) ASC";

    let result = names(dql).await;

    assert_eq!(2, result.len());
    assert_eq!(s("Benjamin E."), result[0]);
    assert_eq!(s("Guilherme B."), result[1]);
}

#[tokio::test]
async fn testOrderWithArithmeticExpressionWithSingleValuedPathExpressionAndLiteral() {
    let dql =
        r"SELECT p FROM Doctrine\Tests\Models\Company\CompanyEmployee p ORDER BY p.id + 1 ASC";

    let result = names(dql).await;

    assert_eq!(2, result.len());
    assert_eq!(s("Benjamin E."), result[0]);
    assert_eq!(s("Guilherme B."), result[1]);
}

#[tokio::test]
async fn testOrderWithArithmeticExpressionWithResultVariableAndLiteral() {
    let dql = r"SELECT p, p.salary AS HIDDEN s FROM Doctrine\Tests\Models\Company\CompanyEmployee p ORDER BY s + 1 DESC";

    let result = names(dql).await;

    assert_eq!(2, result.len());
    assert_eq!(s("Guilherme B."), result[0]);
    assert_eq!(s("Benjamin E."), result[1]);
}

#[tokio::test]
async fn testOrderWithArithmeticExpressionWithResultVariableAndLiteral2() {
    let dql = r"SELECT p, p.salary AS HIDDEN s FROM Doctrine\Tests\Models\Company\CompanyEmployee p ORDER BY ((s + 1)) DESC";

    let result = names(dql).await;

    assert_eq!(2, result.len());
    assert_eq!(s("Guilherme B."), result[0]);
    assert_eq!(s("Benjamin E."), result[1]);
}

#[tokio::test]
async fn testOrderWithArithmeticExpressionWithLiteralAndResultVariable() {
    let dql = r"SELECT p, p.salary AS HIDDEN s FROM Doctrine\Tests\Models\Company\CompanyEmployee p ORDER BY 1 + s DESC";

    let result = names(dql).await;

    assert_eq!(2, result.len());
    assert_eq!(s("Guilherme B."), result[0]);
    assert_eq!(s("Benjamin E."), result[1]);
}

#[tokio::test]
async fn testOrderWithArithmeticExpressionWithLiteralAndResultVariable2() {
    let dql = r"SELECT p, p.salary AS HIDDEN s FROM Doctrine\Tests\Models\Company\CompanyEmployee p ORDER BY ((1 + s)) DESC";

    let result = names(dql).await;

    assert_eq!(2, result.len());
    assert_eq!(s("Guilherme B."), result[0]);
    assert_eq!(s("Benjamin E."), result[1]);
}

#[tokio::test]
async fn testOrderWithArithmeticExpressionWithResultVariableAndSingleValuedPathExpression() {
    let dql = r"SELECT p, p.salary AS HIDDEN s FROM Doctrine\Tests\Models\Company\CompanyEmployee p ORDER BY s + p.id DESC";

    let result = names(dql).await;

    assert_eq!(2, result.len());
    assert_eq!(s("Guilherme B."), result[0]);
    assert_eq!(s("Benjamin E."), result[1]);
}

#[tokio::test]
async fn testOrderWithArithmeticExpressionWithResultVariableAndSingleValuedPathExpression2() {
    let dql = r"SELECT p, p.salary AS HIDDEN s FROM Doctrine\Tests\Models\Company\CompanyEmployee p ORDER BY ((s + p.id)) DESC";

    let result = names(dql).await;

    assert_eq!(2, result.len());
    assert_eq!(s("Guilherme B."), result[0]);
    assert_eq!(s("Benjamin E."), result[1]);
}

#[tokio::test]
async fn testOrderWithArithmeticExpressionWithSingleValuedPathExpressionAndResultVariable() {
    let dql = r"SELECT p, p.salary AS HIDDEN s FROM Doctrine\Tests\Models\Company\CompanyEmployee p ORDER BY p.id + s DESC";

    let result = names(dql).await;

    assert_eq!(2, result.len());
    assert_eq!(s("Guilherme B."), result[0]);
    assert_eq!(s("Benjamin E."), result[1]);
}

#[tokio::test]
async fn testOrderWithArithmeticExpressionWithLiteralAndResultVariableUsingHiddenResultVariable() {
    let dql = r"SELECT p, 1 + p.salary AS HIDDEN _order FROM Doctrine\Tests\Models\Company\CompanyEmployee p ORDER BY _order DESC";

    let result = names(dql).await;

    assert_eq!(2, result.len());
    assert_eq!(s("Guilherme B."), result[0]);
    assert_eq!(s("Benjamin E."), result[1]);
}
