//! QueryBuilderTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The test entity manager is `getTestEntityManager()`
//! (`tests/common/dql.rs`), behind an `Arc` as the builder holds it. The
//! variadic arguments are `Vec`s of values (`args![...]`); an omitted
//! `$order` is `None`, an explicit `null` `Some(Order::Null)`.
//! `getQuery()` gives the `Query` stand-in (`StandInQuery`). PHPUnit's
//! `assertEquals()` of parameter collections compares each parameter's
//! name, value, type and whether the type was specified.
//! `testWhereWithUnexpectedNamedArguments` has no counterpart: Rust has no
//! named arguments.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/QueryBuilderTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arc_with_non_send_sync,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/dql.rs"]
mod dql;
#[path = "common/models.rs"]
mod models;

use std::sync::Arc;

use php_value::compare::loose_equals;
use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_collections::SortDirection as CriteriaSortDirection;
use php2rust_doctrine_dbal::WrapperParameterType;
use php2rust_doctrine_orm::object::EntityObject;
use php2rust_doctrine_orm::query::context::{DqlQuery, Hint, QueryContext};
use php2rust_doctrine_orm::query::expr::{self, Andx, Join, ORDER_DEPRECATION, Order};
use php2rust_doctrine_orm::query::{Parameter, ParameterTypeInferer, SortDirection, StandInQuery};
use php2rust_doctrine_orm::{QueryBuilder, QueryType};

use dql::{ExpectDeprecation, TestEntityManager, thrown};

const CMS_USER: &str = r"Doctrine\Tests\Models\CMS\CmsUser";
const CMS_ARTICLE: &str = r"Doctrine\Tests\Models\CMS\CmsArticle";
const CMS_GROUP: &str = r"Doctrine\Tests\Models\CMS\CmsGroup";
const STATE: &str = r"Doctrine\Tests\Models\Cache\State";
/// `Cache::MODE_REFRESH`.
const CACHE_MODE_REFRESH: i64 = 4;

type TestResult = Result<(), Thrown>;

macro_rules! args {
    ($($arg:expr),* $(,)?) => {
        vec![$(Value::from($arg)),*]
    };
}

struct SetUp {
    entity_manager: Arc<TestEntityManager>,
}

fn set_up() -> SetUp {
    SetUp {
        entity_manager: Arc::new(TestEntityManager::new()),
    }
}

impl SetUp {
    /// `$this->entityManager->createQueryBuilder()`.
    fn create_query_builder(&self) -> QueryBuilder {
        QueryBuilder::new(Arc::clone(&self.entity_manager) as Arc<dyn QueryContext>)
    }
}

fn assert_valid_query_builder(qb: &QueryBuilder, expected_dql: &str) {
    let dql = qb.get_dql().unwrap();
    let _q = qb.get_query().unwrap();

    assert_eq!(expected_dql, dql);
}

/// `(string) $value`.
fn s(value: &Value) -> String {
    String::from_utf8(expr::to_string(value).unwrap().as_bytes().to_vec()).unwrap()
}

fn count(value: &Value) -> usize {
    value.as_array().expect("an array").len()
}

/// `assertEquals()` of two parameter collections.
fn assert_parameters_equal(expected: &[Parameter], actual: &[Parameter]) {
    assert_eq!(expected.len(), actual.len());
    for (e, a) in expected.iter().zip(actual) {
        assert_parameter_equal(e, a);
    }
}

fn assert_parameter_equal(expected: &Parameter, actual: &Parameter) {
    assert_eq!(expected.get_name(), actual.get_name());
    assert!(loose_equals(expected.get_value(), actual.get_value()));
    assert_eq!(expected.get_type(), actual.get_type());
    assert_eq!(expected.type_was_specified(), actual.type_was_specified());
}

fn expect_thrown(result: Result<impl std::fmt::Debug, Thrown>, class: &str, message: &str) {
    let error = result.unwrap_err();
    assert_eq!((class.to_owned(), message.to_owned()), thrown(&error));
}

#[test]
fn testSelectSetsType() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.delete(Some(CMS_USER), Some("u"))?
        .select(args!["u.id", "u.username"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u.id, u.username FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
    Ok(())
}

#[test]
fn testDeleteSetsType() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.from(CMS_USER, "u", None)?.delete(None, None)?;

    assert_valid_query_builder(&qb, r"DELETE Doctrine\Tests\Models\CMS\CmsUser u");
    Ok(())
}

#[test]
fn testDeleteWithLimitNotSupported() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let result = qb
        .delete(Some(CMS_USER), Some("c"))?
        .set_max_results(Some(1))
        .map(|_| ());
    expect_thrown(
        result,
        "RuntimeException",
        "Setting a limit is not supported for delete or update queries.",
    );
    Ok(())
}

#[test]
fn testUpdateWithLimitNotSupported() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let result = qb
        .update(Some(CMS_USER), Some("c"))?
        .set_max_results(Some(1))
        .map(|_| ());
    expect_thrown(
        result,
        "RuntimeException",
        "Setting a limit is not supported for delete or update queries.",
    );
    Ok(())
}

#[test]
fn testUpdateSetsType() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.from(CMS_USER, "u", None)?.update(None, None)?;

    assert_valid_query_builder(&qb, r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u");
    Ok(())
}

#[test]
fn testSimpleSelect() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.from(CMS_USER, "u", None)?
        .select(args!["u.id", "u.username"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u.id, u.username FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
    Ok(())
}

#[test]
fn testSimpleSelectArray() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.from(CMS_USER, "u", None)?
        .select(vec![Value::Array(PhpArray::list(args![
            "u.id",
            "u.username"
        ]))])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u.id, u.username FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
    Ok(())
}

#[test]
fn testSimpleDelete() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.delete(Some(CMS_USER), Some("u"))?;

    assert_valid_query_builder(&qb, r"DELETE Doctrine\Tests\Models\CMS\CmsUser u");
    Ok(())
}

#[test]
fn testSimpleSelectWithFromIndexBy() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.from(CMS_USER, "u", Some("u.id"))?
        .select(args!["u.id", "u.username"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u.id, u.username FROM Doctrine\Tests\Models\CMS\CmsUser u INDEX BY u.id",
    );
    Ok(())
}

#[test]
fn testSimpleSelectWithIndexBy() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.from(CMS_USER, "u", None)?
        .index_by("u", "u.id")?
        .select(args!["u.id", "u.username"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u.id, u.username FROM Doctrine\Tests\Models\CMS\CmsUser u INDEX BY u.id",
    );
    Ok(())
}

#[test]
fn testSimpleUpdate() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.update(Some(CMS_USER), Some("u"))?
        .set("u.username", ":username")?;

    assert_valid_query_builder(
        &qb,
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.username = :username",
    );
    Ok(())
}

#[test]
fn testInnerJoin() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u", "a"])?
        .from(CMS_USER, "u", None)?
        .inner_join("u.articles", "a", None, None, None)?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.articles a",
    );
    Ok(())
}

#[test]
fn testComplexInnerJoin() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u", "a"])?
        .from(CMS_USER, "u", None)?
        .inner_join(
            "u.articles",
            "a",
            Some("ON"),
            Some(Value::from("u.id = a.author_id")),
            None,
        )?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.articles a ON u.id = a.author_id",
    );
    Ok(())
}

#[test]
fn testComplexInnerJoinWithComparisonCondition() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let condition = qb.expr().eq("u.id", "a.author_id");
    qb.select(args!["u", "a"])?
        .from(CMS_USER, "u", None)?
        .inner_join(
            CMS_ARTICLE,
            "a",
            Some(Join::ON),
            Some(condition.into()),
            None,
        )?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN Doctrine\Tests\Models\CMS\CmsArticle a ON u.id = a.author_id",
    );
    Ok(())
}

#[test]
fn testComplexInnerJoinWithCompositeCondition() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let condition = qb.expr().and_x(args![
        qb.expr().eq("u.id", "a.author_id"),
        qb.expr().is_not_null("u.name"),
    ])?;
    qb.select(args!["u", "a"])?
        .from(CMS_USER, "u", None)?
        .inner_join(
            "u.articles",
            "a",
            Some(Join::ON),
            Some(condition.into()),
            None,
        )?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.articles a ON u.id = a.author_id AND u.name IS NOT NULL",
    );
    Ok(())
}

#[test]
fn testComplexInnerJoinWithFuncCondition() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let condition = qb
        .expr()
        .r#in("u.id", Value::Array(PhpArray::list(args![1, 2, 3])))?;
    qb.select(args!["u", "a"])?
        .from(CMS_USER, "u", None)?
        .inner_join(
            "u.articles",
            "a",
            Some(Join::WITH),
            Some(condition.into()),
            None,
        )?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.articles a WITH u.id IN(1, 2, 3)",
    );
    Ok(())
}

#[test]
fn testComplexInnerJoinWithIndexBy() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u", "a"])?
        .from(CMS_USER, "u", None)?
        .inner_join(
            "u.articles",
            "a",
            Some("ON"),
            Some(Value::from("u.id = a.author_id")),
            Some("a.name"),
        )?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.articles a INDEX BY a.name ON u.id = a.author_id",
    );
    Ok(())
}

#[test]
fn testLeftJoin() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u", "a"])?
        .from(CMS_USER, "u", None)?
        .left_join("u.articles", "a", None, None, None)?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.articles a",
    );
    Ok(())
}

#[test]
fn testLeftJoinWithIndexBy() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u", "a"])?
        .from(CMS_USER, "u", None)?
        .left_join("u.articles", "a", None, None, Some("a.name"))?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.articles a INDEX BY a.name",
    );
    Ok(())
}

#[test]
fn testMultipleFrom() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u", "g"])?
        .from(CMS_USER, "u", None)?
        .from(CMS_GROUP, "g", None)?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u, g FROM Doctrine\Tests\Models\CMS\CmsUser u, Doctrine\Tests\Models\CMS\CmsGroup g",
    );
    Ok(())
}

#[test]
fn testMultipleFromWithIndexBy() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u", "g"])?
        .from(CMS_USER, "u", None)?
        .from(CMS_GROUP, "g", None)?
        .index_by("g", "g.id")?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u, g FROM Doctrine\Tests\Models\CMS\CmsUser u, Doctrine\Tests\Models\CMS\CmsGroup g INDEX BY g.id",
    );
    Ok(())
}

#[test]
fn testMultipleFromWithJoin() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u", "g"])?
        .from(CMS_USER, "u", None)?
        .from(CMS_GROUP, "g", None)?
        .inner_join(
            "u.articles",
            "a",
            Some("ON"),
            Some(Value::from("u.id = a.author_id")),
            None,
        )?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u, g FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.articles a ON u.id = a.author_id, Doctrine\Tests\Models\CMS\CmsGroup g",
    );
    Ok(())
}

#[test]
fn testMultipleFromWithMultipleJoin() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u", "g"])?
        .from(CMS_USER, "u", None)?
        .from(CMS_ARTICLE, "a", None)?
        .inner_join("u.groups", "g", None, None, None)?
        .left_join("u.address", "ad", None, None, None)?
        .inner_join("a.comments", "c", None, None, None)?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u, g FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.groups g LEFT JOIN u.address ad, Doctrine\Tests\Models\CMS\CmsArticle a INNER JOIN a.comments c",
    );
    Ok(())
}

#[test]
fn testWhere() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :uid"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :uid",
    );
    Ok(())
}

#[test]
fn testComplexAndWhere() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :uid OR u.id = :uid2 OR u.id = :uid3"])?
        .and_where(args!["u.name = :name"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE (u.id = :uid OR u.id = :uid2 OR u.id = :uid3) AND u.name = :name",
    );
    Ok(())
}

#[test]
fn testAndWhere() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :uid"])?
        .and_where(args!["u.id = :uid2"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :uid AND u.id = :uid2",
    );
    Ok(())
}

#[test]
fn testOrWhere() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :uid"])?
        .or_where(args!["u.id = :uid2"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :uid OR u.id = :uid2",
    );
    Ok(())
}

#[test]
fn testComplexAndWhereOrWhereNesting() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :uid"])?
        .or_where(args!["u.id = :uid2"])?
        .and_where(args!["u.id = :uid3"])?
        .or_where(args!["u.name = :name1", "u.name = :name2"])?
        .and_where(args!["u.name <> :noname"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE (((u.id = :uid OR u.id = :uid2) AND u.id = :uid3) OR u.name = :name1 OR u.name = :name2) AND u.name <> :noname",
    );
    Ok(())
}

fn one_two_three() -> Value {
    Value::Array(PhpArray::list(args![1, 2, 3]))
}

#[test]
fn testAndWhereIn() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let in_ = qb.expr().r#in("u.id", one_two_three())?;
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :uid"])?
        .and_where(args![in_])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :uid AND u.id IN(1, 2, 3)",
    );
    Ok(())
}

#[test]
fn testOrWhereIn() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let in_ = qb.expr().r#in("u.id", one_two_three())?;
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :uid"])?
        .or_where(args![in_])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :uid OR u.id IN(1, 2, 3)",
    );
    Ok(())
}

#[test]
fn testAndWhereNotIn() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let not_in = qb.expr().not_in("u.id", one_two_three())?;
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :uid"])?
        .and_where(args![not_in])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :uid AND u.id NOT IN(1, 2, 3)",
    );
    Ok(())
}

#[test]
fn testOrWhereNotIn() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let not_in = qb.expr().not_in("u.id", one_two_three())?;
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :uid"])?
        .or_where(args![not_in])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :uid OR u.id NOT IN(1, 2, 3)",
    );
    Ok(())
}

#[test]
fn testGroupBy() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .group_by(&["u.id"])?
        .add_group_by(&["u.username"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u GROUP BY u.id, u.username",
    );
    Ok(())
}

#[test]
fn testHaving() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .group_by(&["u.id"])?
        .having(args!["COUNT(u.id) > 1"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u GROUP BY u.id HAVING COUNT(u.id) > 1",
    );
    Ok(())
}

#[test]
fn testAndHaving() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .group_by(&["u.id"])?
        .having(args!["COUNT(u.id) > 1"])?
        .and_having(args!["COUNT(u.id) < 1"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u GROUP BY u.id HAVING COUNT(u.id) > 1 AND COUNT(u.id) < 1",
    );
    Ok(())
}

#[test]
fn testOrHaving() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .group_by(&["u.id"])?
        .having(args!["COUNT(u.id) > 1"])?
        .and_having(args!["COUNT(u.id) < 1"])?
        .or_having(args!["COUNT(u.id) > 1"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u GROUP BY u.id HAVING (COUNT(u.id) > 1 AND COUNT(u.id) < 1) OR COUNT(u.id) > 1",
    );
    Ok(())
}

#[test]
fn testOrderBy() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .order_by("u.username", Some(SortDirection::Ascending.into()))?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.username ASC",
    );
    Ok(())
}

#[test]
fn testOrderByWithExpression() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let asc = qb.expr().asc("u.username");
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .order_by(asc, None)?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.username ASC",
    );
    Ok(())
}

#[test]
fn testLegacyOrderByNull() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let _expect = ExpectDeprecation::new(ORDER_DEPRECATION);
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .order_by("u.username", Some(Order::Null))?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.username ASC",
    );
    Ok(())
}

#[test]
fn testLegacyOrderByString() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let _expect = ExpectDeprecation::new(ORDER_DEPRECATION);
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .order_by("u.username", Some("ASC".into()))?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.username ASC",
    );
    Ok(())
}

#[test]
fn testAddOrderBy() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .order_by("u.username", Some(SortDirection::Ascending.into()))?
        .add_order_by("u.username", Some(SortDirection::Descending.into()))?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.username ASC, u.username DESC",
    );
    Ok(())
}

#[test]
fn testLegacyAddOrderByNull() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let _expect = ExpectDeprecation::new(ORDER_DEPRECATION);
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .order_by("u.username", Some(SortDirection::Ascending.into()))?
        .add_order_by("u.username", Some(Order::Null))?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.username ASC, u.username ASC",
    );
    Ok(())
}

#[test]
fn testLegacyAddOrderByString() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let _expect = ExpectDeprecation::new(ORDER_DEPRECATION);
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .order_by("u.username", Some(SortDirection::Ascending.into()))?
        .add_order_by("u.username", Some("DESC".into()))?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.username ASC, u.username DESC",
    );
    Ok(())
}

#[test]
fn testAddOrderByWithExpression() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let desc = qb.expr().desc("u.username");
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .order_by("u.username", Some(SortDirection::Ascending.into()))?
        .add_order_by(desc, None)?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.username ASC, u.username DESC",
    );
    Ok(())
}

fn value(parameter: Option<&Parameter>) -> &Value {
    parameter.expect("the parameter").get_value()
}

fn assert_same_str(expected: &str, actual: &Value) {
    assert!(Value::from(expected).identical(actual), "{actual:?}");
}

#[test]
fn testAddCriteriaWhere() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?.from(CMS_USER, "u", None)?;

    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("field", "value"));

    qb.add_criteria(&criteria)?;

    assert_eq!("u.field = :field", s(&qb.get_dql_part("where")));
    assert!(qb.get_parameter("field").is_some());
    Ok(())
}

#[test]
fn testAddMultipleSameCriteriaWhere() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["alias1"])?.from(CMS_USER, "alias1", None)?;

    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().and_x([
        Criteria::expr().eq("field", "value1").into(),
        Criteria::expr().eq("field", "value2").into(),
    ])?);

    qb.add_criteria(&criteria)?;

    assert_eq!(
        "alias1.field = :field AND alias1.field = :field_1",
        s(&qb.get_dql_part("where"))
    );
    assert!(qb.get_parameter("field").is_some());
    assert!(qb.get_parameter("field_1").is_some());
    Ok(())
}

#[test]
fn testAddCriteriaWhereWithMultipleParametersWithSameField() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["alias1"])?.from(CMS_USER, "alias1", None)?;

    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("field", "value1"));
    criteria.and_where(Criteria::expr().gt("field", "value2"))?;

    qb.add_criteria(&criteria)?;

    assert_eq!(
        "alias1.field = :field AND alias1.field > :field_1",
        s(&qb.get_dql_part("where"))
    );
    assert_same_str("value1", value(qb.get_parameter("field")));
    assert_same_str("value2", value(qb.get_parameter("field_1")));
    Ok(())
}

#[test]
fn testAddCriteriaWhereWithMultipleParametersWithDifferentFields() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["alias1"])?.from(CMS_USER, "alias1", None)?;

    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("field1", "value1"));
    criteria.and_where(Criteria::expr().gt("field2", "value2"))?;

    qb.add_criteria(&criteria)?;

    assert_eq!(
        "alias1.field1 = :field1 AND alias1.field2 > :field2",
        s(&qb.get_dql_part("where"))
    );
    assert_same_str("value1", value(qb.get_parameter("field1")));
    assert_same_str("value2", value(qb.get_parameter("field2")));
    Ok(())
}

#[test]
fn testAddCriteriaWhereWithMultipleParametersWithSubpathsAndDifferentProperties() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["alias1"])?.from(CMS_USER, "alias1", None)?;

    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("field1", "value1"));
    criteria.and_where(Criteria::expr().gt("field2", "value2"))?;

    qb.add_criteria(&criteria)?;

    assert_eq!(
        "alias1.field1 = :field1 AND alias1.field2 > :field2",
        s(&qb.get_dql_part("where"))
    );
    assert_same_str("value1", value(qb.get_parameter("field1")));
    assert_same_str("value2", value(qb.get_parameter("field2")));
    Ok(())
}

#[test]
fn testAddCriteriaWhereWithMultipleParametersWithSubpathsAndSameProperty() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["alias1"])?.from(CMS_USER, "alias1", None)?;

    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("field1", "value1"));
    criteria.and_where(Criteria::expr().gt("field1", "value2"))?;

    qb.add_criteria(&criteria)?;

    assert_eq!(
        "alias1.field1 = :field1 AND alias1.field1 > :field1_1",
        s(&qb.get_dql_part("where"))
    );
    assert_same_str("value1", value(qb.get_parameter("field1")));
    assert_same_str("value2", value(qb.get_parameter("field1_1")));
    Ok(())
}

#[test]
fn testAddCriteriaOrder() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?.from(CMS_USER, "u", None)?;

    let mut criteria = Criteria::create();
    criteria.order_by([("field", CriteriaSortDirection::Descending)]);

    qb.add_criteria(&criteria)?;

    let order_by = qb.get_dql_part("orderBy");
    assert_eq!(1, count(&order_by));
    assert_eq!(
        "u.field DESC",
        s(order_by.as_array().unwrap().values().next().unwrap())
    );
    Ok(())
}

#[test]
fn testAddCriteriaOrderOnJoinAlias() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .join("u.article", "a", None, None, None)?;

    let mut criteria = Criteria::create();
    criteria.order_by([("a.field", CriteriaSortDirection::Descending)]);

    qb.add_criteria(&criteria)?;

    let order_by = qb.get_dql_part("orderBy");
    assert_eq!(1, count(&order_by));
    assert_eq!(
        "a.field DESC",
        s(order_by.as_array().unwrap().values().next().unwrap())
    );
    Ok(())
}

#[test]
fn testAddCriteriaLimit() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?.from(CMS_USER, "u", None)?;

    let mut criteria = Criteria::create();
    criteria.set_first_result(2);
    criteria.set_max_results(Some(10));

    qb.add_criteria(&criteria)?;

    assert_eq!(2, qb.get_first_result());
    assert_eq!(Some(10), qb.get_max_results());
    Ok(())
}

#[test]
fn testAddCriteriaUndefinedLimit() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .set_first_result(Some(2))
        .set_max_results(Some(10))?;

    let criteria = Criteria::create();

    qb.add_criteria(&criteria)?;

    assert_eq!(2, qb.get_first_result());
    assert_eq!(Some(10), qb.get_max_results());
    Ok(())
}

#[test]
fn testGetQuery() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?.from(CMS_USER, "u", None)?;
    let q: StandInQuery = qb.get_query()?;

    // `$q::class === Query::class`: the type is the `Query` (stand-in).
    assert_eq!(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u",
        q.get_dql()
    );
    Ok(())
}

#[test]
fn testSetParameter() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :id"])?
        .set_parameter("id", 1, None);

    let parameter = Parameter::new(
        "id",
        1,
        Some(ParameterTypeInferer::infer_type(&Value::from(1))),
    );
    let inferred = qb.get_parameter("id").unwrap();

    assert!(parameter.get_value().identical(inferred.get_value()));
    assert_eq!(parameter.get_type(), inferred.get_type());
    assert!(!inferred.type_was_specified());
    Ok(())
}

#[test]
fn testSetParameters() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let or_x = qb
        .expr()
        .or_x(args!["u.username = :username", "u.username = :username2"])?;
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args![or_x])?;

    let parameters = vec![
        Parameter::new("username", "jwage", None),
        Parameter::new("username2", "jonwage", None),
    ];

    qb.set_parameters(parameters.clone());

    assert_parameters_equal(&parameters, &qb.get_query()?.get_parameters());
    Ok(())
}

#[test]
fn testGetParameters() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :id"])?;

    let parameters = vec![Parameter::new("id", 1, None)];

    qb.set_parameters(parameters.clone());

    assert_parameters_equal(&parameters, qb.get_parameters());
    Ok(())
}

#[test]
fn testGetParameter() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :id"])?;

    let parameters = vec![Parameter::new("id", 1, None)];

    qb.set_parameters(parameters.clone());

    assert_parameter_equal(&parameters[0], qb.get_parameter("id").unwrap());
    Ok(())
}

#[test]
fn testMultipleWhere() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = :uid", "u.id = :uid2"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :uid AND u.id = :uid2",
    );
    Ok(())
}

#[test]
fn testMultipleAndWhere() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .and_where(args!["u.id = :uid", "u.id = :uid2"])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :uid AND u.id = :uid2",
    );
    Ok(())
}

#[test]
fn testMultipleOrWhere() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let eq = qb.expr().eq("u.id", ":uid2");
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .or_where(args!["u.id = :uid", eq])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :uid OR u.id = :uid2",
    );
    Ok(())
}

#[test]
fn testComplexWhere() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let or_expr = qb.expr().or_x(vec![])?;
    or_expr.add(qb.expr().eq("u.id", ":uid3"))?;
    or_expr.add(
        qb.expr()
            .r#in("u.id", Value::Array(PhpArray::list(args![1])))?,
    )?;

    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args![or_expr])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :uid3 OR u.id IN(1)",
    );
    Ok(())
}

#[test]
fn testWhereInWithStringLiterals() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let in_ = qb.expr().r#in(
        "u.name",
        Value::Array(PhpArray::list(args!["one", "two", "three"])),
    )?;
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args![in_])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name IN('one', 'two', 'three')",
    );

    let in_ = qb.expr().r#in(
        "u.name",
        Value::Array(PhpArray::list(args!["O'Reilly", "O'Neil", "Smith"])),
    )?;
    qb.r#where(args![in_])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name IN('O''Reilly', 'O''Neil', 'Smith')",
    );
    Ok(())
}

#[test]
fn testWhereInWithObjectLiterals() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let expr = qb.expr();
    let literals = |names: [&str; 3]| -> Result<Value, Thrown> {
        let mut values = Vec::new();
        for name in names {
            values.push(Value::from(expr.literal(name)?));
        }
        Ok(Value::Array(PhpArray::list(values)))
    };
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args![
            expr.r#in("u.name", literals(["one", "two", "three"])?)?
        ])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name IN('one', 'two', 'three')",
    );

    qb.r#where(args![
        expr.r#in("u.name", literals(["O'Reilly", "O'Neil", "Smith"])?)?
    ])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name IN('O''Reilly', 'O''Neil', 'Smith')",
    );
    Ok(())
}

#[test]
fn testNegation() -> TestResult {
    let t = set_up();
    let expr = php2rust_doctrine_orm::query::Expr::new();
    let or_expr = expr.or_x(vec![])?;
    or_expr.add(expr.eq("u.id", ":uid3"))?;
    or_expr.add(expr.not(expr.r#in("u.id", Value::Array(PhpArray::list(args![1])))?))?;

    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args![or_expr])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :uid3 OR NOT(u.id IN(1))",
    );
    Ok(())
}

#[test]
fn testSomeAllAny() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let expr = php2rust_doctrine_orm::query::Expr::new();

    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args![expr.gt(
            "u.id",
            expr.all(r"select a.id from Doctrine\Tests\Models\CMS\CmsArticle a")
        )])?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id > ALL(select a.id from Doctrine\Tests\Models\CMS\CmsArticle a)",
    );
    Ok(())
}

#[test]
fn testMultipleIsolatedQueryConstruction() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let expr = php2rust_doctrine_orm::query::Expr::new();

    qb.select(args!["u"])?.from(CMS_USER, "u", None)?;
    qb.r#where(args![expr.eq("u.name", ":name")])?;
    qb.set_parameter("name", "romanb", None);

    let q1 = qb.get_query()?;

    assert_eq!(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name = :name",
        q1.get_dql()
    );
    assert_eq!(1, q1.get_parameters().len());

    // add another condition and construct a second query
    qb.and_where(args![expr.eq("u.id", ":id")])?;
    qb.set_parameter("id", 42, None);

    let q2 = qb.get_query()?;

    assert_eq!(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name = :name AND u.id = :id",
        q2.get_dql()
    );
    // two different, independent queries
    assert!(!std::ptr::eq(&raw const q1, &raw const q2));
    assert_eq!(2, q2.get_parameters().len());
    assert_eq!(1, q1.get_parameters().len()); // $q1 unaffected
    Ok(())
}

#[test]
fn testGetEntityManager() {
    let t = set_up();
    let qb = t.create_query_builder();
    let em: Arc<dyn QueryContext> = Arc::clone(&t.entity_manager) as Arc<dyn QueryContext>;
    assert!(Arc::ptr_eq(&em, qb.get_entity_manager()));
}

#[test]
fn testSelectWithFuncExpression() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let expr = qb.expr();
    qb.select(args![expr.count("e.id")])?;

    assert_valid_query_builder(&qb, "SELECT COUNT(e.id)");
    Ok(())
}

#[test]
fn testResetDQLPart() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.username = ?1"])?
        .order_by("u.username", None)?;

    assert_eq!("u.username = ?1", s(&qb.get_dql_part("where")));
    assert_eq!(1, count(&qb.get_dql_part("orderBy")));

    qb.reset_dql_part("where").reset_dql_part("orderBy");

    assert!(qb.get_dql_part("where").is_null());
    assert_eq!(0, count(&qb.get_dql_part("orderBy")));
    Ok(())
}

#[test]
fn testResetDQLParts() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.username = ?1"])?
        .order_by("u.username", None)?;

    qb.reset_dql_parts(Some(&["where", "orderBy"]));

    assert_eq!(1, count(&qb.get_dql_part("select")));
    assert!(qb.get_dql_part("where").is_null());
    assert_eq!(0, count(&qb.get_dql_part("orderBy")));
    Ok(())
}

#[test]
fn testResetAllDQLParts() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.username = ?1"])?
        .order_by("u.username", None)?;

    qb.reset_dql_parts(None);

    assert_eq!(0, count(&qb.get_dql_part("select")));
    assert!(qb.get_dql_part("where").is_null());
    assert_eq!(0, count(&qb.get_dql_part("orderBy")));
    Ok(())
}

fn andx_count(value: &Value) -> usize {
    value
        .as_object()
        .and_then(|o| o.downcast_ref::<Andx>())
        .expect("an Andx")
        .count()
}

#[test]
fn testDeepClone() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .and_where(args!["u.username = ?1"])?
        .and_where(args!["u.status = ?2"])?;

    let expr = qb.get_dql_part("where");
    assert_eq!(
        2,
        andx_count(&expr),
        "Modifying the second query should affect the first one."
    );

    let mut qb2 = qb.clone();
    qb2.and_where(args!["u.name = ?3"])?;

    assert_eq!(
        2,
        andx_count(&expr),
        "Modifying the second query should affect the first one."
    );
    Ok(())
}

#[test]
fn testAddCriteriaWhereWithJoinAlias() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["alias1"])?.from(CMS_USER, "alias1", None)?;
    qb.join("alias1.articles", "alias2", None, None, None)?;

    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("field", "value1"));
    criteria.and_where(Criteria::expr().gt("alias2.field", "value2"))?;

    qb.add_criteria(&criteria)?;

    assert_eq!(
        "alias1.field = :field AND alias2.field > :alias2_field",
        s(&qb.get_dql_part("where"))
    );
    assert_same_str("value1", value(qb.get_parameter("field")));
    assert_same_str("value2", value(qb.get_parameter("alias2_field")));
    Ok(())
}

#[test]
fn testAddCriteriaWhereWithDefaultAndJoinAlias() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["alias1"])?.from(CMS_USER, "alias1", None)?;
    qb.join("alias1.articles", "alias2", None, None, None)?;

    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("alias1.field", "value1"));
    criteria.and_where(Criteria::expr().gt("alias2.field", "value2"))?;

    qb.add_criteria(&criteria)?;

    assert_eq!(
        "alias1.field = :alias1_field AND alias2.field > :alias2_field",
        s(&qb.get_dql_part("where"))
    );
    assert_same_str("value1", value(qb.get_parameter("alias1_field")));
    assert_same_str("value2", value(qb.get_parameter("alias2_field")));
    Ok(())
}

#[test]
fn testAddCriteriaWhereOnJoinAliasWithDuplicateFields() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["alias1"])?.from(CMS_USER, "alias1", None)?;
    qb.join("alias1.articles", "alias2", None, None, None)?;

    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("alias1.field", "value1"));
    criteria.and_where(Criteria::expr().gt("alias2.field", "value2"))?;
    criteria.and_where(Criteria::expr().lt("alias2.field", "value3"))?;

    qb.add_criteria(&criteria)?;

    assert_eq!(
        "(alias1.field = :alias1_field AND alias2.field > :alias2_field) AND alias2.field < :alias2_field_2",
        s(&qb.get_dql_part("where"))
    );
    assert_same_str("value1", value(qb.get_parameter("alias1_field")));
    assert_same_str("value2", value(qb.get_parameter("alias2_field")));
    assert_same_str("value3", value(qb.get_parameter("alias2_field_2")));
    Ok(())
}

#[test]
fn testParametersAreCloned() {
    let t = set_up();
    let mut original_qb = QueryBuilder::new(Arc::clone(&t.entity_manager) as Arc<dyn QueryContext>);

    original_qb.set_parameter("parameter1", "value1", None);

    let mut copy = original_qb.clone();
    copy.set_parameter("parameter2", "value2", None);

    assert_eq!(1, original_qb.get_parameters().len());
    assert_same_str("value1", value(copy.get_parameter("parameter1")));
    assert_same_str("value2", value(copy.get_parameter("parameter2")));
}

#[test]
fn testGetRootAlias() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?.from(CMS_USER, "u", None)?;

    assert_eq!("u", qb.get_root_alias()?);
    Ok(())
}

#[test]
fn testGetRootAliases() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?.from(CMS_USER, "u", None)?;

    assert_eq!(vec!["u".to_owned()], qb.get_root_aliases()?);
    Ok(())
}

#[test]
fn testGetRootEntities() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?.from(CMS_USER, "u", None)?;

    assert_eq!(vec![CMS_USER.to_owned()], qb.get_root_entities()?);
    Ok(())
}

#[test]
fn testGetSeveralRootAliases() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .from(CMS_USER, "u2", None)?;

    assert_eq!(
        vec!["u".to_owned(), "u2".to_owned()],
        qb.get_root_aliases()?
    );
    assert_eq!("u", qb.get_root_alias()?);
    Ok(())
}

#[test]
fn testBCAddJoinWithoutRootAlias() -> TestResult {
    let _expect = ExpectDeprecation::new("https://github.com/doctrine/orm/pull/12051");
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?.from(CMS_USER, "u", None)?.add(
        "join",
        Value::Array(PhpArray::list(args!["INNER JOIN u.groups g"])),
        true,
    )?;

    assert_eq!(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.groups g",
        qb.get_dql()?
    );
    Ok(())
}

#[test]
fn testEmptyStringLiteral() -> TestResult {
    let t = set_up();
    let expr = php2rust_doctrine_orm::query::Expr::new();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args![expr.eq("u.username", expr.literal("")?)])?;

    assert_eq!(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.username = ''",
        qb.get_dql()?
    );
    Ok(())
}

#[test]
fn testEmptyNumericLiteral() -> TestResult {
    let t = set_up();
    let expr = php2rust_doctrine_orm::query::Expr::new();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args![expr.eq("u.username", expr.literal(0)?)])?;

    assert_eq!(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.username = 0",
        qb.get_dql()?
    );
    Ok(())
}

#[test]
fn testAddFromString() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.add("select", "u", false)?
        .add("from", format!("{CMS_USER} u").as_str(), false)?;

    assert_eq!(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u",
        qb.get_dql()?
    );
    Ok(())
}

#[test]
fn testAddDistinct() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .distinct(true)
        .from(CMS_USER, "u", None)?;

    assert_eq!(
        r"SELECT DISTINCT u FROM Doctrine\Tests\Models\CMS\CmsUser u",
        qb.get_dql()?
    );
    Ok(())
}

#[test]
fn testDistinctUpdatesState() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?.from(CMS_USER, "u", None)?;

    qb.get_dql()?;
    qb.distinct(true);

    assert_eq!(
        r"SELECT DISTINCT u FROM Doctrine\Tests\Models\CMS\CmsUser u",
        qb.get_dql()?
    );
    Ok(())
}

#[test]
fn testWhereAppend() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    let result = qb
        .add("where", "u.foo = ?1", false)?
        .add("where", "u.bar = ?2", true)
        .map(|_| ());
    expect_thrown(
        result,
        "InvalidArgumentException",
        "Using $append = true does not have an effect with 'where' or 'having' parts. See QueryBuilder#andWhere() for an example for correct usage.",
    );
    Ok(())
}

#[test]
fn testSecondLevelCacheQueryBuilderOptions() -> TestResult {
    let t = set_up();
    let mut default_query_builder = t.create_query_builder();
    default_query_builder
        .select(args!["s"])?
        .from(STATE, "s", None)?;

    assert!(!default_query_builder.is_cacheable());
    assert_eq!(0, default_query_builder.get_lifetime());
    assert_eq!(None, default_query_builder.get_cache_region());
    assert_eq!(None, default_query_builder.get_cache_mode());

    let default_query = default_query_builder.get_query()?;

    assert!(!default_query.is_cacheable());
    assert_eq!(0, default_query.get_lifetime());
    assert_eq!(None, default_query.get_cache_region());
    assert_eq!(None, default_query.get_cache_mode());

    let mut builder = t.create_query_builder();
    builder
        .select(args!["s"])?
        .set_lifetime(123)
        .set_cacheable(true)
        .set_cache_region("foo_reg")
        .set_cache_mode(CACHE_MODE_REFRESH)
        .from(STATE, "s", None)?;

    assert!(builder.is_cacheable());
    assert_eq!(123, builder.get_lifetime());
    assert_eq!(Some("foo_reg"), builder.get_cache_region());
    assert_eq!(Some(CACHE_MODE_REFRESH), builder.get_cache_mode());

    let query = builder.get_query()?;

    assert!(query.is_cacheable());
    assert_eq!(123, query.get_lifetime());
    assert_eq!(Some("foo_reg".to_owned()), query.get_cache_region());
    assert_eq!(Some(CACHE_MODE_REFRESH), query.get_cache_mode());
    Ok(())
}

#[test]
fn testRebuildsFromParts() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .join("u.article", "a", None, None, None)?;

    let dql_parts = qb.get_dql_parts();
    let dql = qb.get_dql()?;

    let mut qb2 = t.create_query_builder();
    // `array_filter($dqlParts)`: the truthy parts.
    for (name, part) in dql_parts.iter().filter(|(_, part)| part.is_truthy()) {
        let name = String::from_utf8_lossy(&name.bytes()).into_owned();
        qb2.add(&name, part.clone(), false)?;
    }

    let dql2 = qb2.get_dql()?;

    assert_eq!(dql, dql2);
    Ok(())
}

#[test]
fn testGetAllAliasesWithNoJoins() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?.from(CMS_USER, "u", None)?;

    let aliases = qb.get_all_aliases()?;

    assert_eq!(vec!["u".to_owned()], aliases);
    Ok(())
}

#[test]
fn testGetAllAliasesWithJoins() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .join("u.groups", "g", None, None, None)?;

    let aliases = qb.get_all_aliases()?;

    assert_eq!(vec!["u".to_owned(), "g".to_owned()], aliases);
    Ok(())
}

fn assert_same_int(expected: i64, actual: &Value) {
    assert!(Value::from(expected).identical(actual), "{actual:?}");
}

#[test]
fn testGetParameterTypeJuggling() -> TestResult {
    let t = set_up();
    let mut builder = t.create_query_builder();
    builder
        .select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id = ?0"])?;

    builder.set_parameter(0, 0, None);

    assert_eq!(1, builder.get_parameters().len());
    assert_same_int(0, value(builder.get_parameter(0)));
    assert_same_int(0, value(builder.get_parameter("0")));
    Ok(())
}

#[test]
fn testSetParameterWithNameZeroIsNotOverridden() -> TestResult {
    let t = set_up();
    let mut builder = t.create_query_builder();
    builder
        .select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id != ?0"])?
        .and_where(args!["u.username = :name"])?;

    builder.set_parameter(0, 0, None);
    builder.set_parameter("name", "Doctrine", None);

    assert_eq!(2, builder.get_parameters().len());
    assert_same_int(0, value(builder.get_parameter("0")));
    assert_same_str("Doctrine", value(builder.get_parameter("name")));
    Ok(())
}

#[test]
fn testSetParameterWithNameZeroDoesNotOverrideAnotherParameter() -> TestResult {
    let t = set_up();
    let mut builder = t.create_query_builder();
    builder
        .select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id != ?0"])?
        .and_where(args!["u.username = :name"])?;

    builder.set_parameter("name", "Doctrine", None);
    builder.set_parameter(0, 0, None);

    assert_eq!(2, builder.get_parameters().len());
    assert_same_int(0, value(builder.get_parameter(0)));
    assert_same_str("Doctrine", value(builder.get_parameter("name")));
    Ok(())
}

#[test]
fn testSetParameterWithTypeJugglingWorks() -> TestResult {
    let t = set_up();
    let mut builder = t.create_query_builder();
    builder
        .select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args!["u.id != ?0"])?
        .and_where(args!["u.username = :name"])?;

    builder.set_parameter("0", 1, None);
    builder.set_parameter("name", "Doctrine", None);
    builder.set_parameter(0, 2, None);
    builder.set_parameter("0", 3, None);

    assert_eq!(2, builder.get_parameters().len());
    assert_same_int(3, value(builder.get_parameter(0)));
    assert_same_int(3, value(builder.get_parameter("0")));
    assert_same_str("Doctrine", value(builder.get_parameter("name")));
    Ok(())
}

#[test]
fn testJoin() -> TestResult {
    let t = set_up();
    let mut builder = t.create_query_builder();
    builder
        .select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .left_join(CMS_ARTICLE, "a0", None, None, None)?
        .inner_join(CMS_ARTICLE, "a1", None, None, None)?;

    assert_eq!(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN Doctrine\Tests\Models\CMS\CmsArticle a0 INNER JOIN Doctrine\Tests\Models\CMS\CmsArticle a1",
        builder.get_dql()?
    );
    Ok(())
}

#[test]
fn testUpdateWithoutAlias() {
    let t = set_up();
    let mut qb = t.create_query_builder();

    expect_thrown(
        qb.update(Some(&format!("{CMS_USER} u")), None).map(|_| ()),
        "InvalidArgumentException",
        r"Doctrine\ORM\QueryBuilder::update(): The alias for entity Doctrine\Tests\Models\CMS\CmsUser u must not be omitted.",
    );
}

#[test]
fn testDeleteWithoutAlias() {
    let t = set_up();
    let mut qb = t.create_query_builder();

    expect_thrown(
        qb.delete(Some(&format!("{CMS_USER} u")), None).map(|_| ()),
        "InvalidArgumentException",
        r"Doctrine\ORM\QueryBuilder::delete(): The alias for entity Doctrine\Tests\Models\CMS\CmsUser u must not be omitted.",
    );
}

fn type_name(name: &str) -> WrapperParameterType {
    WrapperParameterType::from(name)
}

#[test]
fn testCreateNamedParameter() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();

    let name = qb.create_named_parameter("john doe", Some(type_name("string")), None);
    let rank = qb.create_named_parameter(100, Some(type_name("integer")), None);
    let (name_eq, rank_eq) = (qb.expr().eq("u.name", name), qb.expr().eq("u.rank", rank));
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args![name_eq])?
        .or_where(args![rank_eq])?;

    assert_eq!(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name = :dcValue1 OR u.rank = :dcValue2",
        qb.get_dql()?
    );
    let dc_value1 = qb.get_parameter("dcValue1").unwrap();
    assert!(loose_equals(
        &Value::from("john doe"),
        dc_value1.get_value()
    ));
    assert_eq!(&type_name("string"), dc_value1.get_type());
    let dc_value2 = qb.get_parameter("dcValue2").unwrap();
    assert!(loose_equals(&Value::from(100), dc_value2.get_value()));
    assert_eq!(&type_name("integer"), dc_value2.get_type());
    Ok(())
}

#[test]
fn testCreateNamedParameterCustomPlaceholder() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();

    let name = qb.create_named_parameter("john doe", Some(type_name("string")), Some(":test"));
    let rank = qb.create_named_parameter(100, Some(type_name("integer")), None);
    let (name_eq, rank_eq) = (qb.expr().eq("u.name", name), qb.expr().eq("u.rank", rank));
    qb.select(args!["u"])?
        .from(CMS_USER, "u", None)?
        .r#where(args![name_eq])?
        .and_where(args![rank_eq])?;

    assert_eq!(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name = :test AND u.rank = :dcValue1",
        qb.get_dql()?
    );
    let test = qb.get_parameter("test").unwrap();
    assert!(loose_equals(&Value::from("john doe"), test.get_value()));
    assert_eq!(&type_name("string"), test.get_type());
    let dc_value1 = qb.get_parameter("dcValue1").unwrap();
    assert!(loose_equals(&Value::from(100), dc_value1.get_value()));
    assert_eq!(&type_name("integer"), dc_value1.get_type());
    Ok(())
}

#[test]
fn testType() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();

    assert_eq!(QueryType::Select, qb.get_type());

    qb.delete(None, None)?;
    assert_eq!(QueryType::Delete, qb.get_type());

    qb.update(None, None)?;
    assert_eq!(QueryType::Update, qb.get_type());

    qb.select(vec![])?;
    assert_eq!(QueryType::Select, qb.get_type());
    Ok(())
}

fn cms_user() -> Value {
    Value::Object(EntityObject::object(CMS_USER).expect("CmsUser is declared"))
}

fn hint_value(hint: Hint) -> Value {
    match hint {
        Hint::Value(value) => value,
        other => panic!("not a value hint: {other:?}"),
    }
}

fn provide_hint() -> Vec<Value> {
    vec![
        Value::from("bar"),
        cms_user(),
        Value::Array(PhpArray::list(args!["a", "b", "c"])),
        Value::from(1),
        Value::Bool(true),
    ]
}

#[test]
fn testSingleHint() -> TestResult {
    let t = set_up();
    for expected in provide_hint() {
        let mut qb = t.create_query_builder();
        qb.delete(Some(CMS_USER), Some("u"))?
            .select(args!["u.id", "u.username"])?
            .set_hint("foo", Hint::Value(expected.clone()));

        assert_valid_query_builder(
            &qb,
            r"SELECT u.id, u.username FROM Doctrine\Tests\Models\CMS\CmsUser u",
        );

        let query = qb.get_query()?;
        assert!(query.has_hint("foo"));
        assert!(loose_equals(&expected, &hint_value(query.get_hint("foo"))));
    }
    Ok(())
}

fn set_five_hints(qb: &mut QueryBuilder, object: &Value) -> TestResult {
    qb.delete(Some(CMS_USER), Some("u"))?
        .select(args!["u.id", "u.username"])?
        .set_hint("string", Hint::Value(Value::from("bar")))
        .set_hint("object", Hint::Value(object.clone()))
        .set_hint(
            "array",
            Hint::Value(Value::Array(PhpArray::list(args!["a", "b", "c"]))),
        )
        .set_hint("int", Hint::Value(Value::from(5)))
        .set_hint("bool", Hint::Value(Value::Bool(true)));
    Ok(())
}

#[test]
fn testMultipleHints() -> TestResult {
    let t = set_up();
    let object = cms_user();
    let mut qb = t.create_query_builder();
    set_five_hints(&mut qb, &object)?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u.id, u.username FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );

    let query = qb.get_query()?;
    assert!(query.has_hint("string"));
    assert!(query.has_hint("object"));
    assert!(query.has_hint("array"));
    assert!(query.has_hint("int"));
    assert!(query.has_hint("bool"));

    assert!(loose_equals(
        &Value::from("bar"),
        &hint_value(query.get_hint("string"))
    ));
    assert_eq!(
        CMS_USER,
        hint_value(query.get_hint("object"))
            .as_object()
            .unwrap()
            .class()
    );
    assert!(loose_equals(
        &Value::Array(PhpArray::list(args!["a", "b", "c"])),
        &hint_value(query.get_hint("array"))
    ));
    assert!(loose_equals(
        &Value::from(5),
        &hint_value(query.get_hint("int"))
    ));
    assert!(query.get_hint("bool").is_true());
    Ok(())
}

#[test]
fn testHasHint() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.delete(Some(CMS_USER), Some("u"))?
        .select(args!["u.id", "u.username"])?
        .set_hint("foo", Hint::Value(Value::from("bar")));

    assert_valid_query_builder(
        &qb,
        r"SELECT u.id, u.username FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );

    assert!(qb.has_hint("foo"));
    Ok(())
}

#[test]
fn testGetHint() -> TestResult {
    let t = set_up();
    let mut qb = t.create_query_builder();
    qb.delete(Some(CMS_USER), Some("u"))?
        .select(args!["u.id", "u.username"])?
        .set_hint("foo", Hint::Value(Value::from("bar")));

    assert_valid_query_builder(
        &qb,
        r"SELECT u.id, u.username FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );

    assert!(loose_equals(
        &Value::from("bar"),
        &hint_value(qb.get_hint("foo"))
    ));
    Ok(())
}

#[test]
fn testGetHints() -> TestResult {
    let t = set_up();
    let object = cms_user();
    let mut qb = t.create_query_builder();
    set_five_hints(&mut qb, &object)?;

    assert_valid_query_builder(
        &qb,
        r"SELECT u.id, u.username FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );

    assert_eq!(5, qb.get_hints().len());
    Ok(())
}
