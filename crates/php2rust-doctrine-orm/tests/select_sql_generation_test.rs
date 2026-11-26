//! Port of `SelectSqlGenerationTest`: the SQL of DQL SELECT statements on
//! the test entity manager (`tests/common/dql.rs`). Parameters whose
//! value is an entity or a `ClassMetadata` are set to `null`: the SQL
//! does not depend on the value. The tests on the `MySQLPlatform`,
//! `PostgreSQLPlatform` and `OraclePlatform` are pending: the DBAL port
//! has only `SQLitePlatform`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/SelectSqlGenerationTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/dql.rs"]
mod dql;
#[path = "common/models.rs"]
mod models;

use std::sync::Arc;

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::LockMode;
use php2rust_doctrine_orm::query::ast::Node;
use php2rust_doctrine_orm::query::ast::functions::FunctionNode;
use php2rust_doctrine_orm::query::context::{
    FunctionFactory, HINT_FORCE_PARTIAL_LOAD, HINT_LOCK_MODE, HYDRATE_OBJECT, Hint,
};
use php2rust_doctrine_orm::query::{Parser, SqlWalker, TokenType};
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;

use dql::{TestEntityManager, thrown};

const QUERY_EXCEPTION: &str = r"Doctrine\ORM\Query\QueryException";

/// `MyAbsFunction`.
#[derive(Debug)]
struct MyAbsFunction {
    name: String,
    simple_arithmetic_expression: Option<Node>,
}

impl FunctionNode for MyAbsFunction {
    fn class(&self) -> &'static str {
        r"Doctrine\Tests\ORM\Query\MyAbsFunction"
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let expression = self.simple_arithmetic_expression.as_ref().unwrap();
        Ok(format!(
            "ABS({})",
            sql_walker.walk_simple_arithmetic_expression(expression)?
        ))
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        parser.match_token(TokenType::T_IDENTIFIER)?;
        parser.match_token(TokenType::T_OPEN_PARENTHESIS)?;
        self.simple_arithmetic_expression = Some(parser.simple_arithmetic_expression()?);
        parser.match_token(TokenType::T_CLOSE_PARENTHESIS)
    }
}

fn my_abs_function() -> FunctionFactory {
    Arc::new(|name| {
        Box::new(MyAbsFunction {
            name: name.to_owned(),
            simple_arithmetic_expression: None,
        })
    })
}

fn partial_load(enabled: bool) -> [(&'static str, Hint); 1] {
    [(HINT_FORCE_PARTIAL_LOAD, Hint::Value(Value::Bool(enabled)))]
}

fn assert_get_sql(
    entity_manager: &TestEntityManager,
    dql: &str,
    sql: &str,
    params: &[(&str, Value)],
) {
    let query = entity_manager.create_query(dql);
    for (name, value) in params {
        query.set_parameter(name, value.clone(), None);
    }
    assert_eq!(sql, query.get_single_sql().unwrap());
}

fn assert_sql_generation(dql_to_be_tested: &str, sql_to_be_confirmed: &str) {
    assert_sql_generation_with(
        &TestEntityManager::new(),
        dql_to_be_tested,
        sql_to_be_confirmed,
        &[],
        &[],
        HYDRATE_OBJECT,
    );
}

fn assert_sql_generation_hints(
    dql_to_be_tested: &str,
    sql_to_be_confirmed: &str,
    query_hints: &[(&str, Hint)],
) {
    assert_sql_generation_with(
        &TestEntityManager::new(),
        dql_to_be_tested,
        sql_to_be_confirmed,
        query_hints,
        &[],
        HYDRATE_OBJECT,
    );
}

fn assert_sql_generation_with(
    entity_manager: &TestEntityManager,
    dql_to_be_tested: &str,
    sql_to_be_confirmed: &str,
    query_hints: &[(&str, Hint)],
    query_params: &[(&str, Value)],
    hydration_mode: i64,
) {
    let query = entity_manager.create_query(dql_to_be_tested);
    query.set_hydration_mode(hydration_mode);
    for (name, value) in query_params {
        query.set_parameter(name, value.clone(), None);
    }
    for (name, value) in query_hints {
        query.hint(name, value.clone());
    }
    assert_eq!(sql_to_be_confirmed, query.get_single_sql().unwrap());
}

fn assert_invalid_sql_generation(dql_to_be_tested: &str, expected_exception: &str) {
    let entity_manager = TestEntityManager::new();
    let query = entity_manager.create_query(dql_to_be_tested);
    match query.get_single_sql() {
        Ok(sql) => panic!("{sql}"),
        Err(error) => assert!(
            thrown_is_a(&error, expected_exception),
            "{:?}",
            thrown(&error)
        ),
    }
}

fn assert_invalid_sql_generation_message(
    dql_to_be_tested: &str,
    expected_exception: &str,
    message: &str,
) {
    let entity_manager = TestEntityManager::new();
    let query = entity_manager.create_query(dql_to_be_tested);
    match query.get_single_sql() {
        Ok(sql) => panic!("{sql}"),
        Err(error) => {
            assert!(
                thrown_is_a(&error, expected_exception),
                "{:?}",
                thrown(&error)
            );
            assert_eq!(message, thrown(&error).1);
        }
    }
}

#[test]
fn testJoinWithRangeVariablePutsConditionIntoSqlWhereClause() {
    assert_sql_generation(
        r"SELECT c.id FROM Doctrine\Tests\Models\Company\CompanyPerson c JOIN Doctrine\Tests\Models\Company\CompanyPerson r WHERE c.spouse = r AND r.id = 42",
        "SELECT c0_.id AS id_0 FROM company_persons c0_ LEFT JOIN company_managers c1_ ON c0_.id = c1_.id LEFT JOIN company_employees c2_ ON c0_.id = c2_.id INNER JOIN company_persons c3_ LEFT JOIN company_managers c4_ ON c3_.id = c4_.id LEFT JOIN company_employees c5_ ON c3_.id = c5_.id WHERE c0_.spouse_id = c3_.id AND c3_.id = 42",
    );
}

#[test]
fn testJoinWithRangeVariableAndInheritancePutsConditionIntoSqlWhereClause() {
    assert_sql_generation(
        r"SELECT c.id FROM Doctrine\Tests\Models\Company\CompanyPerson c JOIN Doctrine\Tests\Models\Company\CompanyPerson r WHERE c.spouse = r AND r.id = 42",
        "SELECT c0_.id AS id_0 FROM company_persons c0_ LEFT JOIN company_managers c1_ ON c0_.id = c1_.id LEFT JOIN company_employees c2_ ON c0_.id = c2_.id INNER JOIN company_persons c3_ LEFT JOIN company_managers c4_ ON c3_.id = c4_.id LEFT JOIN company_employees c5_ ON c3_.id = c5_.id WHERE c0_.spouse_id = c3_.id AND c3_.id = 42",
    );
}

#[test]
fn testSupportsSelectForAllFields() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_",
    );
}

#[test]
fn testSupportsSelectForOneField() {
    assert_sql_generation(
        r"SELECT u.id FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT c0_.id AS id_0 FROM cms_users c0_",
    );
}

#[test]
fn testSupportsSelectForOneNestedField() {
    assert_sql_generation(
        r"SELECT u.id FROM Doctrine\Tests\Models\CMS\CmsArticle a JOIN a.user u",
        "SELECT c0_.id AS id_0 FROM cms_articles c1_ INNER JOIN cms_users c0_ ON c1_.user_id = c0_.id",
    );
}

#[test]
fn testSupportsSelectForAllNestedField() {
    assert_sql_generation(
        r"SELECT a FROM Doctrine\Tests\Models\CMS\CmsArticle a JOIN a.user u ORDER BY u.name ASC",
        "SELECT c0_.id AS id_0, c0_.topic AS topic_1, c0_.text AS text_2, c0_.version AS version_3, c0_.user_id AS user_id_4 FROM cms_articles c0_ INNER JOIN cms_users c1_ ON c0_.user_id = c1_.id ORDER BY c1_.name ASC",
    );
}

#[test]
fn testNotExistsExpression() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE NOT EXISTS (SELECT p.phonenumber FROM Doctrine\Tests\Models\CMS\CmsPhonenumber p WHERE p.phonenumber = 1234)",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE NOT EXISTS (SELECT c1_.phonenumber FROM cms_phonenumbers c1_ WHERE c1_.phonenumber = 1234)",
    );
}

#[test]
fn testSupportsSelectForMultipleColumnsOfASingleComponent() {
    assert_sql_generation(
        r"SELECT u.username, u.name FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT c0_.username AS username_0, c0_.name AS name_1 FROM cms_users c0_",
    );
}

#[test]
fn testSupportsSelectUsingMultipleFromComponents() {
    assert_sql_generation(
        r"SELECT u, p FROM Doctrine\Tests\Models\CMS\CmsUser u, Doctrine\Tests\Models\CMS\CmsPhonenumber p WHERE u = p.user",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c1_.phonenumber AS phonenumber_4, c0_.email_id AS email_id_5, c1_.user_id AS user_id_6 FROM cms_users c0_, cms_phonenumbers c1_ WHERE c0_.id = c1_.user_id",
    );
}

#[test]
fn testSupportsJoinOnMultipleComponents() {
    assert_sql_generation(
        r"SELECT u, p FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN Doctrine\Tests\Models\CMS\CmsPhonenumber p ON u = p.user",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c1_.phonenumber AS phonenumber_4, c0_.email_id AS email_id_5, c1_.user_id AS user_id_6 FROM cms_users c0_ INNER JOIN cms_phonenumbers c1_ ON (c0_.id = c1_.user_id)",
    );
}

#[test]
fn testSupportsJoinOnMultipleComponentsWithJoinedInheritanceType() {
    assert_sql_generation(
        r"SELECT e FROM Doctrine\Tests\Models\Company\CompanyEmployee e JOIN Doctrine\Tests\Models\Company\CompanyManager m ON e.id = m.id",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.salary AS salary_2, c1_.department AS department_3, c1_.startDate AS startDate_4, c2_.title AS title_5, c0_.discr AS discr_6, c0_.spouse_id AS spouse_id_7, c2_.car_id AS car_id_8 FROM company_employees c1_ INNER JOIN company_persons c0_ ON c1_.id = c0_.id LEFT JOIN company_managers c2_ ON c1_.id = c2_.id INNER JOIN (company_managers c3_ INNER JOIN company_employees c5_ ON c3_.id = c5_.id INNER JOIN company_persons c4_ ON c3_.id = c4_.id) ON (c0_.id = c4_.id)",
    );
    assert_sql_generation(
        r"SELECT e FROM Doctrine\Tests\Models\Company\CompanyEmployee e LEFT JOIN Doctrine\Tests\Models\Company\CompanyManager m ON e.id = m.id",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.salary AS salary_2, c1_.department AS department_3, c1_.startDate AS startDate_4, c2_.title AS title_5, c0_.discr AS discr_6, c0_.spouse_id AS spouse_id_7, c2_.car_id AS car_id_8 FROM company_employees c1_ INNER JOIN company_persons c0_ ON c1_.id = c0_.id LEFT JOIN company_managers c2_ ON c1_.id = c2_.id LEFT JOIN (company_managers c3_ INNER JOIN company_employees c5_ ON c3_.id = c5_.id INNER JOIN company_persons c4_ ON c3_.id = c4_.id) ON (c0_.id = c4_.id)",
    );
    assert_sql_generation(
        r"SELECT c FROM Doctrine\Tests\Models\Company\CompanyContract c JOIN c.salesPerson s LEFT JOIN Doctrine\Tests\Models\Company\CompanyEvent e ON s.id = e.id",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.fixPrice AS fixPrice_2, c0_.hoursWorked AS hoursWorked_3, c0_.pricePerHour AS pricePerHour_4, c0_.maxPrice AS maxPrice_5, c0_.discr AS discr_6, c0_.salesPerson_id AS salesPerson_id_7 FROM company_contracts c0_ INNER JOIN company_employees c1_ ON c0_.salesPerson_id = c1_.id LEFT JOIN company_persons c2_ ON c1_.id = c2_.id LEFT JOIN company_managers c3_ ON c1_.id = c3_.id LEFT JOIN (company_events c4_ LEFT JOIN company_auctions c5_ ON c4_.id = c5_.id LEFT JOIN company_raffles c6_ ON c4_.id = c6_.id) ON (c2_.id = c4_.id) WHERE c0_.discr IN ('fix', 'flexible', 'flexultra')",
    );
}

#[test]
fn testSupportsSelectWithCollectionAssociationJoin() {
    assert_sql_generation(
        r"SELECT u, p FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.phonenumbers p",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c1_.phonenumber AS phonenumber_4, c0_.email_id AS email_id_5, c1_.user_id AS user_id_6 FROM cms_users c0_ INNER JOIN cms_phonenumbers c1_ ON c0_.id = c1_.user_id",
    );
}

#[test]
fn testSupportsSelectWithSingleValuedAssociationJoin() {
    assert_sql_generation(
        r"SELECT u, a FROM Doctrine\Tests\Models\Forum\ForumUser u JOIN u.avatar a",
        "SELECT f0_.id AS id_0, f0_.username AS username_1, f1_.id AS id_2, f0_.avatar_id AS avatar_id_3 FROM forum_users f0_ INNER JOIN forum_avatars f1_ ON f0_.avatar_id = f1_.id",
    );
}

#[test]
fn testSelectCorrelatedSubqueryComplexMathematicalExpression() {
    assert_sql_generation(
        r"SELECT (SELECT (count(p.phonenumber)+5)*10 FROM Doctrine\Tests\Models\CMS\CmsPhonenumber p JOIN p.user ui WHERE ui.id = u.id) AS c FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT (SELECT (count(c0_.phonenumber) + 5) * 10 AS sclr_1 FROM cms_phonenumbers c0_ INNER JOIN cms_users c1_ ON c0_.user_id = c1_.id WHERE c1_.id = c2_.id) AS sclr_0 FROM cms_users c2_",
    );
}

#[test]
fn testSelectComplexMathematicalExpression() {
    assert_sql_generation(
        r"SELECT (count(p.phonenumber)+5)*10 FROM Doctrine\Tests\Models\CMS\CmsPhonenumber p JOIN p.user ui WHERE ui.id = ?1",
        "SELECT (count(c0_.phonenumber) + 5) * 10 AS sclr_0 FROM cms_phonenumbers c0_ INNER JOIN cms_users c1_ ON c0_.user_id = c1_.id WHERE c1_.id = ?",
    );
}

#[test]
fn testConstantValueInSelect() {
    assert_sql_generation(
        r"SELECT u.name, 'foo' AS bar FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT c0_.name AS name_0, 'foo' AS sclr_1 FROM cms_users c0_",
    );
}

#[test]
fn testSupportsOrderByWithAscAsDefault() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\Forum\ForumUser u ORDER BY u.id",
        "SELECT f0_.id AS id_0, f0_.username AS username_1, f0_.avatar_id AS avatar_id_2 FROM forum_users f0_ ORDER BY f0_.id ASC",
    );
}

#[test]
fn testSupportsOrderByAsc() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\Forum\ForumUser u ORDER BY u.id asc",
        "SELECT f0_.id AS id_0, f0_.username AS username_1, f0_.avatar_id AS avatar_id_2 FROM forum_users f0_ ORDER BY f0_.id ASC",
    );
}

#[test]
fn testSupportsOrderByDesc() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\Forum\ForumUser u ORDER BY u.id desc",
        "SELECT f0_.id AS id_0, f0_.username AS username_1, f0_.avatar_id AS avatar_id_2 FROM forum_users f0_ ORDER BY f0_.id DESC",
    );
}

#[test]
fn testSupportsOrderByWithSimpleArithmeticExpression() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\Forum\ForumUser u ORDER BY LENGTH(u.username) + LENGTH(u.username) asc",
        "SELECT f0_.id AS id_0, f0_.username AS username_1, f0_.avatar_id AS avatar_id_2 FROM forum_users f0_ ORDER BY LENGTH(f0_.username) + LENGTH(f0_.username) ASC",
    );
}

#[test]
fn testSupportsSelectDistinct() {
    assert_sql_generation(
        r"SELECT DISTINCT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT DISTINCT c0_.name AS name_0 FROM cms_users c0_",
    );
}

#[test]
fn testSupportsAggregateFunctionInSelectedFields() {
    assert_sql_generation(
        r"SELECT COUNT(u.id) FROM Doctrine\Tests\Models\CMS\CmsUser u GROUP BY u.id",
        "SELECT COUNT(c0_.id) AS sclr_0 FROM cms_users c0_ GROUP BY c0_.id",
    );
}

#[test]
fn testSupportsAggregateFunctionWithSimpleArithmetic() {
    assert_sql_generation(
        r"SELECT MAX(u.id + 4) * 2 FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT MAX(c0_.id + 4) * 2 AS sclr_0 FROM cms_users c0_",
    );
}

#[test]
fn testSupportsWhereClauseWithPositionalParameter() {
    assert_sql_generation(
        r"select u from Doctrine\Tests\Models\Forum\ForumUser u where u.id = ?1",
        "SELECT f0_.id AS id_0, f0_.username AS username_1, f0_.avatar_id AS avatar_id_2 FROM forum_users f0_ WHERE f0_.id = ?",
    );
}

#[test]
fn testSupportsWhereClauseWithNamedParameter() {
    assert_sql_generation(
        r"select u from Doctrine\Tests\Models\Forum\ForumUser u where u.username = :name",
        "SELECT f0_.id AS id_0, f0_.username AS username_1, f0_.avatar_id AS avatar_id_2 FROM forum_users f0_ WHERE f0_.username = ?",
    );
}

#[test]
fn testSupportsWhereAndClauseWithNamedParameters() {
    assert_sql_generation(
        r"select u from Doctrine\Tests\Models\Forum\ForumUser u where u.username = :name and u.username = :name2",
        "SELECT f0_.id AS id_0, f0_.username AS username_1, f0_.avatar_id AS avatar_id_2 FROM forum_users f0_ WHERE f0_.username = ? AND f0_.username = ?",
    );
}

#[test]
fn testSupportsCombinedWhereClauseWithNamedParameter() {
    assert_sql_generation(
        r"select u from Doctrine\Tests\Models\Forum\ForumUser u where (u.username = :name OR u.username = :name2) AND u.id = :id",
        "SELECT f0_.id AS id_0, f0_.username AS username_1, f0_.avatar_id AS avatar_id_2 FROM forum_users f0_ WHERE (f0_.username = ? OR f0_.username = ?) AND f0_.id = ?",
    );
}

#[test]
fn testSupportsAggregateFunctionInASelectDistinct() {
    assert_sql_generation(
        r"SELECT COUNT(DISTINCT u.name) FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT COUNT(DISTINCT c0_.name) AS sclr_0 FROM cms_users c0_",
    );
}

#[test]
fn testSupportsASqlKeywordInAStringLiteralParam() {
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name LIKE '%foo OR bar%'",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ WHERE c0_.name LIKE '%foo OR bar%'",
    );
}

#[test]
fn testSupportsArithmeticExpressionsInWherePart() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE ((u.id + 5000) * u.id + 3) < 10000000",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE ((c0_.id + 5000) * c0_.id + 3) < 10000000",
    );
}

#[test]
fn testSupportsMultipleEntitiesInFromClause() {
    assert_sql_generation(
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u, Doctrine\Tests\Models\CMS\CmsArticle a JOIN a.user u2 WHERE u.id = u2.id",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c1_.id AS id_4, c1_.topic AS topic_5, c1_.text AS text_6, c1_.version AS version_7, c0_.email_id AS email_id_8, c1_.user_id AS user_id_9 FROM cms_users c0_, cms_articles c1_ INNER JOIN cms_users c2_ ON c1_.user_id = c2_.id WHERE c0_.id = c2_.id",
    );
}

#[test]
fn testSupportsMultipleEntitiesInFromClauseUsingPathExpression() {
    assert_sql_generation(
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u, Doctrine\Tests\Models\CMS\CmsArticle a WHERE u.id = a.user",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c1_.id AS id_4, c1_.topic AS topic_5, c1_.text AS text_6, c1_.version AS version_7, c0_.email_id AS email_id_8, c1_.user_id AS user_id_9 FROM cms_users c0_, cms_articles c1_ WHERE c0_.id = c1_.user_id",
    );
}

#[test]
fn testSupportsPlainJoinWithoutClause() {
    assert_sql_generation(
        r"SELECT u.id, a.id from Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.articles a",
        "SELECT c0_.id AS id_0, c1_.id AS id_1 FROM cms_users c0_ LEFT JOIN cms_articles c1_ ON c0_.id = c1_.user_id",
    );
    assert_sql_generation(
        r"SELECT u.id, a.id from Doctrine\Tests\Models\CMS\CmsUser u JOIN u.articles a",
        "SELECT c0_.id AS id_0, c1_.id AS id_1 FROM cms_users c0_ INNER JOIN cms_articles c1_ ON c0_.id = c1_.user_id",
    );
}

#[test]
fn testSupportsJoinAndWithClauseRestriction() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.articles a WITH a.topic LIKE '%foo%'",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ LEFT JOIN cms_articles c1_ ON c0_.id = c1_.user_id AND (c1_.topic LIKE '%foo%')",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.articles a WITH a.topic LIKE '%foo%'",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ INNER JOIN cms_articles c1_ ON c0_.id = c1_.user_id AND (c1_.topic LIKE '%foo%')",
    );
}

#[test]
fn testJoinOnClauseNotYetSupportedThrowsException() {
    assert_invalid_sql_generation(
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.articles a ON a.topic LIKE '%foo%'",
        QUERY_EXCEPTION,
    );
}

#[test]
fn testSupportsMultipleJoins() {
    assert_sql_generation(
        r"SELECT u.id, a.id, p.phonenumber, c.id from Doctrine\Tests\Models\CMS\CmsUser u JOIN u.articles a JOIN u.phonenumbers p JOIN a.comments c",
        "SELECT c0_.id AS id_0, c1_.id AS id_1, c2_.phonenumber AS phonenumber_2, c3_.id AS id_3 FROM cms_users c0_ INNER JOIN cms_articles c1_ ON c0_.id = c1_.user_id INNER JOIN cms_phonenumbers c2_ ON c0_.id = c2_.user_id INNER JOIN cms_comments c3_ ON c1_.id = c3_.article_id",
    );
}

#[test]
fn testSupportsBetweenClauseWithPositionalParameters() {
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id BETWEEN ?1 AND ?2",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ WHERE c0_.id BETWEEN ? AND ?",
    );
}

#[test]
fn testSupportsNotBetweenForSizeFunction() {
    assert_sql_generation(
        r"SELECT m.name FROM Doctrine\Tests\Models\StockExchange\Market m WHERE SIZE(m.stocks) NOT BETWEEN ?1 AND ?2",
        "SELECT e0_.name AS name_0 FROM exchange_markets e0_ WHERE (SELECT COUNT(*) FROM exchange_stocks e1_ WHERE e1_.market_id = e0_.id) NOT BETWEEN ? AND ?",
    );
}

#[test]
fn testSupportsFunctionalExpressionsInWherePart() {
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE TRIM(u.name) = 'someone'",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ WHERE TRIM(c0_.name) = 'someone'",
    );
}

#[test]
fn testSupportsInstanceOfExpressionsInWherePart() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\Company\CompanyPerson u WHERE u INSTANCE OF Doctrine\Tests\Models\Company\CompanyEmployee",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.title AS title_2, c2_.salary AS salary_3, c2_.department AS department_4, c2_.startDate AS startDate_5, c0_.discr AS discr_6, c0_.spouse_id AS spouse_id_7, c1_.car_id AS car_id_8 FROM company_persons c0_ LEFT JOIN company_managers c1_ ON c0_.id = c1_.id LEFT JOIN company_employees c2_ ON c0_.id = c2_.id WHERE c0_.discr IN ('manager', 'employee')",
    );
}

#[test]
fn testSupportsInstanceOfExpressionInWherePartWithMultipleValues() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\Company\CompanyPerson u WHERE u INSTANCE OF (Doctrine\Tests\Models\Company\CompanyEmployee, \Doctrine\Tests\Models\Company\CompanyManager)",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.title AS title_2, c2_.salary AS salary_3, c2_.department AS department_4, c2_.startDate AS startDate_5, c0_.discr AS discr_6, c0_.spouse_id AS spouse_id_7, c1_.car_id AS car_id_8 FROM company_persons c0_ LEFT JOIN company_managers c1_ ON c0_.id = c1_.id LEFT JOIN company_employees c2_ ON c0_.id = c2_.id WHERE c0_.discr IN ('manager', 'employee')",
    );
}

#[test]
fn testSupportsInstanceOfExpressionsInWherePartPrefixedSlash() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\Company\CompanyPerson u WHERE u INSTANCE OF \Doctrine\Tests\Models\Company\CompanyEmployee",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.title AS title_2, c2_.salary AS salary_3, c2_.department AS department_4, c2_.startDate AS startDate_5, c0_.discr AS discr_6, c0_.spouse_id AS spouse_id_7, c1_.car_id AS car_id_8 FROM company_persons c0_ LEFT JOIN company_managers c1_ ON c0_.id = c1_.id LEFT JOIN company_employees c2_ ON c0_.id = c2_.id WHERE c0_.discr IN ('manager', 'employee')",
    );
}

#[test]
fn testSupportsInstanceOfExpressionsInWherePartWithUnrelatedClass() {
    assert_invalid_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\Company\CompanyPerson u WHERE u INSTANCE OF \Doctrine\Tests\Models\CMS\CmsUser",
        r"Doctrine\ORM\Query\QueryException",
    );
}

#[test]
fn testSupportsInstanceOfExpressionsInWherePartInDeeperLevel() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\Company\CompanyEmployee u WHERE u INSTANCE OF Doctrine\Tests\Models\Company\CompanyManager",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.salary AS salary_2, c1_.department AS department_3, c1_.startDate AS startDate_4, c2_.title AS title_5, c0_.discr AS discr_6, c0_.spouse_id AS spouse_id_7, c2_.car_id AS car_id_8 FROM company_employees c1_ INNER JOIN company_persons c0_ ON c1_.id = c0_.id LEFT JOIN company_managers c2_ ON c1_.id = c2_.id WHERE c0_.discr IN ('manager')",
    );
}

#[test]
fn testSupportsInstanceOfExpressionsInWherePartInDeepestLevel() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\Company\CompanyManager u WHERE u INSTANCE OF Doctrine\Tests\Models\Company\CompanyManager",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.salary AS salary_2, c1_.department AS department_3, c1_.startDate AS startDate_4, c2_.title AS title_5, c0_.discr AS discr_6, c0_.spouse_id AS spouse_id_7, c2_.car_id AS car_id_8 FROM company_managers c2_ INNER JOIN company_employees c1_ ON c2_.id = c1_.id INNER JOIN company_persons c0_ ON c2_.id = c0_.id WHERE c0_.discr IN ('manager')",
    );
}

#[test]
fn testSupportsInstanceOfExpressionsUsingInputParameterInWherePart() {
    // The parameter is `getClassMetadata(CompanyEmployee::class)`; the
    // SQL does not depend on it.
    assert_sql_generation_with(
        &TestEntityManager::new(),
        r"SELECT u FROM Doctrine\Tests\Models\Company\CompanyPerson u WHERE u INSTANCE OF ?1",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.title AS title_2, c2_.salary AS salary_3, c2_.department AS department_4, c2_.startDate AS startDate_5, c0_.discr AS discr_6, c0_.spouse_id AS spouse_id_7, c1_.car_id AS car_id_8 FROM company_persons c0_ LEFT JOIN company_managers c1_ ON c0_.id = c1_.id LEFT JOIN company_employees c2_ ON c0_.id = c2_.id WHERE c0_.discr IN (?)",
        &[],
        &[("1", Value::Null)],
        HYDRATE_OBJECT,
    );
}

#[test]
fn testSupportsSingleValuedInExpressionWithoutSpacesInWherePart() {
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE IDENTITY(u.email) IN(46)",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ WHERE c0_.email_id IN (46)",
    );
}

#[test]
fn testSupportsMultipleValuedInExpressionInWherePart() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id IN (1, 2)",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE c0_.id IN (1, 2)",
    );
}

#[test]
fn testSupportsNotInExpressionInWherePart() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE :id NOT IN (1)",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE ? NOT IN (1)",
    );
}

#[test]
fn testInExpressionWithSingleValuedAssociationPathExpressionInWherePart() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\Forum\ForumUser u WHERE u.avatar IN (?1, ?2)",
        "SELECT f0_.id AS id_0, f0_.username AS username_1, f0_.avatar_id AS avatar_id_2 FROM forum_users f0_ WHERE f0_.avatar_id IN (?, ?)",
    );
}

#[test]
fn testInvalidInExpressionWithSingleValuedAssociationPathExpressionOnInverseSide() {
    assert_invalid_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.address IN (?1, ?2)",
        r"Doctrine\ORM\Query\QueryException",
    );
}

#[test]
fn testInExpressionWithArithmeticExpression() {
    let entity_manager = TestEntityManager::new();
    entity_manager.add_custom_string_function("FOO", my_abs_function());
    assert_sql_generation_with(
        &entity_manager,
        r"SELECT u FROM Doctrine\Tests\Models\Forum\ForumUser u WHERE u.username IN (FOO('Lo'), 'Lo', :name)",
        "SELECT f0_.id AS id_0, f0_.username AS username_1, f0_.avatar_id AS avatar_id_2 FROM forum_users f0_ WHERE f0_.username IN (ABS('Lo'), 'Lo', ?)",
        &[],
        &[],
        HYDRATE_OBJECT,
    );
    assert_sql_generation_with(
        &entity_manager,
        r"SELECT u FROM Doctrine\Tests\Models\Forum\ForumUser u WHERE u.id IN (1 + 1)",
        "SELECT f0_.id AS id_0, f0_.username AS username_1, f0_.avatar_id AS avatar_id_2 FROM forum_users f0_ WHERE f0_.id IN (1 + 1)",
        &[],
        &[],
        HYDRATE_OBJECT,
    );
}

#[test]
fn testSupportsExistsExpressionInWherePartWithCorrelatedSubquery() {
    assert_sql_generation(
        r"SELECT u.id FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE EXISTS (SELECT p.phonenumber FROM Doctrine\Tests\Models\CMS\CmsPhonenumber p WHERE p.phonenumber = u.id)",
        "SELECT c0_.id AS id_0 FROM cms_users c0_ WHERE EXISTS (SELECT c1_.phonenumber FROM cms_phonenumbers c1_ WHERE c1_.phonenumber = c0_.id)",
    );
}

#[test]
fn testSubqueriesInComparisonExpression() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE (u.id >= (SELECT u2.id FROM Doctrine\Tests\Models\CMS\CmsUser u2 WHERE u2.name = :name)) AND (u.id <= (SELECT u3.id FROM Doctrine\Tests\Models\CMS\CmsUser u3 WHERE u3.name = :name))",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE (c0_.id >= (SELECT c1_.id FROM cms_users c1_ WHERE c1_.name = ?)) AND (c0_.id <= (SELECT c2_.id FROM cms_users c2_ WHERE c2_.name = ?))",
    );
}

#[test]
fn testSupportsMemberOfExpressionOneToMany() {
    // The parameter is a `CmsPhonenumber`; the SQL does not depend on it.
    assert_get_sql(
        &TestEntityManager::new(),
        r"SELECT u.id FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE :param MEMBER OF u.phonenumbers",
        "SELECT c0_.id AS id_0 FROM cms_users c0_ WHERE EXISTS (SELECT 1 FROM cms_phonenumbers c1_ WHERE c0_.id = c1_.user_id AND c1_.phonenumber = ?)",
        &[("param", Value::Null)],
    );
}

#[test]
fn testSupportsMemberOfExpressionManyToMany() {
    // The parameter is a `CmsGroup`; the SQL does not depend on it.
    assert_get_sql(
        &TestEntityManager::new(),
        r"SELECT u.id FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE :param MEMBER OF u.groups",
        "SELECT c0_.id AS id_0 FROM cms_users c0_ WHERE EXISTS (SELECT 1 FROM cms_users_groups c1_ WHERE c1_.user_id = c0_.id AND c1_.group_id IN (?))",
        &[("param", Value::Null)],
    );
}

#[test]
fn testSupportsMemberOfExpressionManyToManyParameterArray() {
    // The parameter is two `CmsGroup`s; the SQL does not depend on them.
    let groups = PhpArray::list([Value::Null, Value::Null]);
    assert_get_sql(
        &TestEntityManager::new(),
        r"SELECT u.id FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE :param MEMBER OF u.groups",
        "SELECT c0_.id AS id_0 FROM cms_users c0_ WHERE EXISTS (SELECT 1 FROM cms_users_groups c1_ WHERE c1_.user_id = c0_.id AND c1_.group_id IN (?))",
        &[("param", Value::Array(groups))],
    );
}

#[test]
fn testSupportsMemberOfExpressionSelfReferencing() {
    // The parameter is a `CompanyPerson`; the SQL does not depend on it.
    assert_get_sql(
        &TestEntityManager::new(),
        r"SELECT p FROM Doctrine\Tests\Models\Company\CompanyPerson p WHERE :param MEMBER OF p.friends",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.title AS title_2, c2_.salary AS salary_3, c2_.department AS department_4, c2_.startDate AS startDate_5, c0_.discr AS discr_6, c0_.spouse_id AS spouse_id_7, c1_.car_id AS car_id_8 FROM company_persons c0_ LEFT JOIN company_managers c1_ ON c0_.id = c1_.id LEFT JOIN company_employees c2_ ON c0_.id = c2_.id WHERE EXISTS (SELECT 1 FROM company_persons_friends c3_ WHERE c3_.person_id = c0_.id AND c3_.friend_id IN (?))",
        &[("param", Value::Null)],
    );
}

#[test]
fn testSupportsMemberOfWithSingleValuedAssociation() {
    assert_get_sql(
        &TestEntityManager::new(),
        r"SELECT u.id FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.email MEMBER OF u.groups",
        "SELECT c0_.id AS id_0 FROM cms_users c0_ WHERE EXISTS (SELECT 1 FROM cms_users_groups c1_ WHERE c1_.user_id = c0_.id AND c1_.group_id IN (c0_.email_id))",
        &[],
    );
}

#[test]
fn testSupportsMemberOfWithIdentificationVariable() {
    assert_get_sql(
        &TestEntityManager::new(),
        r"SELECT u.id FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u MEMBER OF u.groups",
        "SELECT c0_.id AS id_0 FROM cms_users c0_ WHERE EXISTS (SELECT 1 FROM cms_users_groups c1_ WHERE c1_.user_id = c0_.id AND c1_.group_id IN (c0_.id))",
        &[],
    );
}

#[test]
fn testSupportsCurrentDateFunction() {
    assert_get_sql(
        &TestEntityManager::new(),
        r"SELECT d.id FROM Doctrine\Tests\Models\Generic\DateTimeModel d WHERE d.datetime > current_date()",
        "SELECT d0_.id AS id_0 FROM date_time_model d0_ WHERE d0_.col_datetime > CURRENT_DATE",
        &[],
    );
}

#[test]
fn testSupportsCurrentTimeFunction() {
    assert_get_sql(
        &TestEntityManager::new(),
        r"SELECT d.id FROM Doctrine\Tests\Models\Generic\DateTimeModel d WHERE d.time > current_time()",
        "SELECT d0_.id AS id_0 FROM date_time_model d0_ WHERE d0_.col_time > CURRENT_TIME",
        &[],
    );
}

#[test]
fn testSupportsCurrentTimestampFunction() {
    assert_get_sql(
        &TestEntityManager::new(),
        r"SELECT d.id FROM Doctrine\Tests\Models\Generic\DateTimeModel d WHERE d.datetime > current_timestamp()",
        "SELECT d0_.id AS id_0 FROM date_time_model d0_ WHERE d0_.col_datetime > CURRENT_TIMESTAMP",
        &[],
    );
}

#[test]
fn testExistsExpressionInWhereCorrelatedSubqueryAssocCondition() {
    assert_sql_generation(
        r"SELECT DISTINCT emp FROM Doctrine\Tests\Models\CMS\CmsEmployee emp
                WHERE EXISTS (
                    SELECT spouseEmp
                    FROM Doctrine\Tests\Models\CMS\CmsEmployee spouseEmp
                    WHERE spouseEmp = emp.spouse)",
        "SELECT DISTINCT c0_.id AS id_0, c0_.name AS name_1, c0_.spouse_id AS spouse_id_2 FROM cms_employees c0_ WHERE EXISTS (SELECT c1_.id FROM cms_employees c1_ WHERE c1_.id = c0_.spouse_id)",
    );
}

#[test]
fn testExistsExpressionWithSimpleSelectReturningScalar() {
    assert_sql_generation(
        r"SELECT DISTINCT emp FROM Doctrine\Tests\Models\CMS\CmsEmployee emp
                WHERE EXISTS (
                    SELECT 1
                    FROM Doctrine\Tests\Models\CMS\CmsEmployee spouseEmp
                    WHERE spouseEmp = emp.spouse)",
        "SELECT DISTINCT c0_.id AS id_0, c0_.name AS name_1, c0_.spouse_id AS spouse_id_2 FROM cms_employees c0_ WHERE EXISTS (SELECT 1 AS sclr_3 FROM cms_employees c1_ WHERE c1_.id = c0_.spouse_id)",
    );
}

#[test]
fn testLimitFromQueryClass() {
    let entity_manager = TestEntityManager::new();
    let query = entity_manager.create_query(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u");
    query.set_max_results(Some(10));
    assert_eq!(
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ LIMIT 10",
        query.get_single_sql().unwrap()
    );
}

#[test]
fn testLimitAndOffsetFromQueryClass() {
    let entity_manager = TestEntityManager::new();
    let query = entity_manager.create_query(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u");
    query.set_max_results(Some(10)).set_first_result(0);
    // DBAL 2.8+ doesn't add OFFSET part when offset is 0
    let sql = query.get_single_sql().unwrap();
    assert!(
        sql == "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ LIMIT 10"
            || sql
                == "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ LIMIT 10 OFFSET 0",
        "{sql}"
    );
}

#[test]
fn testSizeFunction() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE SIZE(u.phonenumbers) > 1",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE (SELECT COUNT(*) FROM cms_phonenumbers c1_ WHERE c1_.user_id = c0_.id) > 1",
    );
}

#[test]
fn testSizeFunctionSupportsManyToMany() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE SIZE(u.groups) > 1",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE (SELECT COUNT(*) FROM cms_users_groups c1_ WHERE c1_.user_id = c0_.id) > 1",
    );
}

#[test]
fn testEmptyCollectionComparisonExpression() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.phonenumbers IS EMPTY",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE (SELECT COUNT(*) FROM cms_phonenumbers c1_ WHERE c1_.user_id = c0_.id) = 0",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.phonenumbers IS NOT EMPTY",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE (SELECT COUNT(*) FROM cms_phonenumbers c1_ WHERE c1_.user_id = c0_.id) > 0",
    );
}

#[test]
fn testNestedExpressions() {
    assert_sql_generation(
        r"select u from Doctrine\Tests\Models\CMS\CmsUser u where u.id > 10 and u.id < 42 and ((u.id * 2) > 5)",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE c0_.id > 10 AND c0_.id < 42 AND ((c0_.id * 2) > 5)",
    );
}

#[test]
fn testNestedExpressions2() {
    assert_sql_generation(
        r"select u from Doctrine\Tests\Models\CMS\CmsUser u where (u.id > 10) and (u.id < 42 and ((u.id * 2) > 5)) or u.id <> 42",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE (c0_.id > 10) AND (c0_.id < 42 AND ((c0_.id * 2) > 5)) OR c0_.id <> 42",
    );
}

#[test]
fn testNestedExpressions3() {
    assert_sql_generation(
        r"select u from Doctrine\Tests\Models\CMS\CmsUser u where (u.id > 10) and (u.id between 1 and 10 or u.id in (1, 2, 3, 4, 5))",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE (c0_.id > 10) AND (c0_.id BETWEEN 1 AND 10 OR c0_.id IN (1, 2, 3, 4, 5))",
    );
}

#[test]
fn testOrderByCollectionAssociationSize() {
    assert_sql_generation(
        r"select u, size(u.articles) as numArticles from Doctrine\Tests\Models\CMS\CmsUser u order by numArticles",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, (SELECT COUNT(*) FROM cms_articles c1_ WHERE c1_.user_id = c0_.id) AS sclr_4, c0_.email_id AS email_id_5 FROM cms_users c0_ ORDER BY sclr_4 ASC",
    );
}

#[test]
fn testOrderBySupportsSingleValuedPathExpressionOwningSide() {
    assert_sql_generation(
        r"select a from Doctrine\Tests\Models\CMS\CmsArticle a order by a.user",
        "SELECT c0_.id AS id_0, c0_.topic AS topic_1, c0_.text AS text_2, c0_.version AS version_3, c0_.user_id AS user_id_4 FROM cms_articles c0_ ORDER BY c0_.user_id ASC",
    );
}

#[test]
fn testOrderBySupportsSingleValuedPathExpressionInverseSide() {
    assert_invalid_sql_generation(
        r"select u from Doctrine\Tests\Models\CMS\CmsUser u order by u.address",
        QUERY_EXCEPTION,
    );
}

#[test]
fn testBooleanLiteralInWhereOnSqlite() {
    assert_sql_generation(
        r"SELECT b FROM Doctrine\Tests\Models\Generic\BooleanModel b WHERE b.booleanField = true",
        "SELECT b0_.id AS id_0, b0_.booleanField AS booleanField_1 FROM boolean_model b0_ WHERE b0_.booleanField = 1",
    );
    assert_sql_generation(
        r"SELECT b FROM Doctrine\Tests\Models\Generic\BooleanModel b WHERE b.booleanField = false",
        "SELECT b0_.id AS id_0, b0_.booleanField AS booleanField_1 FROM boolean_model b0_ WHERE b0_.booleanField = 0",
    );
}

#[test]
fn testSingleValuedAssociationFieldInWhere() {
    assert_sql_generation(
        r"SELECT p FROM Doctrine\Tests\Models\CMS\CmsPhonenumber p WHERE p.user = ?1",
        "SELECT c0_.phonenumber AS phonenumber_0, c0_.user_id AS user_id_1 FROM cms_phonenumbers c0_ WHERE c0_.user_id = ?",
    );
}

#[test]
fn testSingleValuedAssociationNullCheckOnOwningSide() {
    assert_sql_generation(
        r"SELECT a FROM Doctrine\Tests\Models\CMS\CmsAddress a WHERE a.user IS NULL",
        "SELECT c0_.id AS id_0, c0_.country AS country_1, c0_.zip AS zip_2, c0_.city AS city_3, c0_.user_id AS user_id_4 FROM cms_addresses c0_ WHERE c0_.user_id IS NULL",
    );
}

#[test]
fn testSingleValuedAssociationNullCheckOnInverseSide() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.address a WHERE a.id IS NULL",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ LEFT JOIN cms_addresses c1_ ON c0_.id = c1_.user_id WHERE c1_.id IS NULL",
    );
}

#[test]
fn testStringFunctionLikeExpression() {
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE LOWER(u.name) LIKE '%foo OR bar%'",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ WHERE LOWER(c0_.name) LIKE '%foo OR bar%'",
    );
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE LOWER(u.name) LIKE :str",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ WHERE LOWER(c0_.name) LIKE ?",
    );
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE CONCAT(UPPER(u.name), '_moo') LIKE :str",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ WHERE UPPER(c0_.name) || '_moo' LIKE ?",
    );
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE UPPER(u.name) LIKE UPPER(:str)",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ WHERE UPPER(c0_.name) LIKE UPPER(?)",
    );
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE UPPER(LOWER(u.name)) LIKE UPPER(LOWER(:str))",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ WHERE UPPER(LOWER(c0_.name)) LIKE UPPER(LOWER(?))",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.articles a WITH a.topic LIKE u.name",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ LEFT JOIN cms_articles c1_ ON c0_.id = c1_.user_id AND (c1_.topic LIKE c0_.name)",
    );
}

#[test]
fn testStringFunctionNotLikeExpression() {
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE LOWER(u.name) NOT LIKE '%foo OR bar%'",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ WHERE LOWER(c0_.name) NOT LIKE '%foo OR bar%'",
    );
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE UPPER(LOWER(u.name)) NOT LIKE UPPER(LOWER(:str))",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ WHERE UPPER(LOWER(c0_.name)) NOT LIKE UPPER(LOWER(?))",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.articles a WITH a.topic NOT LIKE u.name",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ LEFT JOIN cms_articles c1_ ON c0_.id = c1_.user_id AND (c1_.topic NOT LIKE c0_.name)",
    );
}

#[test]
fn testOrderedCollectionFetchJoined() {
    assert_sql_generation(
        r"SELECT r, l FROM Doctrine\Tests\Models\Routing\RoutingRoute r JOIN r.legs l",
        "SELECT r0_.id AS id_0, r1_.id AS id_1, r1_.departureDate AS departureDate_2, r1_.arrivalDate AS arrivalDate_3, r1_.from_id AS from_id_4, r1_.to_id AS to_id_5 FROM RoutingRoute r0_ INNER JOIN RoutingRouteLegs r2_ ON r0_.id = r2_.route_id INNER JOIN RoutingLeg r1_ ON r1_.id = r2_.leg_id ORDER BY r1_.departureDate ASC",
    );
}

#[test]
fn testSubselectInSelect() {
    assert_sql_generation(
        r"SELECT u.name, (SELECT COUNT(p.phonenumber) FROM Doctrine\Tests\Models\CMS\CmsPhonenumber p WHERE p.phonenumber = 1234) pcount FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name = 'jon'",
        "SELECT c0_.name AS name_0, (SELECT COUNT(c1_.phonenumber) AS sclr_2 FROM cms_phonenumbers c1_ WHERE c1_.phonenumber = 1234) AS sclr_1 FROM cms_users c0_ WHERE c0_.name = 'jon'",
    );
}

#[test]
fn testLockModeNoneQueryHint() {
    assert_sql_generation_hints(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.username = 'gblanco'",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE c0_.username = 'gblanco'",
        &[(HINT_LOCK_MODE, Hint::LockMode(LockMode::None))],
    );
}

#[test]
fn testSupportSelectWithMoreThan10InputParameters() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = ?1 OR u.id = ?2 OR u.id = ?3 OR u.id = ?4 OR u.id = ?5 OR u.id = ?6 OR u.id = ?7 OR u.id = ?8 OR u.id = ?9 OR u.id = ?10 OR u.id = ?11",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE c0_.id = ? OR c0_.id = ? OR c0_.id = ? OR c0_.id = ? OR c0_.id = ? OR c0_.id = ? OR c0_.id = ? OR c0_.id = ? OR c0_.id = ? OR c0_.id = ? OR c0_.id = ?",
    );
}

#[test]
fn testSupportToCustomDQLFunctions() {
    let entity_manager = TestEntityManager::new();
    entity_manager.add_custom_numeric_function("MYABS", my_abs_function());
    assert_sql_generation_with(
        &entity_manager,
        r"SELECT MYABS(p.phonenumber) FROM Doctrine\Tests\Models\CMS\CmsPhonenumber p",
        "SELECT ABS(c0_.phonenumber) AS sclr_0 FROM cms_phonenumbers c0_",
        &[],
        &[],
        HYDRATE_OBJECT,
    );
}

#[test]
fn testMappedSuperclassAssociationJoin() {
    assert_sql_generation(
        r"SELECT f FROM Doctrine\Tests\Models\DirectoryTree\File f JOIN f.parentDirectory d WHERE f.id = ?1",
        r#"SELECT f0_.id AS id_0, f0_.name AS name_1, f0_.extension AS extension_2, f0_.parentDirectory_id AS parentDirectory_id_3 FROM "file" f0_ INNER JOIN Directory d1_ ON f0_.parentDirectory_id = d1_.id WHERE f0_.id = ?"#,
    );
}

#[test]
fn testGroupBy() {
    assert_sql_generation(
        r"SELECT g.id, count(u.id) FROM Doctrine\Tests\Models\CMS\CmsGroup g JOIN g.users u GROUP BY g.id",
        "SELECT c0_.id AS id_0, count(c1_.id) AS sclr_1 FROM cms_groups c0_ INNER JOIN cms_users_groups c2_ ON c0_.id = c2_.group_id INNER JOIN cms_users c1_ ON c1_.id = c2_.user_id GROUP BY c0_.id",
    );
}

#[test]
fn testGroupByIdentificationVariable() {
    assert_sql_generation(
        r"SELECT g, count(u.id) FROM Doctrine\Tests\Models\CMS\CmsGroup g JOIN g.users u GROUP BY g",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, count(c1_.id) AS sclr_2 FROM cms_groups c0_ INNER JOIN cms_users_groups c2_ ON c0_.id = c2_.group_id INNER JOIN cms_users c1_ ON c1_.id = c2_.user_id GROUP BY c0_.id, c0_.name",
    );
}

#[test]
fn testCaseContainingNullIf() {
    assert_sql_generation(
        r"SELECT NULLIF(g.id, g.name) AS NullIfEqual FROM Doctrine\Tests\Models\CMS\CmsGroup g",
        "SELECT NULLIF(c0_.id, c0_.name) AS sclr_0 FROM cms_groups c0_",
    );
}

#[test]
fn testCaseContainingCoalesce() {
    assert_sql_generation(
        r"SELECT COALESCE(NULLIF(u.name, ''), u.username) as Display FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT COALESCE(NULLIF(c0_.name, ''), c0_.username) AS sclr_0 FROM cms_users c0_",
    );
}

#[test]
fn testSubSelectDiscriminator() {
    assert_sql_generation(
        r"SELECT u.name, (SELECT COUNT(cfc.id) total FROM Doctrine\Tests\Models\Company\CompanyFixContract cfc) as cfc_count FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT c0_.name AS name_0, (SELECT COUNT(c1_.id) AS sclr_2 FROM company_contracts c1_ WHERE c1_.discr IN ('fix')) AS sclr_1 FROM cms_users c0_",
    );
}

#[test]
fn testIdVariableResultVariableReuse() {
    let entity_manager = TestEntityManager::new();
    let query = entity_manager.create_query(r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name IN (SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u)");
    assert!(query.get_single_sql().is_err());
}

#[test]
fn testSubSelectAliasesFromOuterQuery() {
    assert_sql_generation(
        r"SELECT uo, (SELECT ui.name FROM Doctrine\Tests\Models\CMS\CmsUser ui WHERE ui.id = uo.id) AS bar FROM Doctrine\Tests\Models\CMS\CmsUser uo",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, (SELECT c1_.name FROM cms_users c1_ WHERE c1_.id = c0_.id) AS sclr_4, c0_.email_id AS email_id_5 FROM cms_users c0_",
    );
}

#[test]
fn testSubSelectAliasesFromOuterQueryWithSubquery() {
    assert_sql_generation(
        r"SELECT uo, (SELECT ui.name FROM Doctrine\Tests\Models\CMS\CmsUser ui WHERE ui.id = uo.id AND ui.name IN (SELECT uii.name FROM Doctrine\Tests\Models\CMS\CmsUser uii)) AS bar FROM Doctrine\Tests\Models\CMS\CmsUser uo",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, (SELECT c1_.name FROM cms_users c1_ WHERE c1_.id = c0_.id AND c1_.name IN (SELECT c2_.name FROM cms_users c2_)) AS sclr_4, c0_.email_id AS email_id_5 FROM cms_users c0_",
    );
}

#[test]
fn testSubSelectAliasesFromOuterQueryReuseInWhereClause() {
    assert_sql_generation(
        r"SELECT uo, (SELECT ui.name FROM Doctrine\Tests\Models\CMS\CmsUser ui WHERE ui.id = uo.id) AS bar FROM Doctrine\Tests\Models\CMS\CmsUser uo WHERE bar = ?0",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, (SELECT c1_.name FROM cms_users c1_ WHERE c1_.id = c0_.id) AS sclr_4, c0_.email_id AS email_id_5 FROM cms_users c0_ WHERE sclr_4 = ?",
    );
}

#[test]
fn testSelectForeignKeyPKWithoutFields() {
    assert_sql_generation(
        r"SELECT t, s, l FROM Doctrine\Tests\Models\DDC117\DDC117Link l INNER JOIN l.target t INNER JOIN l.source s",
        "SELECT d0_.article_id AS article_id_0, d0_.title AS title_1, d1_.article_id AS article_id_2, d1_.title AS title_3, d2_.source_id AS source_id_4, d2_.target_id AS target_id_5 FROM DDC117Link d2_ INNER JOIN DDC117Article d0_ ON d2_.target_id = d0_.article_id INNER JOIN DDC117Article d1_ ON d2_.source_id = d1_.article_id",
    );
}

#[test]
fn testGeneralCaseWithSingleWhenClause() {
    assert_sql_generation(
        r"SELECT g.id, CASE WHEN ((g.id / 2) > 18) THEN 1 ELSE 0 END AS test FROM Doctrine\Tests\Models\CMS\CmsGroup g",
        "SELECT c0_.id AS id_0, CASE WHEN ((c0_.id / 2) > 18) THEN 1 ELSE 0 END AS sclr_1 FROM cms_groups c0_",
    );
}

#[test]
fn testGeneralCaseWithMultipleWhenClause() {
    assert_sql_generation(
        r"SELECT g.id, CASE WHEN (g.id / 2 < 10) THEN 2 WHEN ((g.id / 2) > 20) THEN 1 ELSE 0 END AS test FROM Doctrine\Tests\Models\CMS\CmsGroup g",
        "SELECT c0_.id AS id_0, CASE WHEN (c0_.id / 2 < 10) THEN 2 WHEN ((c0_.id / 2) > 20) THEN 1 ELSE 0 END AS sclr_1 FROM cms_groups c0_",
    );
}

#[test]
fn testSimpleCaseWithSingleWhenClause() {
    assert_sql_generation(
        r"SELECT g FROM Doctrine\Tests\Models\CMS\CmsGroup g WHERE g.id = CASE g.name WHEN 'admin' THEN 1 ELSE 2 END",
        "SELECT c0_.id AS id_0, c0_.name AS name_1 FROM cms_groups c0_ WHERE c0_.id = CASE c0_.name WHEN 'admin' THEN 1 ELSE 2 END",
    );
}

#[test]
fn testSimpleCaseWithMultipleWhenClause() {
    assert_sql_generation(
        r"SELECT g FROM Doctrine\Tests\Models\CMS\CmsGroup g WHERE g.id = (CASE g.name WHEN 'admin' THEN 1 WHEN 'moderator' THEN 2 ELSE 3 END)",
        "SELECT c0_.id AS id_0, c0_.name AS name_1 FROM cms_groups c0_ WHERE c0_.id = (CASE c0_.name WHEN 'admin' THEN 1 WHEN 'moderator' THEN 2 ELSE 3 END)",
    );
}

#[test]
fn testGeneralCaseWithSingleWhenClauseInSubselect() {
    assert_sql_generation(
        r"SELECT g FROM Doctrine\Tests\Models\CMS\CmsGroup g WHERE g.id IN (SELECT CASE WHEN ((g2.id / 2) > 18) THEN 2 ELSE 1 END FROM Doctrine\Tests\Models\CMS\CmsGroup g2)",
        "SELECT c0_.id AS id_0, c0_.name AS name_1 FROM cms_groups c0_ WHERE c0_.id IN (SELECT CASE WHEN ((c1_.id / 2) > 18) THEN 2 ELSE 1 END AS sclr_2 FROM cms_groups c1_)",
    );
}

#[test]
fn testGeneralCaseWithMultipleWhenClauseInSubselect() {
    assert_sql_generation(
        r"SELECT g FROM Doctrine\Tests\Models\CMS\CmsGroup g WHERE g.id IN (SELECT CASE WHEN (g.id / 2 < 10) THEN 3 WHEN ((g.id / 2) > 20) THEN 2 ELSE 1 END FROM Doctrine\Tests\Models\CMS\CmsGroup g2)",
        "SELECT c0_.id AS id_0, c0_.name AS name_1 FROM cms_groups c0_ WHERE c0_.id IN (SELECT CASE WHEN (c0_.id / 2 < 10) THEN 3 WHEN ((c0_.id / 2) > 20) THEN 2 ELSE 1 END AS sclr_2 FROM cms_groups c1_)",
    );
}

#[test]
fn testSimpleCaseWithSingleWhenClauseInSubselect() {
    assert_sql_generation(
        r"SELECT g FROM Doctrine\Tests\Models\CMS\CmsGroup g WHERE g.id IN (SELECT CASE g2.name WHEN 'admin' THEN 1 ELSE 2 END FROM Doctrine\Tests\Models\CMS\CmsGroup g2)",
        "SELECT c0_.id AS id_0, c0_.name AS name_1 FROM cms_groups c0_ WHERE c0_.id IN (SELECT CASE c1_.name WHEN 'admin' THEN 1 ELSE 2 END AS sclr_2 FROM cms_groups c1_)",
    );
}

#[test]
fn testSimpleCaseWithMultipleWhenClauseInSubselect() {
    assert_sql_generation(
        r"SELECT g FROM Doctrine\Tests\Models\CMS\CmsGroup g WHERE g.id IN (SELECT CASE g2.name WHEN 'admin' THEN 1 WHEN 'moderator' THEN 2 ELSE 3 END FROM Doctrine\Tests\Models\CMS\CmsGroup g2)",
        "SELECT c0_.id AS id_0, c0_.name AS name_1 FROM cms_groups c0_ WHERE c0_.id IN (SELECT CASE c1_.name WHEN 'admin' THEN 1 WHEN 'moderator' THEN 2 ELSE 3 END AS sclr_2 FROM cms_groups c1_)",
    );
}

#[test]
fn testNullLiteralInSelectClause() {
    assert_sql_generation(
        r"SELECT g.id, NULL FROM Doctrine\Tests\Models\CMS\CmsGroup g",
        "SELECT c0_.id AS id_0, NULL AS sclr_1 FROM cms_groups c0_",
    );
}

#[test]
fn testAliasedNullLiteralInSelectClause() {
    assert_sql_generation(
        r"SELECT g.id, NULL AS nil FROM Doctrine\Tests\Models\CMS\CmsGroup g",
        "SELECT c0_.id AS id_0, NULL AS sclr_1 FROM cms_groups c0_",
    );
}

#[test]
fn testNullLiteralInSubselect() {
    assert_sql_generation(
        r"SELECT g.id FROM Doctrine\Tests\Models\CMS\CmsGroup g WHERE g.id IN (SELECT NULL FROM Doctrine\Tests\Models\CMS\CmsGroup g2)",
        "SELECT c0_.id AS id_0 FROM cms_groups c0_ WHERE c0_.id IN (SELECT NULL AS sclr_1 FROM cms_groups c1_)",
    );
}

#[test]
fn testGeneralCaseWithNullLiteralInElseClause() {
    assert_sql_generation(
        r"SELECT g.id, COUNT(CASE WHEN ((g.id / 2) > 18) THEN 1 ELSE NULL END) AS test FROM Doctrine\Tests\Models\CMS\CmsGroup g GROUP BY g.id",
        "SELECT c0_.id AS id_0, COUNT(CASE WHEN ((c0_.id / 2) > 18) THEN 1 ELSE NULL END) AS sclr_1 FROM cms_groups c0_ GROUP BY c0_.id",
    );
}

#[test]
fn testSimpleCaseWithNullLiteralInThenClause() {
    assert_sql_generation(
        r"SELECT g.id, CASE g.name WHEN 'admin' THEN NULL ELSE 1 END AS test FROM Doctrine\Tests\Models\CMS\CmsGroup g",
        "SELECT c0_.id AS id_0, CASE c0_.name WHEN 'admin' THEN NULL ELSE 1 END AS sclr_1 FROM cms_groups c0_",
    );
}

#[test]
fn testSimpleCaseWithStringPrimary() {
    assert_sql_generation(
        r"SELECT g.id, CASE WHEN ((g.id / 2) > 18) THEN 'Foo' ELSE 'Bar' END AS test FROM Doctrine\Tests\Models\CMS\CmsGroup g",
        "SELECT c0_.id AS id_0, CASE WHEN ((c0_.id / 2) > 18) THEN 'Foo' ELSE 'Bar' END AS sclr_1 FROM cms_groups c0_",
    );
}

#[test]
fn testCaseNegativeValuesInThenExpression() {
    assert_sql_generation(
        r"SELECT CASE g.name WHEN 'admin' THEN - 1 ELSE - 2 END FROM Doctrine\Tests\Models\CMS\CmsGroup g",
        "SELECT CASE c0_.name WHEN 'admin' THEN -1 ELSE -2 END AS sclr_0 FROM cms_groups c0_",
    );
    assert_sql_generation(
        r"SELECT CASE g.name WHEN 'admin' THEN  - 2 WHEN 'guest' THEN - 1 ELSE 0 END FROM Doctrine\Tests\Models\CMS\CmsGroup g",
        "SELECT CASE c0_.name WHEN 'admin' THEN -2 WHEN 'guest' THEN -1 ELSE 0 END AS sclr_0 FROM cms_groups c0_",
    );
    assert_sql_generation(
        r"SELECT CASE g.name WHEN 'admin' THEN (- 1) ELSE (- 2) END FROM Doctrine\Tests\Models\CMS\CmsGroup g",
        "SELECT CASE c0_.name WHEN 'admin' THEN (-1) ELSE (-2) END AS sclr_0 FROM cms_groups c0_",
    );
    assert_sql_generation(
        r"SELECT CASE g.name WHEN 'admin' THEN ( - :value) ELSE ( + :value) END FROM Doctrine\Tests\Models\CMS\CmsGroup g",
        "SELECT CASE c0_.name WHEN 'admin' THEN (-?) ELSE (+?) END AS sclr_0 FROM cms_groups c0_",
    );
    assert_sql_generation(
        r"SELECT CASE g.name WHEN 'admin' THEN ( - g.id) ELSE ( + g.id) END FROM Doctrine\Tests\Models\CMS\CmsGroup g",
        "SELECT CASE c0_.name WHEN 'admin' THEN (-c0_.id) ELSE (+c0_.id) END AS sclr_0 FROM cms_groups c0_",
    );
}

#[test]
fn testIdentityFunctionWithCompositePrimaryKey() {
    assert_sql_generation(
        r"SELECT IDENTITY(p.poi, 'long') AS long FROM Doctrine\Tests\Models\Navigation\NavPhotos p",
        "SELECT n0_.poi_long AS sclr_0 FROM navigation_photos n0_",
    );
    assert_sql_generation(
        r"SELECT IDENTITY(p.poi, 'lat') AS lat FROM Doctrine\Tests\Models\Navigation\NavPhotos p",
        "SELECT n0_.poi_lat AS sclr_0 FROM navigation_photos n0_",
    );
    assert_sql_generation(
        r"SELECT IDENTITY(p.poi, 'long') AS long, IDENTITY(p.poi, 'lat') AS lat FROM Doctrine\Tests\Models\Navigation\NavPhotos p",
        "SELECT n0_.poi_long AS sclr_0, n0_.poi_lat AS sclr_1 FROM navigation_photos n0_",
    );
    assert_invalid_sql_generation(
        r"SELECT IDENTITY(p.poi, 'invalid') AS invalid FROM Doctrine\Tests\Models\Navigation\NavPhotos p",
        r"Doctrine\ORM\Query\QueryException",
    );
}

#[test]
fn testPartialWithAssociationIdentifier() {
    assert_sql_generation(
        r"SELECT PARTIAL l.{_source, _target} FROM Doctrine\Tests\Models\Legacy\LegacyUserReference l",
        "SELECT l0_.iUserIdSource AS iUserIdSource_0, l0_.iUserIdTarget AS iUserIdTarget_1 FROM legacy_users_reference l0_",
    );
    assert_sql_generation(
        r"SELECT PARTIAL l.{_description, _source, _target} FROM Doctrine\Tests\Models\Legacy\LegacyUserReference l",
        "SELECT l0_.description AS description_0, l0_.iUserIdSource AS iUserIdSource_1, l0_.iUserIdTarget AS iUserIdTarget_2 FROM legacy_users_reference l0_",
    );
}

#[test]
fn testIdentityFunctionInSelectClause() {
    assert_sql_generation(
        r"SELECT IDENTITY(u.email) as email_id FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT c0_.email_id AS sclr_0 FROM cms_users c0_",
    );
}

#[test]
fn testIdentityFunctionOnInverseSideThrowsQueryException() {
    assert_invalid_sql_generation_message(
        r"SELECT IDENTITY(u.address) FROM Doctrine\Tests\Models\CMS\CmsUser u",
        r"Doctrine\ORM\Query\QueryException",
        r#"A single-valued association path expression to an inverse side is not supported in DQL queries. Instead of "u.address" use an explicit join."#,
    );
}

#[test]
fn testIdentityFunctionInJoinedSubclass() {
    assert_sql_generation(
        r"SELECT m, IDENTITY(m.car) as car_id FROM Doctrine\Tests\Models\Company\CompanyManager m",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.salary AS salary_2, c1_.department AS department_3, c1_.startDate AS startDate_4, c2_.title AS title_5, c2_.car_id AS sclr_6, c0_.discr AS discr_7, c0_.spouse_id AS spouse_id_8, c2_.car_id AS car_id_9 FROM company_managers c2_ INNER JOIN company_employees c1_ ON c2_.id = c1_.id INNER JOIN company_persons c0_ ON c2_.id = c0_.id",
    );
    assert_sql_generation(
        r"SELECT m, IDENTITY(m.spouse) as spouse_id FROM Doctrine\Tests\Models\Company\CompanyManager m",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.salary AS salary_2, c1_.department AS department_3, c1_.startDate AS startDate_4, c2_.title AS title_5, c0_.spouse_id AS sclr_6, c0_.discr AS discr_7, c0_.spouse_id AS spouse_id_8, c2_.car_id AS car_id_9 FROM company_managers c2_ INNER JOIN company_employees c1_ ON c2_.id = c1_.id INNER JOIN company_persons c0_ ON c2_.id = c0_.id",
    );
}

#[test]
fn testIdentityFunctionDoesNotAcceptStateField() {
    assert_invalid_sql_generation(
        r"SELECT IDENTITY(u.name) as name FROM Doctrine\Tests\Models\CMS\CmsUser u",
        r"Doctrine\ORM\Query\QueryException",
    );
}

#[test]
fn testInheritanceTypeJoinInRootClassWithDisabledForcePartialLoad() {
    assert_sql_generation_hints(
        r"SELECT p FROM Doctrine\Tests\Models\Company\CompanyPerson p",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.title AS title_2, c2_.salary AS salary_3, c2_.department AS department_4, c2_.startDate AS startDate_5, c0_.discr AS discr_6, c0_.spouse_id AS spouse_id_7, c1_.car_id AS car_id_8 FROM company_persons c0_ LEFT JOIN company_managers c1_ ON c0_.id = c1_.id LEFT JOIN company_employees c2_ ON c0_.id = c2_.id",
        &partial_load(false),
    );
}

#[test]
fn testInheritanceTypeJoinInRootClassWithEnabledForcePartialLoad() {
    assert_sql_generation_hints(
        r"SELECT p FROM Doctrine\Tests\Models\Company\CompanyPerson p",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c0_.discr AS discr_2 FROM company_persons c0_",
        &partial_load(true),
    );
}

#[test]
fn testInheritanceTypeJoinInChildClassWithDisabledForcePartialLoad() {
    assert_sql_generation_hints(
        r"SELECT e FROM Doctrine\Tests\Models\Company\CompanyEmployee e",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.salary AS salary_2, c1_.department AS department_3, c1_.startDate AS startDate_4, c2_.title AS title_5, c0_.discr AS discr_6, c0_.spouse_id AS spouse_id_7, c2_.car_id AS car_id_8 FROM company_employees c1_ INNER JOIN company_persons c0_ ON c1_.id = c0_.id LEFT JOIN company_managers c2_ ON c1_.id = c2_.id",
        &partial_load(false),
    );
}

#[test]
fn testInheritanceTypeJoinInChildClassWithEnabledForcePartialLoad() {
    assert_sql_generation_hints(
        r"SELECT e FROM Doctrine\Tests\Models\Company\CompanyEmployee e",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.salary AS salary_2, c1_.department AS department_3, c1_.startDate AS startDate_4, c0_.discr AS discr_5 FROM company_employees c1_ INNER JOIN company_persons c0_ ON c1_.id = c0_.id",
        &partial_load(true),
    );
}

#[test]
fn testInheritanceTypeJoinInLeafClassWithDisabledForcePartialLoad() {
    assert_sql_generation_hints(
        r"SELECT m FROM Doctrine\Tests\Models\Company\CompanyManager m",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.salary AS salary_2, c1_.department AS department_3, c1_.startDate AS startDate_4, c2_.title AS title_5, c0_.discr AS discr_6, c0_.spouse_id AS spouse_id_7, c2_.car_id AS car_id_8 FROM company_managers c2_ INNER JOIN company_employees c1_ ON c2_.id = c1_.id INNER JOIN company_persons c0_ ON c2_.id = c0_.id",
        &partial_load(false),
    );
}

#[test]
fn testInheritanceTypeJoinInLeafClassWithEnabledForcePartialLoad() {
    assert_sql_generation_hints(
        r"SELECT m FROM Doctrine\Tests\Models\Company\CompanyManager m",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.salary AS salary_2, c1_.department AS department_3, c1_.startDate AS startDate_4, c2_.title AS title_5, c0_.discr AS discr_6 FROM company_managers c2_ INNER JOIN company_employees c1_ ON c2_.id = c1_.id INNER JOIN company_persons c0_ ON c2_.id = c0_.id",
        &partial_load(true),
    );
}

#[test]
fn testInheritanceTypeSingleTableInRootClassWithDisabledForcePartialLoad() {
    assert_sql_generation_hints(
        r"SELECT c FROM Doctrine\Tests\Models\Company\CompanyContract c",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.fixPrice AS fixPrice_2, c0_.hoursWorked AS hoursWorked_3, c0_.pricePerHour AS pricePerHour_4, c0_.maxPrice AS maxPrice_5, c0_.discr AS discr_6, c0_.salesPerson_id AS salesPerson_id_7 FROM company_contracts c0_ WHERE c0_.discr IN ('fix', 'flexible', 'flexultra')",
        &partial_load(false),
    );
}

#[test]
fn testInheritanceTypeSingleTableInRootClassWithEnabledForcePartialLoad() {
    assert_sql_generation_hints(
        r"SELECT c FROM Doctrine\Tests\Models\Company\CompanyContract c",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.fixPrice AS fixPrice_2, c0_.hoursWorked AS hoursWorked_3, c0_.pricePerHour AS pricePerHour_4, c0_.maxPrice AS maxPrice_5, c0_.discr AS discr_6 FROM company_contracts c0_ WHERE c0_.discr IN ('fix', 'flexible', 'flexultra')",
        &partial_load(true),
    );
}

#[test]
fn testInheritanceTypeSingleTableInChildClassWithDisabledForcePartialLoad() {
    assert_sql_generation_hints(
        r"SELECT fc FROM Doctrine\Tests\Models\Company\CompanyFlexContract fc",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.hoursWorked AS hoursWorked_2, c0_.pricePerHour AS pricePerHour_3, c0_.maxPrice AS maxPrice_4, c0_.discr AS discr_5, c0_.salesPerson_id AS salesPerson_id_6 FROM company_contracts c0_ WHERE c0_.discr IN ('flexible', 'flexultra')",
        &partial_load(false),
    );
}

#[test]
fn testInheritanceTypeSingleTableInChildClassWithEnabledForcePartialLoad() {
    assert_sql_generation_hints(
        r"SELECT fc FROM Doctrine\Tests\Models\Company\CompanyFlexContract fc",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.hoursWorked AS hoursWorked_2, c0_.pricePerHour AS pricePerHour_3, c0_.maxPrice AS maxPrice_4, c0_.discr AS discr_5 FROM company_contracts c0_ WHERE c0_.discr IN ('flexible', 'flexultra')",
        &partial_load(true),
    );
}

#[test]
fn testInheritanceTypeSingleTableInLeafClassWithDisabledForcePartialLoad() {
    assert_sql_generation_hints(
        r"SELECT fuc FROM Doctrine\Tests\Models\Company\CompanyFlexUltraContract fuc",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.hoursWorked AS hoursWorked_2, c0_.pricePerHour AS pricePerHour_3, c0_.maxPrice AS maxPrice_4, c0_.discr AS discr_5, c0_.salesPerson_id AS salesPerson_id_6 FROM company_contracts c0_ WHERE c0_.discr IN ('flexultra')",
        &partial_load(false),
    );
}

#[test]
fn testInheritanceTypeSingleTableInLeafClassWithEnabledForcePartialLoad() {
    assert_sql_generation_hints(
        r"SELECT fuc FROM Doctrine\Tests\Models\Company\CompanyFlexUltraContract fuc",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.hoursWorked AS hoursWorked_2, c0_.pricePerHour AS pricePerHour_3, c0_.maxPrice AS maxPrice_4, c0_.discr AS discr_5 FROM company_contracts c0_ WHERE c0_.discr IN ('flexultra')",
        &partial_load(true),
    );
}

#[test]
fn testSelfReferenceWithOneToOneDoesNotDuplicateAlias() {
    assert_sql_generation(
        r"SELECT p, pp FROM Doctrine\Tests\Models\Company\CompanyPerson p JOIN p.spouse pp",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.title AS title_2, c2_.salary AS salary_3, c2_.department AS department_4, c2_.startDate AS startDate_5, c3_.id AS id_6, c3_.name AS name_7, c4_.title AS title_8, c5_.salary AS salary_9, c5_.department AS department_10, c5_.startDate AS startDate_11, c0_.discr AS discr_12, c0_.spouse_id AS spouse_id_13, c1_.car_id AS car_id_14, c3_.discr AS discr_15, c3_.spouse_id AS spouse_id_16, c4_.car_id AS car_id_17 FROM company_persons c0_ LEFT JOIN company_managers c1_ ON c0_.id = c1_.id LEFT JOIN company_employees c2_ ON c0_.id = c2_.id INNER JOIN company_persons c3_ ON c0_.spouse_id = c3_.id LEFT JOIN company_managers c4_ ON c3_.id = c4_.id LEFT JOIN company_employees c5_ ON c3_.id = c5_.id",
    );
}

#[test]
fn testAliasDoesNotExceedPlatformDefinedLength() {
    assert_sql_generation(
        r"SELECT m FROM Doctrine\Tests\ORM\Query\DDC1384Model m",
        "SELECT d0_.aVeryLongIdentifierThatShouldBeShortenedByTheSQLWalkerFooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooo AS ooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooo_0 FROM DDC1384Model d0_",
    );
}

#[test]
fn testIssue331() {
    assert_sql_generation(
        r"SELECT e.name FROM Doctrine\Tests\Models\Company\CompanyEmployee e",
        "SELECT c0_.name AS name_0 FROM company_employees c1_ INNER JOIN company_persons c0_ ON c1_.id = c0_.id LEFT JOIN company_managers c2_ ON c1_.id = c2_.id",
    );
}

#[test]
fn testForeignKeyAsPrimaryKeySubselect() {
    assert_sql_generation(
        r"SELECT s FROM Doctrine\Tests\Models\DDC117\DDC117Article s WHERE EXISTS (SELECT r FROM Doctrine\Tests\Models\DDC117\DDC117Reference r WHERE r.source = s)",
        "SELECT d0_.article_id AS article_id_0, d0_.title AS title_1 FROM DDC117Article d0_ WHERE EXISTS (SELECT d1_.source_id, d1_.target_id FROM DDC117Reference d1_ WHERE d1_.source_id = d0_.article_id)",
    );
}

#[test]
fn testSelectWithArithmeticExpressionBeforeField() {
    assert_sql_generation(
        r"SELECT - e.value AS value, e.id FROM Doctrine\Tests\ORM\Query\DDC1474Entity e",
        "SELECT -d0_.value AS sclr_0, d0_.id AS id_1 FROM DDC1474Entity d0_",
    );
    assert_sql_generation(
        r"SELECT e.id, + e.value AS value FROM Doctrine\Tests\ORM\Query\DDC1474Entity e",
        "SELECT d0_.id AS id_0, +d0_.value AS sclr_1 FROM DDC1474Entity d0_",
    );
}

#[test]
fn testGroupByAllFieldsWhenObjectHasForeignKeys() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u GROUP BY u",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ GROUP BY c0_.id, c0_.status, c0_.username, c0_.name, c0_.email_id",
    );
    assert_sql_generation(
        r"SELECT e FROM Doctrine\Tests\Models\CMS\CmsEmployee e GROUP BY e",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c0_.spouse_id AS spouse_id_2 FROM cms_employees c0_ GROUP BY c0_.id, c0_.name, c0_.spouse_id",
    );
}

#[test]
fn testGroupBySupportsResultVariable() {
    assert_sql_generation(
        r"SELECT u, u.status AS st FROM Doctrine\Tests\Models\CMS\CmsUser u GROUP BY st",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.status AS status_4, c0_.email_id AS email_id_5 FROM cms_users c0_ GROUP BY c0_.status",
    );
}

#[test]
fn testGroupBySupportsIdentificationVariable() {
    assert_sql_generation(
        r"SELECT u AS user FROM Doctrine\Tests\Models\CMS\CmsUser u GROUP BY user",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ GROUP BY id_0, status_1, username_2, name_3",
    );
}

#[test]
fn testSupportsBitComparison() {
    assert_sql_generation(
        r"SELECT BIT_OR(4,2), BIT_AND(4,2), u FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT (4 | 2) AS sclr_0, (4 & 2) AS sclr_1, c0_.id AS id_2, c0_.status AS status_3, c0_.username AS username_4, c0_.name AS name_5, c0_.email_id AS email_id_6 FROM cms_users c0_",
    );
    assert_sql_generation(
        r"SELECT BIT_OR(u.id,2), BIT_AND(u.id,2) FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE BIT_OR(u.id,2) > 0",
        "SELECT (c0_.id | 2) AS sclr_0, (c0_.id & 2) AS sclr_1 FROM cms_users c0_ WHERE (c0_.id | 2) > 0",
    );
    assert_sql_generation(
        r"SELECT BIT_OR(u.id,2), BIT_AND(u.id,2) FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE BIT_AND(u.id , 4) > 0",
        "SELECT (c0_.id | 2) AS sclr_0, (c0_.id & 2) AS sclr_1 FROM cms_users c0_ WHERE (c0_.id & 4) > 0",
    );
    assert_sql_generation(
        r"SELECT BIT_OR(u.id,2), BIT_AND(u.id,2) FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE BIT_OR(u.id , 2) > 0 OR BIT_AND(u.id , 4) > 0",
        "SELECT (c0_.id | 2) AS sclr_0, (c0_.id & 2) AS sclr_1 FROM cms_users c0_ WHERE (c0_.id | 2) > 0 OR (c0_.id & 4) > 0",
    );
}

#[test]
fn testParenthesesOnTheLeftHandOfComparison() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u where ( (u.id + u.id) * u.id ) > 100",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE ((c0_.id + c0_.id) * c0_.id) > 100",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u where (u.id + u.id) * u.id > 100",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE (c0_.id + c0_.id) * c0_.id > 100",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u where 100 < (u.id + u.id) * u.id ",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE 100 < (c0_.id + c0_.id) * c0_.id",
    );
}

#[test]
fn testSupportsParenthesisExpressionInSubSelect() {
    assert_sql_generation(
        r"SELECT u.id, (SELECT (1000*SUM(subU.id)/SUM(subU.id)) FROM Doctrine\Tests\Models\CMS\CmsUser subU where subU.id = u.id) AS subSelect FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT c0_.id AS id_0, (SELECT (1000 * SUM(c1_.id) / SUM(c1_.id)) FROM cms_users c1_ WHERE c1_.id = c0_.id) AS sclr_1 FROM cms_users c0_",
    );
}

#[test]
fn testSupportsSubSqlFunction() {
    assert_sql_generation(
        r"SELECT u1 FROM Doctrine\Tests\Models\CMS\CmsUser u1 WHERE u1.name IN ( SELECT TRIM(u2.name) FROM Doctrine\Tests\Models\CMS\CmsUser u2 )",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE c0_.name IN (SELECT TRIM(c1_.name) AS sclr_5 FROM cms_users c1_)",
    );
    assert_sql_generation(
        r"SELECT u1 FROM Doctrine\Tests\Models\CMS\CmsUser u1 WHERE u1.name IN ( SELECT TRIM(u2.name) FROM Doctrine\Tests\Models\CMS\CmsUser u2  WHERE LOWER(u2.name) LIKE '%fabio%')",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE c0_.name IN (SELECT TRIM(c1_.name) AS sclr_5 FROM cms_users c1_ WHERE LOWER(c1_.name) LIKE '%fabio%')",
    );
    assert_sql_generation(
        r"SELECT u1 FROM Doctrine\Tests\Models\CMS\CmsUser u1 WHERE u1.email IN ( SELECT TRIM(IDENTITY(u2.email)) FROM Doctrine\Tests\Models\CMS\CmsUser u2 )",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE c0_.email_id IN (SELECT TRIM(c1_.email_id) AS sclr_5 FROM cms_users c1_)",
    );
    assert_sql_generation(
        r"SELECT u1 FROM Doctrine\Tests\Models\CMS\CmsUser u1 WHERE u1.email IN ( SELECT IDENTITY(u2.email) FROM Doctrine\Tests\Models\CMS\CmsUser u2 )",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE c0_.email_id IN (SELECT c1_.email_id AS sclr_5 FROM cms_users c1_)",
    );
    assert_sql_generation(
        r"SELECT u1 FROM Doctrine\Tests\Models\CMS\CmsUser u1 WHERE COUNT(u1.id) = ( SELECT SUM(u2.id) FROM Doctrine\Tests\Models\CMS\CmsUser u2 )",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE COUNT(c0_.id) = (SELECT SUM(c1_.id) AS sclr_5 FROM cms_users c1_)",
    );
    assert_sql_generation(
        r"SELECT u1 FROM Doctrine\Tests\Models\CMS\CmsUser u1 WHERE COUNT(u1.id) <= ( SELECT SUM(u2.id) + COUNT(u2.email) FROM Doctrine\Tests\Models\CMS\CmsUser u2 )",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE COUNT(c0_.id) <= (SELECT SUM(c1_.id) + COUNT(c1_.email_id) AS sclr_5 FROM cms_users c1_)",
    );
}

#[test]
fn testSupportsNewOperator() {
    assert_sql_generation(
        r"SELECT new Doctrine\Tests\Models\CMS\CmsUserDTO(u.name, e.email, a.city) FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.email e JOIN u.address a",
        "SELECT c0_.name AS sclr_0, c1_.email AS sclr_1, c2_.city AS sclr_2 FROM cms_users c0_ INNER JOIN cms_emails c1_ ON c0_.email_id = c1_.id INNER JOIN cms_addresses c2_ ON c0_.id = c2_.user_id",
    );
    assert_sql_generation(
        r"SELECT new Doctrine\Tests\Models\CMS\CmsUserDTO(u.name, e.email, a.id + u.id) FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.email e JOIN u.address a",
        "SELECT c0_.name AS sclr_0, c1_.email AS sclr_1, c2_.id + c0_.id AS sclr_2 FROM cms_users c0_ INNER JOIN cms_emails c1_ ON c0_.email_id = c1_.id INNER JOIN cms_addresses c2_ ON c0_.id = c2_.user_id",
    );
    assert_sql_generation(
        r"SELECT new Doctrine\Tests\Models\CMS\CmsUserDTO(u.name, e.email, a.city, COUNT(p)) FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.email e JOIN u.address a JOIN u.phonenumbers p",
        "SELECT c0_.name AS sclr_0, c1_.email AS sclr_1, c2_.city AS sclr_2, COUNT(c3_.phonenumber) AS sclr_3 FROM cms_users c0_ INNER JOIN cms_emails c1_ ON c0_.email_id = c1_.id INNER JOIN cms_addresses c2_ ON c0_.id = c2_.user_id INNER JOIN cms_phonenumbers c3_ ON c0_.id = c3_.user_id",
    );
    assert_sql_generation(
        r"SELECT new Doctrine\Tests\Models\CMS\CmsUserDTO(u.name, e.email, a.city, COUNT(p) + u.id) FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.email e JOIN u.address a JOIN u.phonenumbers p",
        "SELECT c0_.name AS sclr_0, c1_.email AS sclr_1, c2_.city AS sclr_2, COUNT(c3_.phonenumber) + c0_.id AS sclr_3 FROM cms_users c0_ INNER JOIN cms_emails c1_ ON c0_.email_id = c1_.id INNER JOIN cms_addresses c2_ ON c0_.id = c2_.user_id INNER JOIN cms_phonenumbers c3_ ON c0_.id = c3_.user_id",
    );
    assert_sql_generation(
        r"SELECT new Doctrine\Tests\Models\CMS\CmsUserDTO(a.id, a.country, a.city), new Doctrine\Tests\Models\CMS\CmsAddressDTO(u.name, e.email) FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.email e JOIN u.address a ORDER BY u.name",
        "SELECT c0_.id AS sclr_0, c0_.country AS sclr_1, c0_.city AS sclr_2, c1_.name AS sclr_3, c2_.email AS sclr_4 FROM cms_users c1_ INNER JOIN cms_emails c2_ ON c1_.email_id = c2_.id INNER JOIN cms_addresses c0_ ON c1_.id = c0_.user_id ORDER BY c1_.name ASC",
    );
    assert_sql_generation(
        r"SELECT new Doctrine\Tests\Models\CMS\CmsUserDTO(a.id, (SELECT 1 FROM Doctrine\Tests\Models\CMS\CmsUser su), a.country, a.city), new Doctrine\Tests\Models\CMS\CmsAddressDTO(u.name, e.email) FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.email e JOIN u.address a ORDER BY u.name",
        "SELECT c0_.id AS sclr_0, (SELECT 1 AS sclr_2 FROM cms_users c1_) AS sclr_1, c0_.country AS sclr_3, c0_.city AS sclr_4, c2_.name AS sclr_5, c3_.email AS sclr_6 FROM cms_users c2_ INNER JOIN cms_emails c3_ ON c2_.email_id = c3_.id INNER JOIN cms_addresses c0_ ON c2_.id = c0_.user_id ORDER BY c2_.name ASC",
    );
    assert_sql_generation(
        r"SELECT new Doctrine\Tests\Models\CMS\CmsUserDTO(u.name, e.email, NULL, 123) FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.email e",
        "SELECT c0_.name AS sclr_0, c1_.email AS sclr_1, NULL AS sclr_2, 123 AS sclr_3 FROM cms_users c0_ INNER JOIN cms_emails c1_ ON c0_.email_id = c1_.id",
    );
}

#[test]
fn testWhereFunctionIsNullComparisonExpression() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE IDENTITY(u.email) IS NULL",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE c0_.email_id IS NULL",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE NULLIF(u.name, 'FabioBatSilva') IS NULL AND IDENTITY(u.email) IS NOT NULL",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE NULLIF(c0_.name, 'FabioBatSilva') IS NULL AND c0_.email_id IS NOT NULL",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE IDENTITY(u.email) IS NOT NULL",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE c0_.email_id IS NOT NULL",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE NULLIF(u.name, 'FabioBatSilva') IS NOT NULL",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE NULLIF(c0_.name, 'FabioBatSilva') IS NOT NULL",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE COALESCE(u.name, u.id) IS NOT NULL",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE COALESCE(c0_.name, c0_.id) IS NOT NULL",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE COALESCE(u.id, IDENTITY(u.email)) IS NOT NULL",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE COALESCE(c0_.id, c0_.email_id) IS NOT NULL",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE COALESCE(IDENTITY(u.email), NULLIF(u.name, 'FabioBatSilva')) IS NOT NULL",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE COALESCE(c0_.email_id, NULLIF(c0_.name, 'FabioBatSilva')) IS NOT NULL",
    );
}

#[test]
fn testCustomTypeValueSql() {
    dql::register_negative_to_positive_type();
    assert_sql_generation(
        r"SELECT p.customInteger FROM Doctrine\Tests\Models\CustomType\CustomTypeParent p WHERE p.id = 1",
        "SELECT -(c0_.customInteger) AS customInteger_0 FROM customtype_parents c0_ WHERE c0_.id = 1",
    );
}

#[test]
fn testCustomTypeValueSqlIgnoresIdentifierColumn() {
    dql::register_negative_to_positive_type();
    assert_sql_generation(
        r"SELECT p.id FROM Doctrine\Tests\Models\CustomType\CustomTypeParent p WHERE p.id = 1",
        "SELECT c0_.id AS id_0 FROM customtype_parents c0_ WHERE c0_.id = 1",
    );
}

#[test]
fn testCustomTypeValueSqlForAllFields() {
    dql::register_negative_to_positive_type();
    assert_sql_generation(
        r"SELECT p FROM Doctrine\Tests\Models\CustomType\CustomTypeParent p",
        "SELECT c0_.id AS id_0, -(c0_.customInteger) AS customInteger_1, c0_.child_id AS child_id_2 FROM customtype_parents c0_",
    );
}

#[test]
fn testCustomTypeValueSqlForPartialObject() {
    dql::register_negative_to_positive_type();
    assert_sql_generation(
        r"SELECT partial p.{id, customInteger} FROM Doctrine\Tests\Models\CustomType\CustomTypeParent p",
        "SELECT c0_.id AS id_0, -(c0_.customInteger) AS customInteger_1, c0_.child_id AS child_id_2 FROM customtype_parents c0_",
    );
}

#[test]
fn testMultipleFromAndInheritanceCondition() {
    assert_sql_generation(
        r"SELECT fix, flex FROM Doctrine\Tests\Models\Company\CompanyFixContract fix, Doctrine\Tests\Models\Company\CompanyFlexContract flex",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.fixPrice AS fixPrice_2, c1_.id AS id_3, c1_.completed AS completed_4, c1_.hoursWorked AS hoursWorked_5, c1_.pricePerHour AS pricePerHour_6, c1_.maxPrice AS maxPrice_7, c0_.discr AS discr_8, c0_.salesPerson_id AS salesPerson_id_9, c1_.discr AS discr_10, c1_.salesPerson_id AS salesPerson_id_11 FROM company_contracts c0_, company_contracts c1_ WHERE (c0_.discr IN ('fix') AND c1_.discr IN ('flexible', 'flexultra'))",
    );
}

#[test]
fn testOrderByClauseSupportsSimpleArithmeticExpression() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.id + 1 ",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ ORDER BY c0_.id + 1 ASC",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY ( ( (u.id + 1) * (u.id - 1) ) / 2)",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ ORDER BY (((c0_.id + 1) * (c0_.id - 1)) / 2) ASC",
    );
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY ((u.id + 5000) * u.id + 3) ",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ ORDER BY ((c0_.id + 5000) * c0_.id + 3) ASC",
    );
}

#[test]
fn testOrderByClauseSupportsFunction() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY CONCAT(u.username, u.name) ",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ ORDER BY c0_.username || c0_.name ASC",
    );
}

#[test]
fn testStripNonAlphanumericCharactersFromAlias() {
    assert_sql_generation(
        r"SELECT e FROM Doctrine\Tests\Models\Generic\NonAlphaColumnsEntity e",
        r#"SELECT n0_."simple-entity-id" AS simpleentityid_0, n0_."simple-entity-value" AS simpleentityvalue_1 FROM "not-a-simple-entity" n0_"#,
    );
    assert_sql_generation(
        r"SELECT e.value FROM Doctrine\Tests\Models\Generic\NonAlphaColumnsEntity e ORDER BY e.value",
        r#"SELECT n0_."simple-entity-value" AS simpleentityvalue_0 FROM "not-a-simple-entity" n0_ ORDER BY n0_."simple-entity-value" ASC"#,
    );
    assert_sql_generation(
        r"SELECT TRIM(e.value) FROM Doctrine\Tests\Models\Generic\NonAlphaColumnsEntity e ORDER BY e.value",
        r#"SELECT TRIM(n0_."simple-entity-value") AS sclr_0 FROM "not-a-simple-entity" n0_ ORDER BY n0_."simple-entity-value" ASC"#,
    );
}

#[test]
fn testColumnNameWithNumbersAndNonAlphanumericCharacters() {
    assert_sql_generation(
        r"SELECT e FROM Doctrine\Tests\Models\Quote\NumericEntity e",
        r#"SELECT t0_."1:1" AS 11_0, t0_."2:2" AS 22_1 FROM table t0_"#,
    );
    assert_sql_generation(
        r"SELECT e.value FROM Doctrine\Tests\Models\Quote\NumericEntity e",
        r#"SELECT t0_."2:2" AS 22_0 FROM table t0_"#,
    );
    assert_sql_generation(
        r"SELECT TRIM(e.value) FROM Doctrine\Tests\Models\Quote\NumericEntity e",
        r#"SELECT TRIM(t0_."2:2") AS sclr_0 FROM table t0_"#,
    );
}

#[test]
fn testQuotedTableDeclaration() {
    assert_sql_generation(
        r"SELECT u FROM Doctrine\Tests\Models\Quote\User u",
        r#"SELECT q0_."user-id" AS userid_0, q0_."user-name" AS username_1 FROM "quote-user" q0_"#,
    );
}

#[test]
fn testQuotedWalkJoinVariableDeclaration() {
    assert_sql_generation(
        r"SELECT u, a FROM Doctrine\Tests\Models\Quote\User u JOIN u.address a",
        r#"SELECT q0_."user-id" AS userid_0, q0_."user-name" AS username_1, q1_."address-id" AS addressid_2, q1_."address-zip" AS addresszip_3, q1_.type AS type_4, q1_."user-id" AS userid_5, q1_."city-id" AS cityid_6 FROM "quote-user" q0_ INNER JOIN "quote-address" q1_ ON q0_."user-id" = q1_."user-id" AND q1_.type IN ('simple', 'full')"#,
    );
    assert_sql_generation(
        r"SELECT u, p FROM Doctrine\Tests\Models\Quote\User u JOIN u.phones p",
        r#"SELECT q0_."user-id" AS userid_0, q0_."user-name" AS username_1, q1_."phone-number" AS phonenumber_2, q1_."user-id" AS userid_3 FROM "quote-user" q0_ INNER JOIN "quote-phone" q1_ ON q0_."user-id" = q1_."user-id""#,
    );
    assert_sql_generation(
        r"SELECT u, g FROM Doctrine\Tests\Models\Quote\User u JOIN u.groups g",
        r#"SELECT q0_."user-id" AS userid_0, q0_."user-name" AS username_1, q1_."group-id" AS groupid_2, q1_."group-name" AS groupname_3, q1_."parent-id" AS parentid_4 FROM "quote-user" q0_ INNER JOIN "quote-users-groups" q2_ ON q0_."user-id" = q2_."user-id" INNER JOIN "quote-group" q1_ ON q1_."group-id" = q2_."group-id""#,
    );
    assert_sql_generation(
        r"SELECT a, u FROM Doctrine\Tests\Models\Quote\Address a JOIN a.user u",
        r#"SELECT q0_."address-id" AS addressid_0, q0_."address-zip" AS addresszip_1, q1_."user-id" AS userid_2, q1_."user-name" AS username_3, q0_.type AS type_4, q0_."user-id" AS userid_5, q0_."city-id" AS cityid_6 FROM "quote-address" q0_ INNER JOIN "quote-user" q1_ ON q0_."user-id" = q1_."user-id" WHERE q0_.type IN ('simple', 'full')"#,
    );
    assert_sql_generation(
        r"SELECT g, u FROM Doctrine\Tests\Models\Quote\Group g JOIN g.users u",
        r#"SELECT q0_."group-id" AS groupid_0, q0_."group-name" AS groupname_1, q1_."user-id" AS userid_2, q1_."user-name" AS username_3, q0_."parent-id" AS parentid_4 FROM "quote-group" q0_ INNER JOIN "quote-users-groups" q2_ ON q0_."group-id" = q2_."group-id" INNER JOIN "quote-user" q1_ ON q1_."user-id" = q2_."user-id""#,
    );
    assert_sql_generation(
        r"SELECT g, p FROM Doctrine\Tests\Models\Quote\Group g JOIN g.parent p",
        r#"SELECT q0_."group-id" AS groupid_0, q0_."group-name" AS groupname_1, q1_."group-id" AS groupid_2, q1_."group-name" AS groupname_3, q0_."parent-id" AS parentid_4, q1_."parent-id" AS parentid_5 FROM "quote-group" q0_ INNER JOIN "quote-group" q1_ ON q0_."parent-id" = q1_."group-id""#,
    );
}

#[test]
fn testCaseThenParameterArithmeticExpression() {
    assert_sql_generation(
        r"SELECT SUM(CASE WHEN e.salary <= :value THEN e.salary - :value WHEN e.salary >= :value THEN :value - e.salary ELSE 0 END) FROM Doctrine\Tests\Models\Company\CompanyEmployee e",
        "SELECT SUM(CASE WHEN c0_.salary <= ? THEN c0_.salary - ? WHEN c0_.salary >= ? THEN ? - c0_.salary ELSE 0 END) AS sclr_0 FROM company_employees c0_ INNER JOIN company_persons c1_ ON c0_.id = c1_.id LEFT JOIN company_managers c2_ ON c0_.id = c2_.id",
    );
    assert_sql_generation(
        r"SELECT SUM(CASE WHEN e.salary <= :value THEN e.salary - :value WHEN e.salary >= :value THEN :value - e.salary ELSE e.salary + 0 END) FROM Doctrine\Tests\Models\Company\CompanyEmployee e",
        "SELECT SUM(CASE WHEN c0_.salary <= ? THEN c0_.salary - ? WHEN c0_.salary >= ? THEN ? - c0_.salary ELSE c0_.salary + 0 END) AS sclr_0 FROM company_employees c0_ INNER JOIN company_persons c1_ ON c0_.id = c1_.id LEFT JOIN company_managers c2_ ON c0_.id = c2_.id",
    );
    assert_sql_generation(
        r"SELECT SUM(CASE WHEN e.salary <= :value THEN (e.salary - :value) WHEN e.salary >= :value THEN (:value - e.salary) ELSE (e.salary + :value) END) FROM Doctrine\Tests\Models\Company\CompanyEmployee e",
        "SELECT SUM(CASE WHEN c0_.salary <= ? THEN (c0_.salary - ?) WHEN c0_.salary >= ? THEN (? - c0_.salary) ELSE (c0_.salary + ?) END) AS sclr_0 FROM company_employees c0_ INNER JOIN company_persons c1_ ON c0_.id = c1_.id LEFT JOIN company_managers c2_ ON c0_.id = c2_.id",
    );
}

#[test]
fn testArithmeticPriority() {
    assert_sql_generation(
        r"SELECT 100/(2*2) FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT 100 / (2 * 2) AS sclr_0 FROM cms_users c0_",
    );
    assert_sql_generation(
        r"SELECT (u.id / (u.id * 2)) FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT (c0_.id / (c0_.id * 2)) AS sclr_0 FROM cms_users c0_",
    );
    assert_sql_generation(
        r"SELECT 100/(2*2) + (u.id / (u.id * 2)) FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE (u.id / (u.id * 2)) > 0",
        "SELECT 100 / (2 * 2) + (c0_.id / (c0_.id * 2)) AS sclr_0 FROM cms_users c0_ WHERE (c0_.id / (c0_.id * 2)) > 0",
    );
}

#[test]
fn testOrderByClauseShouldReplaceOrderByRelationMapping() {
    assert_sql_generation(
        r"SELECT r, b FROM Doctrine\Tests\Models\Routing\RoutingRoute r JOIN r.bookings b",
        "SELECT r0_.id AS id_0, r1_.id AS id_1, r1_.passengerName AS passengerName_2, r1_.route_id AS route_id_3 FROM RoutingRoute r0_ INNER JOIN RoutingRouteBooking r1_ ON r0_.id = r1_.route_id ORDER BY r1_.passengerName ASC",
    );
    assert_sql_generation(
        r"SELECT r, b FROM Doctrine\Tests\Models\Routing\RoutingRoute r JOIN r.bookings b ORDER BY b.passengerName DESC",
        "SELECT r0_.id AS id_0, r1_.id AS id_1, r1_.passengerName AS passengerName_2, r1_.route_id AS route_id_3 FROM RoutingRoute r0_ INNER JOIN RoutingRouteBooking r1_ ON r0_.id = r1_.route_id ORDER BY r1_.passengerName DESC",
    );
}

#[test]
fn testHavingSupportIsNullExpression() {
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u HAVING u.username IS NULL",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ HAVING c0_.username IS NULL",
    );
    assert_sql_generation(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u HAVING MAX(u.name) IS NULL",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ HAVING MAX(c0_.name) IS NULL",
    );
}

#[test]
fn testClassTableInheritanceJoinWithConditionAppliesToBaseTable() {
    assert_sql_generation(
        r"SELECT e.id FROM Doctrine\Tests\Models\Company\CompanyOrganization o JOIN o.events e WITH e.id = ?1",
        "SELECT c0_.id AS id_0 FROM company_organizations c1_ INNER JOIN (company_events c0_ LEFT JOIN company_auctions c2_ ON c0_.id = c2_.id LEFT JOIN company_raffles c3_ ON c0_.id = c3_.id) ON c1_.id = c0_.org_id AND (c0_.id = ?)",
    );
}

#[test]
fn testSingleTableInheritanceLeftJoinWithCondition() {
    assert_sql_generation(
        r"SELECT c FROM Doctrine\Tests\Models\Company\CompanyEmployee e LEFT JOIN Doctrine\Tests\Models\Company\CompanyContract c ON c.salesPerson = e.id",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.fixPrice AS fixPrice_2, c0_.hoursWorked AS hoursWorked_3, c0_.pricePerHour AS pricePerHour_4, c0_.maxPrice AS maxPrice_5, c0_.discr AS discr_6, c0_.salesPerson_id AS salesPerson_id_7 FROM company_employees c1_ INNER JOIN company_persons c2_ ON c1_.id = c2_.id LEFT JOIN company_managers c3_ ON c1_.id = c3_.id LEFT JOIN company_contracts c0_ ON (c0_.salesPerson_id = c2_.id) AND c0_.discr IN ('fix', 'flexible', 'flexultra')",
    );
}

#[test]
fn testSingleTableInheritanceLeftJoinWithConditionAndWhere() {
    assert_sql_generation(
        r"SELECT c FROM Doctrine\Tests\Models\Company\CompanyEmployee e LEFT JOIN Doctrine\Tests\Models\Company\CompanyContract c ON c.salesPerson = e.id WHERE e.salary > 1000",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.fixPrice AS fixPrice_2, c0_.hoursWorked AS hoursWorked_3, c0_.pricePerHour AS pricePerHour_4, c0_.maxPrice AS maxPrice_5, c0_.discr AS discr_6, c0_.salesPerson_id AS salesPerson_id_7 FROM company_employees c1_ INNER JOIN company_persons c2_ ON c1_.id = c2_.id LEFT JOIN company_managers c3_ ON c1_.id = c3_.id LEFT JOIN company_contracts c0_ ON (c0_.salesPerson_id = c2_.id) AND c0_.discr IN ('fix', 'flexible', 'flexultra') WHERE c1_.salary > 1000",
    );
}

#[test]
fn testSingleTableInheritanceInnerJoinWithCondition() {
    assert_sql_generation(
        r"SELECT c FROM Doctrine\Tests\Models\Company\CompanyEmployee e INNER JOIN Doctrine\Tests\Models\Company\CompanyContract c ON c.salesPerson = e.id",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.fixPrice AS fixPrice_2, c0_.hoursWorked AS hoursWorked_3, c0_.pricePerHour AS pricePerHour_4, c0_.maxPrice AS maxPrice_5, c0_.discr AS discr_6, c0_.salesPerson_id AS salesPerson_id_7 FROM company_employees c1_ INNER JOIN company_persons c2_ ON c1_.id = c2_.id LEFT JOIN company_managers c3_ ON c1_.id = c3_.id INNER JOIN company_contracts c0_ ON (c0_.salesPerson_id = c2_.id) AND c0_.discr IN ('fix', 'flexible', 'flexultra')",
    );
}

#[test]
fn testSingleTableInheritanceLeftJoinNonAssociationWithConditionAndWhere() {
    assert_sql_generation(
        r"SELECT c FROM Doctrine\Tests\Models\Company\CompanyContract c LEFT JOIN Doctrine\Tests\Models\Company\CompanyEmployee e ON e.id = c.salesPerson WHERE c.completed = true",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.fixPrice AS fixPrice_2, c0_.hoursWorked AS hoursWorked_3, c0_.pricePerHour AS pricePerHour_4, c0_.maxPrice AS maxPrice_5, c0_.discr AS discr_6, c0_.salesPerson_id AS salesPerson_id_7 FROM company_contracts c0_ LEFT JOIN (company_employees c1_ INNER JOIN company_persons c2_ ON c1_.id = c2_.id LEFT JOIN company_managers c3_ ON c1_.id = c3_.id) ON (c2_.id = c0_.salesPerson_id) WHERE (c0_.completed = 1) AND c0_.discr IN ('fix', 'flexible', 'flexultra')",
    );
}

#[test]
fn testSingleTableInheritanceJoinCreatesOnCondition() {
    assert_sql_generation(
        r"SELECT c FROM Doctrine\Tests\Models\Company\CompanyContract c JOIN c.salesPerson s WHERE c.completed = true",
        "SELECT c0_.id AS id_0, c0_.completed AS completed_1, c0_.fixPrice AS fixPrice_2, c0_.hoursWorked AS hoursWorked_3, c0_.pricePerHour AS pricePerHour_4, c0_.maxPrice AS maxPrice_5, c0_.discr AS discr_6, c0_.salesPerson_id AS salesPerson_id_7 FROM company_contracts c0_ INNER JOIN company_employees c1_ ON c0_.salesPerson_id = c1_.id LEFT JOIN company_persons c2_ ON c1_.id = c2_.id LEFT JOIN company_managers c3_ ON c1_.id = c3_.id WHERE (c0_.completed = 1) AND c0_.discr IN ('fix', 'flexible', 'flexultra')",
    );
}

#[test]
fn testSingleTableInheritanceCreatesOnConditionAndWhere() {
    assert_sql_generation_with(
        &TestEntityManager::new(),
        r"SELECT e, COUNT(c) FROM Doctrine\Tests\Models\Company\CompanyEmployee e JOIN e.contracts c WHERE e.department = :department",
        "SELECT c0_.id AS id_0, c0_.name AS name_1, c1_.salary AS salary_2, c1_.department AS department_3, c1_.startDate AS startDate_4, c2_.title AS title_5, COUNT(c3_.id) AS sclr_6, c0_.discr AS discr_7, c0_.spouse_id AS spouse_id_8, c2_.car_id AS car_id_9 FROM company_employees c1_ INNER JOIN company_persons c0_ ON c1_.id = c0_.id LEFT JOIN company_managers c2_ ON c1_.id = c2_.id INNER JOIN company_contract_employees c4_ ON c1_.id = c4_.employee_id INNER JOIN company_contracts c3_ ON c3_.id = c4_.contract_id AND c3_.discr IN ('fix', 'flexible', 'flexultra') WHERE c1_.department = ?",
        &[],
        &[("department", Value::from("foobar"))],
        HYDRATE_OBJECT,
    );
}

#[test]
fn testHavingSupportResultVariableInExpression() {
    assert_sql_generation(
        r"SELECT u.name AS foo FROM Doctrine\Tests\Models\CMS\CmsUser u HAVING foo IN (?1)",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ HAVING name_0 IN (?)",
    );
}

#[test]
fn testHavingSupportResultVariableLikeExpression() {
    assert_sql_generation(
        r"SELECT u.name AS foo FROM Doctrine\Tests\Models\CMS\CmsUser u HAVING foo LIKE '3'",
        "SELECT c0_.name AS name_0 FROM cms_users c0_ HAVING name_0 LIKE '3'",
    );
}

#[test]
fn testHavingSupportResultVariableNullComparisonExpression() {
    assert_sql_generation(
        r"SELECT u AS user, SUM(a.id) AS score FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN Doctrine\Tests\Models\CMS\CmsAddress a ON a.user = u GROUP BY u HAVING score IS NOT NULL AND score >= 5",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, SUM(c1_.id) AS sclr_4, c0_.email_id AS email_id_5 FROM cms_users c0_ LEFT JOIN cms_addresses c1_ ON (c1_.user_id = c0_.id) GROUP BY c0_.id, c0_.status, c0_.username, c0_.name, c0_.email_id HAVING sclr_4 IS NOT NULL AND sclr_4 >= 5",
    );
}

#[test]
fn testHavingSupportResultVariableInAggregateFunction() {
    assert_sql_generation(
        r"SELECT COUNT(u.name) AS countName FROM Doctrine\Tests\Models\CMS\CmsUser u HAVING countName IS NULL",
        "SELECT COUNT(c0_.name) AS sclr_0 FROM cms_users c0_ HAVING sclr_0 IS NULL",
    );
}

#[test]
fn testHavingRegressionUsingVariableWithMathOperatorsExpression() {
    for operator in ["+", "-", "*", "/"] {
        assert_sql_generation(
            &format!(
                "{}{operator}{}",
                r"SELECT COUNT(u.name) AS countName FROM Doctrine\Tests\Models\CMS\CmsUser u HAVING 1 ",
                " countName > 0"
            ),
            &format!(
                "{}{operator}{}",
                "SELECT COUNT(c0_.name) AS sclr_0 FROM cms_users c0_ HAVING 1 ", " sclr_0 > 0"
            ),
        );
    }
}
