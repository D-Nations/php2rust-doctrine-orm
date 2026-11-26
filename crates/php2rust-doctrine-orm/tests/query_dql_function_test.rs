//! QueryDqlFunctionTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The DQL functions and operators on the `CompanyManager`s of the
//! `company` model set, on `SQLite`. A data provider's rows run in one
//! test, labelled. `testInArithmeticExpression2` adds its custom function
//! to the configuration before the entity manager is created (the port's
//! entity manager takes the configuration by value). `NoOp` is the
//! function of `CustomFunctionsTest`. `testFunctionSqrt` is pending: the
//! DBAL port's `SQLite` library is built without the math functions
//! (`SQRT()`), which PHP's `pdo_sqlite` has.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/QueryDqlFunctionTest.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/CustomFunctionsTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    reason = "PHP's (float) casts and its float assertEquals() of rounded values"
)]

#[path = "common/company.rs"]
mod company;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php_value::date_create::date_create;
use php_value::dateparse::strtotime;
use php_value::{Thrown, Value};
use php2rust_doctrine_orm::abstract_query::{HYDRATE_ARRAY, HYDRATE_OBJECT};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::ast::Node;
use php2rust_doctrine_orm::query::ast::functions::FunctionNode;
use php2rust_doctrine_orm::query::context::register_function_class;
use php2rust_doctrine_orm::query::sql_walker::dispatch;
use php2rust_doctrine_orm::query::{Parser, SqlWalker, TokenType};

use company::{COMPANY_EMPLOYEE, COMPANY_MANAGER, COMPANY_MODELS};
use orm::{assert_loose_eq, at, list, s};

/// `NoOp extends FunctionNode` (`CustomFunctionsTest`).
#[derive(Debug)]
struct NoOp {
    name: String,
    field: Option<Node>,
}

impl FunctionNode for NoOp {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\NoOp"
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        parser.match_token(TokenType::T_IDENTIFIER)?;
        parser.match_token(TokenType::T_OPEN_PARENTHESIS)?;
        self.field = Some(parser.arithmetic_primary()?);
        parser.match_token(TokenType::T_CLOSE_PARENTHESIS)
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        dispatch(sql_walker, self.field.as_ref().unwrap())
    }
}

/// `generateFixture()`.
async fn generate_fixture(em: &Arc<EntityManager>) {
    let manager1 = company::company_manager("Roman B.", 100_000, "IT", "Foo");

    let manager2 = company::company_manager("Benjamin E.", 200_000, "HR", "Foo");

    let manager3 = company::company_manager("Guilherme B.", 400_000, "Complaint Department", "Foo");

    let manager4 = company::company_manager("Jonathan W.", 800_000, "Administration", "Foo");

    em.persist(&manager1).unwrap();
    em.persist(&manager2).unwrap();
    em.persist(&manager3).unwrap();
    em.persist(&manager4).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
}

async fn set_up() -> Arc<EntityManager> {
    company::register_listeners();
    let em = orm::set_up(&COMPANY_MODELS).await;
    generate_fixture(&em).await;
    em
}

async fn single(em: &Arc<EntityManager>, dql: &str) -> Value {
    em.create_query(dql).get_single_result(None).await.unwrap()
}

async fn result(em: &Arc<EntityManager>, dql: &str) -> Vec<Value> {
    list(
        &em.create_query(dql)
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    )
}

async fn array_result(em: &Arc<EntityManager>, dql: &str) -> Vec<Value> {
    list(&em.create_query(dql).get_array_result().await.unwrap())
}

/// `(float) $value`.
fn float(value: &Value) -> f64 {
    match value {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        Value::String(text) => text.to_string_lossy().trim().parse().unwrap(),
        other => panic!("not a number: {other:?}"),
    }
}

/// `assertEquals($expected, $rows[$i][$key])` for each row.
fn assert_column(rows: &[Value], key: &str, expected: &[Value]) {
    assert_eq!(rows.len(), expected.len());
    for (row, expected) in rows.iter().zip(expected) {
        assert_loose_eq(expected, &at(row, key));
    }
}

fn ints(values: &[i64]) -> Vec<Value> {
    values.iter().map(|v| Value::Int(*v)).collect()
}

fn strings(values: &[&str]) -> Vec<Value> {
    values.iter().map(|v| s(v)).collect()
}

#[tokio::test]
async fn testAggregateSum() {
    let em = set_up().await;
    let salary_sum = single(
        &em,
        "SELECT SUM(m.salary) AS salary FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m",
    )
    .await;

    assert_loose_eq(&Value::Int(1_500_000), &at(&salary_sum, "salary"));
}

#[tokio::test]
async fn testAggregateAvg() {
    let em = set_up().await;
    let salary_avg = single(
        &em,
        "SELECT AVG(m.salary) AS salary FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m",
    )
    .await;

    assert_eq!(float(&at(&salary_avg, "salary")).round(), 375_000.0);
}

#[tokio::test]
async fn testAggregateMin() {
    let em = set_up().await;
    let salary = single(
        &em,
        "SELECT MIN(m.salary) AS salary FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m",
    )
    .await;

    assert_loose_eq(&Value::Int(100_000), &at(&salary, "salary"));
}

#[tokio::test]
async fn testAggregateMax() {
    let em = set_up().await;
    let salary = single(
        &em,
        "SELECT MAX(m.salary) AS salary FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m",
    )
    .await;

    assert_loose_eq(&Value::Int(800_000), &at(&salary, "salary"));
}

#[tokio::test]
async fn testAggregateCount() {
    let em = set_up().await;
    let manager_count = single(
        &em,
        "SELECT COUNT(m.id) AS managers FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m",
    )
    .await;

    assert_loose_eq(&Value::Int(4), &at(&manager_count, "managers"));
}

#[tokio::test]
async fn testFunctionAbs() {
    let em = set_up().await;
    let result = result(
        &em,
        "SELECT m, ABS(m.salary * -1) AS abs FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.salary ASC",
    )
    .await;

    assert_column(&result, "abs", &ints(&[100_000, 200_000, 400_000, 800_000]));
}

#[tokio::test]
async fn testFunctionConcat() {
    let em = set_up().await;
    let arg = array_result(
        &em,
        "SELECT m, CONCAT(m.name, m.department) AS namedep FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.salary ASC",
    )
    .await;

    assert_column(
        &arg,
        "namedep",
        &strings(&[
            "Roman B.IT",
            "Benjamin E.HR",
            "Guilherme B.Complaint Department",
            "Jonathan W.Administration",
        ]),
    );
}

#[tokio::test]
async fn testFunctionLength() {
    let em = set_up().await;
    let result = array_result(
        &em,
        "SELECT m, LENGTH(CONCAT(m.name, m.department)) AS namedeplength FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.salary ASC",
    )
    .await;

    assert_column(&result, "namedeplength", &ints(&[10, 13, 32, 25]));
}

#[tokio::test]
async fn testFunctionLocate() {
    let em = set_up().await;
    let dql = "SELECT m, LOCATE('e', LOWER(m.name)) AS loc, LOCATE('e', LOWER(m.name), 7) AS loc2 FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.salary ASC";

    let result = array_result(&em, dql).await;

    assert_column(&result, "loc", &ints(&[0, 2, 6, 0]));
    assert_column(&result, "loc2", &ints(&[0, 10, 9, 0]));
}

#[tokio::test]
async fn testFunctionLower() {
    let em = set_up().await;
    let result = array_result(
        &em,
        "SELECT m, LOWER(m.name) AS lowername FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.salary ASC",
    )
    .await;

    assert_column(
        &result,
        "lowername",
        &strings(&["roman b.", "benjamin e.", "guilherme b.", "jonathan w."]),
    );
}

#[tokio::test]
async fn testFunctionMod() {
    let em = set_up().await;
    let result = array_result(
        &em,
        "SELECT m, MOD(m.salary, 3500) AS amod FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.salary ASC",
    )
    .await;

    assert_column(&result, "amod", &ints(&[2000, 500, 1000, 2000]));
}

#[tokio::test]
async fn testFunctionUpper() {
    let em = set_up().await;
    let result = array_result(
        &em,
        "SELECT m, UPPER(m.name) AS uppername FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.salary ASC",
    )
    .await;

    assert_column(
        &result,
        "uppername",
        &strings(&["ROMAN B.", "BENJAMIN E.", "GUILHERME B.", "JONATHAN W."]),
    );
}

#[tokio::test]
async fn testFunctionSubstring() {
    let em = set_up().await;
    let dql = "SELECT m, SUBSTRING(m.name, 1, 3) AS str1, SUBSTRING(m.name, 5) AS str2 FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.name";

    let result = array_result(&em, dql).await;

    assert_column(&result, "str1", &strings(&["Ben", "Gui", "Jon", "Rom"]));
    assert_column(
        &result,
        "str2",
        &strings(&["amin E.", "herme B.", "than W.", "n B."]),
    );
}

#[tokio::test]
async fn testFunctionTrim() {
    let em = set_up().await;
    let dql = "SELECT m, TRIM(TRAILING '.' FROM m.name) AS str1,  TRIM(LEADING '.' FROM m.name) AS str2, TRIM(CONCAT(' ', CONCAT(m.name, ' '))) AS str3 FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.salary ASC";

    let result = array_result(&em, dql).await;

    assert_column(
        &result,
        "str1",
        &strings(&["Roman B", "Benjamin E", "Guilherme B", "Jonathan W"]),
    );
    let names = strings(&["Roman B.", "Benjamin E.", "Guilherme B.", "Jonathan W."]);
    assert_column(&result, "str2", &names);
    assert_column(&result, "str3", &names);
}

#[tokio::test]
async fn testOperatorAdd() {
    let em = set_up().await;
    let result = result(
        &em,
        "SELECT m, m.salary+2500 AS add FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.salary ASC",
    )
    .await;

    assert_column(&result, "add", &ints(&[102_500, 202_500, 402_500, 802_500]));
}

#[tokio::test]
async fn testOperatorSub() {
    let em = set_up().await;
    let result = result(
        &em,
        "SELECT m, m.salary-2500 AS sub FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.salary ASC",
    )
    .await;

    assert_column(&result, "sub", &ints(&[97_500, 197_500, 397_500, 797_500]));
}

#[tokio::test]
async fn testOperatorMultiply() {
    let em = set_up().await;
    let result = result(
        &em,
        "SELECT m, m.salary*2 AS op FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.salary ASC",
    )
    .await;

    assert_column(
        &result,
        "op",
        &ints(&[200_000, 400_000, 800_000, 1_600_000]),
    );
}

#[tokio::test]
async fn testOperatorDiv() {
    let em = set_up().await;
    let result = result(
        &em,
        "SELECT m, (m.salary/0.5) AS op FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.salary ASC",
    )
    .await;

    assert_column(
        &result,
        "op",
        &ints(&[200_000, 400_000, 800_000, 1_600_000]),
    );
}

#[tokio::test]
async fn testConcatFunction() {
    let em = set_up().await;
    let arg = array_result(
        &em,
        "SELECT CONCAT(m.name, m.department) AS namedep FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m order by namedep desc",
    )
    .await;

    assert_column(
        &arg,
        "namedep",
        &strings(&[
            "Roman B.IT",
            "Jonathan W.Administration",
            "Guilherme B.Complaint Department",
            "Benjamin E.HR",
        ]),
    );
}

#[tokio::test]
async fn testDateDiff() {
    let em = set_up().await;
    let arg = array_result(
        &em,
        "SELECT DATE_DIFF(CURRENT_TIMESTAMP(), DATE_ADD(CURRENT_TIMESTAMP(), 10, 'day')) AS diff FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m",
    )
    .await;

    assert!(
        (float(&at(&arg[0], "diff")) - -10.0).abs() <= 1.0,
        "Should be roughly -10 (or -9)"
    );

    let arg = array_result(
        &em,
        "SELECT DATE_DIFF(DATE_ADD(CURRENT_TIMESTAMP(), 10, 'day'), CURRENT_TIMESTAMP()) AS diff FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m",
    )
    .await;

    assert!(
        (float(&at(&arg[0], "diff")) - 10.0).abs() <= 1.0,
        "Should be roughly 10 (or 9)"
    );
}

/// `dateAddSubProvider()`.
const DATE_ADD_SUB: [(&str, i64, i64); 7] = [
    ("year", 1, 86_400),
    ("month", 1, 86_400),
    ("week", 1, 86_400),
    ("day", 2, 86_400),
    ("hour", 1, 3600),
    ("minute", 1, 60),
    ("second", 10, 10),
];

/// `new DateTimeImmutable($text)`'s timestamp.
fn timestamp(text: &Value) -> i64 {
    let text = text.to_php_string().unwrap();
    date_create(text.as_bytes(), None)
        .unwrap()
        .unwrap()
        .timestamp()
}

/// `$now->modify($modifier)`'s timestamp.
fn modify(now: i64, modifier: &str) -> i64 {
    strtotime(modifier.as_bytes(), now).unwrap().unwrap()
}

/// `testDateAdd()` and `testDateSub()`: on `SQLite`, the expected value is
/// `$now->modify()`'s.
async fn date_add_sub(function: &str, alias: &str, sign: &str) {
    let em = set_up().await;
    for (unit, amount, delta) in DATE_ADD_SUB {
        let dql = format!(
            "SELECT CURRENT_TIMESTAMP() as now, {function}(CURRENT_TIMESTAMP(), {amount}, '{unit}') AS {alias} FROM {COMPANY_MANAGER} m"
        );

        let query = em.create_query(&dql);
        query.set_max_results(Some(1));
        let result = query.get_single_result(Some(HYDRATE_ARRAY)).await.unwrap();

        let Value::Array(row) = &result else {
            panic!("not an array: {result:?}");
        };
        assert!(row.contains_key(&"now".into()), "{unit}");
        assert!(row.contains_key(&alias.into()), "{unit}");

        let now = timestamp(&at(&result, "now"));
        let expected = modify(now, &format!("{sign}{amount} {unit}"));

        let actual = timestamp(&at(&result, alias));
        assert!(
            (expected - actual).abs() <= delta,
            "{unit}: {expected} != {actual}"
        );
    }
}

#[tokio::test]
async fn testDateAdd() {
    date_add_sub("DATE_ADD", "add", "+").await;
}

#[tokio::test]
async fn testDateSub() {
    date_add_sub("DATE_SUB", "sub", "-").await;
}

/// `testBitOrComparison()` and `testBitAndComparison()`.
async fn bit_comparison(function: &str, alias: &str, op: fn(i64, i64) -> i64) {
    let em = set_up().await;
    let dql = format!(
        "SELECT m, {function}(4, 2) AS {alias},{function}( (m.salary/100000) , 2 ) AS salary_{alias} FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m ORDER BY m.id "
    );
    let result = array_result(&em, &dql).await;

    for row in &result {
        assert_loose_eq(&Value::Int(op(4, 2)), &at(row, alias));
    }

    for row in &result {
        // `$result[$i][0]['salary'] / 100000 | 2`
        let salary = float(&at(&at(row, 0_i64), "salary"));
        #[allow(
            clippy::cast_possible_truncation,
            reason = "PHP's int conversion of the quotient"
        )]
        let quotient = (salary / 100_000.0) as i64;
        assert_loose_eq(
            &Value::Int(op(quotient, 2)),
            &at(row, format!("salary_{alias}").as_str()),
        );
    }
}

#[tokio::test]
async fn testBitOrComparison() {
    bit_comparison("BIT_OR", "bit_or", |a, b| a | b).await;
}

#[tokio::test]
async fn testBitAndComparison() {
    bit_comparison("BIT_AND", "bit_and", |a, b| a & b).await;
}

#[tokio::test]
async fn testInArithmeticExpression1() {
    let em = set_up().await;
    let dql = "SELECT m, m.name AS m_name
            FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m
            WHERE m.salary IN (800000 / 8, 100000 * 2)
            ORDER BY m.name DESC";

    let result = array_result(&em, dql).await;

    assert_column(&result, "m_name", &strings(&["Roman B.", "Benjamin E."]));
}

#[tokio::test]
async fn testInArithmeticExpression2() {
    register_function_class(
        "Doctrine\\Tests\\ORM\\Functional\\NoOp",
        Arc::new(|name| {
            Box::new(NoOp {
                name: name.to_owned(),
                field: None,
            })
        }),
    );
    company::register_listeners();
    let em = orm::entity_manager_with(orm::connection(), |config| {
        config.add_custom_string_function("FOO", "Doctrine\\Tests\\ORM\\Functional\\NoOp");
    });
    orm::create_schema(&em, &COMPANY_MODELS).await;
    generate_fixture(&em).await;

    let dql = "SELECT m, m.name AS m_name
            FROM Doctrine\\Tests\\Models\\Company\\CompanyManager m
            WHERE m.department IN (FOO('Administration'))";

    let result = array_result(&em, dql).await;

    assert_column(&result, "m_name", &strings(&["Jonathan W."]));
}

/// `testDateAddWithColumnInterval()` and `testDateSubWithColumnInterval()`.
async fn date_with_column_interval(function: &str) {
    let em = set_up().await;
    let dql = format!(
        "SELECT {function}(CURRENT_TIMESTAMP(), m.salary, 'day') AS add FROM {COMPANY_EMPLOYEE} m"
    );

    let query = em.create_query(&dql);
    query.set_max_results(Some(1));
    let result = query.get_single_result(Some(HYDRATE_ARRAY)).await.unwrap();

    let Value::Array(row) = &result else {
        panic!("not an array: {result:?}");
    };
    assert!(row.contains_key(&"add".into()));
}

#[tokio::test]
async fn testDateAddWithColumnInterval() {
    date_with_column_interval("DATE_ADD").await;
}

#[tokio::test]
async fn testDateSubWithColumnInterval() {
    date_with_column_interval("DATE_SUB").await;
}
