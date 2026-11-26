//! AdvancedDqlQueryTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` with the `company` model set. `assertEquals()` compares
//! loosely (`assert_loose_eq`). `assertGreaterThan(0, $array)` holds for
//! any array in PHP, so `testUpdateAs` checks that the result is one.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/AdvancedDqlQueryTest.php

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
    // new CompanyCar('Caramba')
    let car = new_object(COMPANY_CAR);
    set(&car, "brand", s("Caramba"));

    let manager1 = company_manager("Roman B.", 100_000, "IT", "Foo");
    set(&manager1, "car", Value::Object(car.clone()));

    let person2 = company_employee("Benjamin E.", 200_000, "IT");

    let person3 = company_employee("Guilherme B.", 400_000, "IT2");

    let person4 = company_employee("Jonathan W.", 800_000, "IT2");

    set_spouse(&person2, &person3);

    add_friend(&manager1, &person4).await;
    add_friend(&person2, &person3).await;
    add_friend(&person2, &person4).await;
    add_friend(&person3, &person4).await;

    em.persist(&car).unwrap();
    em.persist(&manager1).unwrap();
    em.persist(&person2).unwrap();
    em.persist(&person3).unwrap();
    em.persist(&person4).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
}

async fn scalar_result(em: &Arc<EntityManager>, dql: &str) -> Vec<Value> {
    list(&em.create_query(dql).get_scalar_result().await.unwrap())
}

#[tokio::test]
async fn testAggregateWithHavingClause() {
    let em = set_up().await;
    let dql = r"SELECT p.department, AVG(p.salary) AS avgSalary FROM Doctrine\Tests\Models\Company\CompanyEmployee p GROUP BY p.department HAVING SUM(p.salary) > 200000 ORDER BY p.department";

    let result = scalar_result(&em, dql).await;

    assert_eq!(2, result.len());
    assert_loose_eq(&s("IT"), &at(&result[0], "department"));
    assert_loose_eq(&Value::Int(150_000), &at(&result[0], "avgSalary"));
    assert_loose_eq(&s("IT2"), &at(&result[1], "department"));
    assert_loose_eq(&Value::Int(600_000), &at(&result[1], "avgSalary"));
}

#[tokio::test]
async fn testCommentsInDQL() {
    let em = set_up().await;
    //same test than testAggregateWithHavingClause but with comments into the DQL
    let dql = "SELECT p.department, AVG(p.salary) AS avgSalary -- comment end of line
-- comment with 'strange chars', & $
  FROM Doctrine\\Tests\\Models\\Company\\CompanyEmployee p
  -- comment beginning of line GROUP BY
GROUP BY p.department HAVING SUM(p.salary) > 200000 ORDER BY p.department -- comment end of line";

    let result = scalar_result(&em, dql).await;

    assert_eq!(2, result.len());
    assert_loose_eq(&s("IT"), &at(&result[0], "department"));
    assert_loose_eq(&Value::Int(150_000), &at(&result[0], "avgSalary"));
    assert_loose_eq(&s("IT2"), &at(&result[1], "department"));
    assert_loose_eq(&Value::Int(600_000), &at(&result[1], "avgSalary"));
}

#[tokio::test]
async fn testUnnamedScalarResultsAreOneBased() {
    let em = set_up().await;
    let dql = r"SELECT p.department, AVG(p.salary) FROM Doctrine\Tests\Models\Company\CompanyEmployee p GROUP BY p.department HAVING SUM(p.salary) > 200000 ORDER BY p.department";

    let result = scalar_result(&em, dql).await;

    assert_eq!(2, result.len());
    assert_loose_eq(&Value::Int(150_000), &at(&result[0], 1));
    assert_loose_eq(&Value::Int(600_000), &at(&result[1], 1));
}

#[tokio::test]
async fn testOrderByResultVariableCollectionSize() {
    let em = set_up().await;
    let dql = r"SELECT p.name, size(p.friends) AS friends FROM Doctrine\Tests\Models\Company\CompanyPerson p WHERE p.friends IS NOT EMPTY ORDER BY friends DESC, p.name DESC";

    let result = scalar_result(&em, dql).await;

    assert_eq!(4, result.len());

    assert_loose_eq(&s("Jonathan W."), &at(&result[0], "name"));
    assert_loose_eq(&Value::Int(3), &at(&result[0], "friends"));

    assert_loose_eq(&s("Guilherme B."), &at(&result[1], "name"));
    assert_loose_eq(&Value::Int(2), &at(&result[1], "friends"));

    assert_loose_eq(&s("Benjamin E."), &at(&result[2], "name"));
    assert_loose_eq(&Value::Int(2), &at(&result[2], "friends"));

    assert_loose_eq(&s("Roman B."), &at(&result[3], "name"));
    assert_loose_eq(&Value::Int(1), &at(&result[3], "friends"));
}

fn assert_case_order(result: &[Value]) {
    assert_eq!(4, result.len());

    assert_loose_eq(&s("Benjamin E."), &at(&result[0], "name"));
    assert_loose_eq(&s("Guilherme B."), &at(&result[1], "name"));
    assert_loose_eq(&s("Roman B."), &at(&result[2], "name"));
    assert_loose_eq(&s("Jonathan W."), &at(&result[3], "name"));
}

#[tokio::test]
async fn testOrderBySimpleCaseExpression() {
    let em = set_up().await;
    let dql = r"            SELECT p.name
            FROM Doctrine\Tests\Models\Company\CompanyEmployee p
            ORDER BY CASE p.name
            WHEN 'Jonathan W.' THEN 1
            WHEN 'Roman B.' THEN 2
            WHEN 'Guilherme B.' THEN 3
            ELSE 4
            END DESC";

    let result = scalar_result(&em, dql).await;

    assert_case_order(&result);
}

#[tokio::test]
async fn testOrderByGeneralCaseExpression() {
    let em = set_up().await;
    let dql = r"            SELECT p.name
            FROM Doctrine\Tests\Models\Company\CompanyEmployee p
            ORDER BY CASE
            WHEN p.name='Jonathan W.' THEN 1
            WHEN p.name='Roman B.' THEN 2
            WHEN p.name='Guilherme B.' THEN 3
            ELSE 4
            END DESC";

    let result = scalar_result(&em, dql).await;

    assert_case_order(&result);
}

#[tokio::test]
async fn testIsNullAssociation() {
    let em = set_up().await;
    let dql = r"SELECT p FROM Doctrine\Tests\Models\Company\CompanyPerson p WHERE p.spouse IS NULL";
    let query = em.create_query(dql);
    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(2, result.len());
    for person in &result {
        let person = obj(person);
        assert!(matches!(get(&person, "id"), Value::Int(id) if id > 0));
        assert!(get(&person, "spouse").is_null());
    }

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;
}

#[tokio::test]
async fn testSelectSubselect() {
    let em = set_up().await;
    let dql = r"SELECT p, (SELECT c.brand FROM Doctrine\Tests\Models\Company\CompanyCar c WHERE p.car = c) brandName FROM Doctrine\Tests\Models\Company\CompanyManager p";
    let query = em.create_query(dql);
    let result = list(&query.get_array_result().await.unwrap());

    assert_eq!(1, result.len());
    assert_loose_eq(&s("Caramba"), &at(&result[0], "brandName"));

    em.clear().unwrap();
}

#[tokio::test]
async fn testInSubselect() {
    let em = set_up().await;
    let dql = r"SELECT p.name FROM Doctrine\Tests\Models\Company\CompanyPerson p
WHERE p.name IN (SELECT n.name FROM Doctrine\Tests\Models\Company\CompanyPerson n WHERE n.name = 'Roman B.')";
    let query = em.create_query(dql);
    let result = list(&query.get_scalar_result().await.unwrap());

    assert_eq!(1, result.len());
    assert_loose_eq(&s("Roman B."), &at(&result[0], "name"));

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;
}

#[tokio::test]
async fn testGroupByMultipleFields() {
    let em = set_up().await;
    let dql = r"SELECT p.department, p.name, count(p.id) FROM Doctrine\Tests\Models\Company\CompanyEmployee p GROUP BY p.department, p.name";
    let query = em.create_query(dql);
    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(4, result.len());

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;
}

#[tokio::test]
async fn testUpdateAs() {
    let em = set_up().await;
    let dql = r"UPDATE Doctrine\Tests\Models\Company\CompanyEmployee AS p SET p.salary = 1";
    em.create_query(dql).execute(None, None).await.unwrap();

    let query = em.create_query(
        r"SELECT count(p.id) FROM Doctrine\Tests\Models\Company\CompanyEmployee p WHERE p.salary = 1",
    );
    // assertGreaterThan(0, $array): an array is greater than any int.
    assert!(matches!(
        query.get_result(HYDRATE_OBJECT).await.unwrap(),
        Value::Array(_)
    ));

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;
}

#[tokio::test]
async fn testDeleteAs() {
    let em = set_up().await;
    let dql = r"DELETE Doctrine\Tests\Models\Company\CompanyEmployee AS p";
    em.create_query(dql)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    let dql = r"SELECT count(p) FROM Doctrine\Tests\Models\Company\CompanyEmployee p";
    let result = em
        .create_query(dql)
        .get_single_scalar_result()
        .await
        .unwrap();

    assert_loose_eq(&Value::Int(0), &result);
}
