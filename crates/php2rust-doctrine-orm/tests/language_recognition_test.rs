//! Port of `LanguageRecognitionTest`: the parser accepts or rejects DQL,
//! with the `NullSqlWalker` output walker (`tests/common/dql.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/LanguageRecognitionTest.php

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

use php_value::Thrown;
use php2rust_doctrine_orm::query::ParserResult;
use php2rust_doctrine_orm::query::context::{HYDRATE_OBJECT, Hint};
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use php2rust_doctrine_orm::query::ast::functions::ConcatFunction;

use dql::{ExpectDeprecation, NullSqlWalker, TestEntityManager, output_walker, thrown};

const QUERY_EXCEPTION: &str = "Doctrine\\ORM\\Query\\QueryException";

fn parse_dql_with(
    entity_manager: &TestEntityManager,
    dql: &str,
    hints: &[(&str, Hint)],
) -> Result<Rc<RefCell<ParserResult>>, Thrown> {
    let query = entity_manager.create_query(dql);
    query.set_dql(dql);
    query.set_hydration_mode(HYDRATE_OBJECT);
    for (key, value) in hints {
        query.hint(key, value.clone());
    }
    // We do NOT test SQL output here. That only unnecessarily slows down the tests!
    let (name, walker) = output_walker(NullSqlWalker::CLASS);
    query.hint(name, walker);
    query.parse()
}

fn parse_dql(dql: &str) -> Result<Rc<RefCell<ParserResult>>, Thrown> {
    parse_dql_with(&TestEntityManager::new(), dql, &[])
}

fn assert_valid_dql(dql: &str) {
    if let Err(error) = parse_dql(dql) {
        panic!("{dql}: {:?}", thrown(&error));
    }
}

fn assert_invalid_dql(dql: &str) {
    match parse_dql(dql) {
        Ok(_) => panic!("{dql} was accepted"),
        Err(error) => assert!(thrown_is_a(&error, QUERY_EXCEPTION), "{:?}", thrown(&error)),
    }
}

#[test]
fn testEmptyQueryString() {
    assert_invalid_dql("");
}

#[test]
fn testPlainFromClauseWithAlias() {
    assert_valid_dql(r"SELECT u.id FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testSelectSingleComponentWithAsterisk() {
    assert_valid_dql(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testRejectsInvalidDQL() {
    // `setEntityNamespaces()` is left out: the parser does not read it.
    for dql in [
        r"SELECT 'foo' AS foo\bar FROM Doctrine\Tests\Models\CMS\CmsUser u",
        r"SELECT \foo FROM Doctrine\Tests\Models\CMS\CmsUser \foo",
        r"SELECT foo\ FROM Doctrine\Tests\Models\CMS\CmsUser foo\",
        r"SELECT foo\bar FROM Doctrine\Tests\Models\CMS\CmsUser foo\bar",
        "SELECT u FROM UnknownClass u",
        r"SELECT u FROM Unknown\Class u",
        r"SELECT u FROM \Unknown\Class u",
        r"SELECT u FROM Unknown\\Class u",
        r"SELECT u FROM Unknown\Class\ u",
        r"SELECT u FROM Doctrine\Tests\Models\CMS\\CmsUser u",
        r"SELECT 'foo' AS \foo FROM Doctrine\Tests\Models\CMS\CmsUser u",
        r"SELECT 'foo' AS \foo\bar FROM Doctrine\Tests\Models\CMS\CmsUser u",
        r"SELECT 'foo' AS foo\ FROM Doctrine\Tests\Models\CMS\CmsUser u",
        r"SELECT 'foo' AS foo\\bar FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "0",
    ] {
        assert_invalid_dql(dql);
    }
}

#[test]
fn testSelectSingleComponentWithMultipleColumns() {
    assert_valid_dql(r"SELECT u.name, u.username FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testSelectMultipleComponentsUsingMultipleFrom() {
    assert_valid_dql(
        r"SELECT u, p FROM Doctrine\Tests\Models\CMS\CmsUser u, Doctrine\Tests\Models\CMS\CmsPhonenumber p WHERE u = p.user",
    );
}

#[test]
fn testSelectMultipleComponentsWithAsterisk() {
    assert_valid_dql(r"SELECT u, p FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.phonenumbers p");
}

#[test]
fn testSelectDistinctIsSupported() {
    assert_valid_dql(r"SELECT DISTINCT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testAggregateFunctionInSelect() {
    assert_valid_dql(r"SELECT COUNT(u.id) FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testMultipleParenthesisInSelect() {
    assert_valid_dql(r"SELECT (((u.id))) as v FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testDuplicatedAliasInAggregateFunction() {
    assert_invalid_dql(
        r"SELECT COUNT(u.id) AS num, SUM(u.id) AS num FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
}

#[test]
fn testAggregateFunctionWithDistinctInSelect() {
    assert_valid_dql(r"SELECT COUNT(DISTINCT u.name) FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testFunctionalExpressionsSupportedInWherePart() {
    assert_valid_dql(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE TRIM(u.name) = 'someone'",
    );
}

#[test]
fn testTrimFalsyString() {
    assert_valid_dql(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE TRIM('0' FROM u.name) = 'someone'",
    );
}

#[test]
fn testArithmeticExpressionsSupportedInWherePart() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE ((u.id + 5000) * u.id + 3) < 10000000",
    );
}

#[test]
fn testInExpressionSupportedInWherePart() {
    assert_valid_dql(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id IN (1, 2)");
}

#[test]
fn testInExpressionWithoutSpacesSupportedInWherePart() {
    assert_valid_dql(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id IN (1,2,3)");
}

#[test]
fn testNotInExpressionSupportedInWherePart() {
    assert_valid_dql(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id NOT IN (1)");
}

#[test]
fn testInExpressionWithSingleValuedAssociationPathExpression() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\Forum\ForumUser u WHERE u.avatar IN (?1, ?2)",
    );
}

#[test]
fn testInvalidInExpressionWithCollectionValuedAssociationPathExpression() {
    assert_invalid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.phonenumbers IN (?1, ?2)",
    );
}

#[test]
fn testInstanceOfExpressionSupportedInWherePart() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\Company\CompanyPerson u WHERE u INSTANCE OF Doctrine\Tests\Models\Company\CompanyEmployee",
    );
}

#[test]
fn testInstanceOfExpressionWithInputParamSupportedInWherePart() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\Company\CompanyPerson u WHERE u INSTANCE OF ?1",
    );
}

#[test]
fn testNotInstanceOfExpressionSupportedInWherePart() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\Company\CompanyPerson u WHERE u NOT INSTANCE OF ?1",
    );
}

#[test]
fn testExistsExpressionSupportedInWherePart() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE EXISTS (SELECT p.phonenumber FROM Doctrine\Tests\Models\CMS\CmsPhonenumber p WHERE p.phonenumber = 1234)",
    );
}

#[test]
fn testNotExistsExpressionSupportedInWherePart() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE NOT EXISTS (SELECT p.phonenumber FROM Doctrine\Tests\Models\CMS\CmsPhonenumber p WHERE p.phonenumber = 1234)",
    );
}

#[test]
fn testAggregateFunctionInHavingClause() {
    assert_valid_dql(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.phonenumbers p HAVING COUNT(p.phonenumber) > 2",
    );
    assert_valid_dql(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.phonenumbers p HAVING MAX(u.name) = 'romanb'",
    );
}

#[test]
fn testLeftJoin() {
    assert_valid_dql(
        r"SELECT u, p FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.phonenumbers p",
    );
}

#[test]
fn testJoin() {
    assert_valid_dql(r"SELECT u,p FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.phonenumbers p");
}

#[test]
fn testInnerJoin() {
    assert_valid_dql(
        r"SELECT u, p FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.phonenumbers p",
    );
}

#[test]
fn testMultipleLeftJoin() {
    assert_valid_dql(
        r"SELECT u, a, p FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.articles a LEFT JOIN u.phonenumbers p",
    );
}

#[test]
fn testMultipleInnerJoin() {
    assert_valid_dql(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.articles a INNER JOIN u.phonenumbers p",
    );
}

#[test]
fn testMixingOfJoins() {
    assert_valid_dql(
        r"SELECT u.name, a.topic, p.phonenumber FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.articles a LEFT JOIN u.phonenumbers p",
    );
}

#[test]
fn testJoinClassPathUsingON() {
    assert_valid_dql(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN Doctrine\Tests\Models\CMS\CmsArticle a ON a.user = u.id",
    );
}

#[test]
fn testJoinClassPathUsingWITH() {
    let _deprecation = ExpectDeprecation::new("https://github.com/doctrine/orm/issues/12192");
    assert_valid_dql(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN Doctrine\Tests\Models\CMS\CmsArticle a WITH a.user = u.id",
    );
}

#[test]
fn testJoinClassPathUsingWHERE() {
    assert_valid_dql(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN Doctrine\Tests\Models\CMS\CmsArticle a WHERE a.user = u.id",
    );
}

#[test]
fn testDDC3701WHEREIsNotWITH() {
    assert_invalid_dql(
        r"SELECT c FROM Doctrine\Tests\Models\Company\CompanyContract c JOIN Doctrine\Tests\Models\Company\CompanyEmployee e WHERE e.id = c.salesPerson WHERE c.completed = true",
    );
}

#[test]
fn testOrderBySingleColumn() {
    assert_valid_dql(r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.name");
}

#[test]
fn testOrderBySingleColumnAscending() {
    assert_valid_dql(r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.name ASC");
}

#[test]
fn testOrderBySingleColumnDescending() {
    assert_valid_dql(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.name DESC",
    );
}

#[test]
fn testOrderByMultipleColumns() {
    assert_valid_dql(
        r"SELECT u.name, u.username FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.username DESC, u.name DESC",
    );
}

#[test]
fn testSubselectInInExpression() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id NOT IN (SELECT u2.id FROM Doctrine\Tests\Models\CMS\CmsUser u2 WHERE u2.name = 'zYne')",
    );
}

#[test]
fn testSubselectInSelectPart() {
    assert_valid_dql(
        r"SELECT u.name, (SELECT COUNT(p.phonenumber) FROM Doctrine\Tests\Models\CMS\CmsPhonenumber p WHERE p.phonenumber = 1234) pcount FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name = 'jon'",
    );
}

#[test]
fn testArithmeticExpressionInSelectPart() {
    assert_valid_dql(r"SELECT SUM(u.id) / COUNT(u.id) FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testArithmeticExpressionInSubselectPart() {
    assert_valid_dql(
        r"SELECT (SELECT SUM(u.id) / COUNT(u.id) FROM Doctrine\Tests\Models\CMS\CmsUser u2) value FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name = 'jon'",
    );
}

#[test]
fn testArithmeticExpressionWithParenthesisInSubselectPart() {
    assert_valid_dql(
        r"SELECT (SELECT (SUM(u.id) / COUNT(u.id)) FROM Doctrine\Tests\Models\CMS\CmsUser u2) value FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name = 'jon'",
    );
}

#[test]
fn testSelectLiteralInSubselect() {
    assert_valid_dql(
        r"SELECT (SELECT 1 FROM Doctrine\Tests\Models\CMS\CmsUser u2) value FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
    assert_valid_dql(
        r"SELECT (SELECT 0 FROM Doctrine\Tests\Models\CMS\CmsUser u2) value FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
}

#[test]
fn testConstantValueInSelect() {
    assert_valid_dql(r"SELECT u.name, 'foo' AS bar FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testDuplicateAliasInSubselectPart() {
    assert_invalid_dql(
        r"SELECT (SELECT SUM(u.id) / COUNT(u.id) AS foo FROM Doctrine\Tests\Models\CMS\CmsUser u2) foo FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name = 'jon'",
    );
}

#[test]
fn testPositionalInputParameter() {
    assert_valid_dql(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = ?1");
}

#[test]
fn testNamedInputParameter() {
    assert_valid_dql(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = :id");
}

#[test]
fn testJoinConditionOverrideNotSupported() {
    assert_invalid_dql(
        r"SELECT u.name, p FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.phonenumbers p ON p.phonenumber = '123 123'",
    );
}

#[test]
fn testIndexByClauseWithOneComponent() {
    assert_valid_dql(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u INDEX BY u.id");
}

#[test]
fn testIndexBySupportsJoins() {
    assert_valid_dql(
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN u.articles a INDEX BY a.id",
    );
}

#[test]
fn testIndexBySupportsJoins2() {
    assert_valid_dql(
        r"SELECT u, p FROM Doctrine\Tests\Models\CMS\CmsUser u INDEX BY u.id LEFT JOIN u.phonenumbers p INDEX BY p.phonenumber",
    );
}

#[test]
fn testBetweenExpressionSupported() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name BETWEEN 'jepso' AND 'zYne'",
    );
}

#[test]
fn testNotBetweenExpressionSupported() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name NOT BETWEEN 'jepso' AND 'zYne'",
    );
}

#[test]
fn testLikeExpression() {
    assert_valid_dql(
        r"SELECT u.id FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name LIKE 'z%'",
    );
}

#[test]
fn testNotLikeExpression() {
    assert_valid_dql(
        r"SELECT u.id FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name NOT LIKE 'z%'",
    );
}

#[test]
fn testLikeExpressionWithCustomEscapeCharacter() {
    assert_valid_dql(
        r"SELECT u.id FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name LIKE 'z|%' ESCAPE '|'",
    );
}

#[test]
fn testFieldComparisonWithoutAlias() {
    assert_invalid_dql(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE id = 1");
}

#[test]
fn testDuplicatedAliasDeclaration() {
    assert_invalid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.articles u WHERE u.id = 1",
    );
}

#[test]
fn testImplicitJoinInWhereOnSingleValuedAssociationPathExpression() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\Forum\ForumUser u JOIN u.avatar a WHERE a.id = ?1",
    );
}

#[test]
fn testImplicitJoinInWhereOnCollectionValuedPathExpression() {
    assert_invalid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.articles a WHERE a.title = ?",
    );
}

#[test]
fn testInvalidSyntaxIsRejected() {
    assert_invalid_dql("FOOBAR CmsUser");
    assert_invalid_dql(r"DELETE FROM Doctrine\Tests\Models\CMS\CmsUser.articles");
    assert_invalid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.articles.comments",
    );
    assert_invalid_dql("SELECT c FROM CmsUser.articles.comments c");
}

#[test]
fn testUpdateWorksWithOneField() {
    assert_valid_dql(r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.name = 'someone'");
}

#[test]
fn testUpdateWorksWithMultipleFields() {
    assert_valid_dql(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.name = 'someone', u.username = 'some'",
    );
}

#[test]
fn testUpdateSupportsConditions() {
    assert_valid_dql(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.name = 'someone' WHERE u.id = 5",
    );
}

#[test]
fn testDeleteAll() {
    assert_valid_dql(r"DELETE FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testDeleteWithCondition() {
    assert_valid_dql(r"DELETE FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = 3");
}

#[test]
fn testImplicitJoinWithCartesianProductAndConditionInWhere() {
    assert_valid_dql(
        r"SELECT u, a FROM Doctrine\Tests\Models\CMS\CmsUser u, Doctrine\Tests\Models\CMS\CmsArticle a WHERE u.name = a.topic",
    );
}

#[test]
fn testAllExpressionWithCorrelatedSubquery() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id > ALL (SELECT u2.id FROM Doctrine\Tests\Models\CMS\CmsUser u2 WHERE u2.name = u.name)",
    );
}

#[test]
fn testCustomJoinsAndWithKeywordSupported() {
    assert_valid_dql(
        r"SELECT u, p FROM Doctrine\Tests\Models\CMS\CmsUser u INNER JOIN u.phonenumbers p WITH p.phonenumber = 123 WHERE u.id = 1",
    );
}

#[test]
fn testAnyExpressionWithCorrelatedSubquery() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id > ANY (SELECT u2.id FROM Doctrine\Tests\Models\CMS\CmsUser u2 WHERE u2.name = u.name)",
    );
}

#[test]
fn testSomeExpressionWithCorrelatedSubquery() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id > SOME (SELECT u2.id FROM Doctrine\Tests\Models\CMS\CmsUser u2 WHERE u2.name = u.name)",
    );
}

#[test]
fn testArithmeticExpressionWithoutParenthesisInWhereClause() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE SIZE(u.phonenumbers) + 1 > 10",
    );
}

#[test]
fn testMemberOfExpression() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE :param MEMBER OF u.phonenumbers",
    );
}

#[test]
fn testSizeFunction() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE SIZE(u.phonenumbers) > 1",
    );
}

#[test]
fn testEmptyCollectionComparisonExpression() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.phonenumbers IS EMPTY",
    );
}

#[test]
fn testSingleValuedAssociationFieldInWhere() {
    assert_valid_dql(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.address = ?1");
    assert_valid_dql(r"SELECT p FROM Doctrine\Tests\Models\CMS\CmsPhonenumber p WHERE p.user = ?1");
}

#[test]
fn testBooleanLiteralInWhere() {
    assert_valid_dql(
        r"SELECT b FROM Doctrine\Tests\Models\Generic\BooleanModel b WHERE b.booleanField = true",
    );
}

#[test]
fn testSubqueryInSelectExpression() {
    assert_valid_dql(
        r"select u, (select max(p.phonenumber) from Doctrine\Tests\Models\CMS\CmsPhonenumber p) maxId from Doctrine\Tests\Models\CMS\CmsUser u",
    );
}

#[test]
fn testUsageOfQComponentOutsideSubquery() {
    assert_invalid_dql(
        r"select u, (select max(p.phonenumber) from Doctrine\Tests\Models\CMS\CmsPhonenumber p) maxId from Doctrine\Tests\Models\CMS\CmsUser u WHERE p.user = ?1",
    );
}

#[test]
fn testUnknownAbstractSchemaName() {
    assert_invalid_dql("SELECT u FROM UnknownClassName u");
}

#[test]
fn testCorrectPartialObjectLoad() {
    assert_valid_dql(r"SELECT PARTIAL u.{id,name} FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testIncorrectPartialObjectLoadBecauseOfMissingIdentifier() {
    assert_invalid_dql(r"SELECT PARTIAL u.{name} FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testScalarExpressionInSelect() {
    assert_valid_dql(r"SELECT u, 42 + u.id AS someNumber FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testInputParameterInSelect() {
    assert_valid_dql(r"SELECT u, u.id + ?1 AS someNumber FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testCustomFunctionsReturningStringInStringPrimary() {
    let entity_manager = TestEntityManager::new();
    entity_manager
        .add_custom_string_function("CC", Arc::new(|name| Box::new(ConcatFunction::new(name))));
    let dql =
        r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE CC('%', u.name) LIKE '%foo%'";
    if let Err(error) = parse_dql_with(&entity_manager, dql, &[]) {
        panic!("{dql}: {:?}", thrown(&error));
    }
}

#[test]
fn testDQLKeywordInJoinIsAllowed() {
    assert_valid_dql(
        r"SELECT u FROM Doctrine\Tests\ORM\Query\DQLKeywordsModelUser u JOIN u.group g",
    );
}

#[test]
fn testDQLKeywordInConditionIsAllowed() {
    assert_valid_dql(
        r"SELECT g FROM Doctrine\Tests\ORM\Query\DQLKeywordsModelGroup g WHERE g.from=0",
    );
}

#[test]
fn testSelectOnlyNonRootEntityAlias() {
    assert_invalid_dql(r"SELECT g FROM Doctrine\Tests\Models\CMS\CmsUser u JOIN u.groups g");
}

#[test]
fn testInputParameterSingleChar() {
    assert_valid_dql(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name = :q");
}

#[test]
fn testGroupBy() {
    assert_valid_dql(
        r"SELECT g.id, count(u.id) FROM Doctrine\Tests\Models\CMS\CmsGroup g JOIN g.users u GROUP BY g.id",
    );
}

#[test]
fn testGroupByIdentificationVariable() {
    assert_valid_dql(
        r"SELECT g, count(u.id) FROM Doctrine\Tests\Models\CMS\CmsGroup g JOIN g.users u GROUP BY g",
    );
}

#[test]
fn testGroupByUnknownIdentificationVariable() {
    assert_invalid_dql(
        r"SELECT g, count(u.id) FROM Doctrine\Tests\Models\CMS\CmsGroup g JOIN g.users u GROUP BY m",
    );
}

#[test]
fn testSizeOfForeignKeyOneToManyPrimaryKeyEntity() {
    assert_valid_dql(
        r"SELECT a, t FROM Doctrine\Tests\Models\DDC117\DDC117Article a JOIN a.translations t WHERE SIZE(a.translations) > 0",
    );
}

#[test]
fn testSizeOfForeignKeyManyToManyPrimaryKeyEntity() {
    assert_valid_dql(
        r"SELECT e, t FROM Doctrine\Tests\Models\DDC117\DDC117Editor e JOIN e.reviewingTranslations t WHERE SIZE(e.reviewingTranslations) > 0",
    );
}

#[test]
fn testCaseSupportContainingNullIfExpression() {
    assert_valid_dql(
        r"SELECT u.id, NULLIF(u.name, u.name) AS shouldBeNull FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
}

#[test]
fn testCaseSupportContainingCoalesceExpression() {
    assert_valid_dql(
        r"select COALESCE(NULLIF(u.name, ''), u.username) as Display FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
}

#[test]
fn testSelectClauseSupportsNullLiteral() {
    assert_valid_dql(r"SELECT u.name, NULL FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testSelectClauseSupportsAliasedNullLiteral() {
    assert_valid_dql(r"SELECT NULL AS nil FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testSubselectSupportsNullLiteral() {
    assert_valid_dql(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id IN (SELECT NULL FROM Doctrine\Tests\Models\CMS\CmsUser u2)",
    );
}

#[test]
fn testGeneralCaseSupportsNullLiteralInThenAndElseClauses() {
    assert_valid_dql(
        r"SELECT CASE WHEN u.id > 10 THEN NULL ELSE NULL END FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
}

#[test]
fn testSimpleCaseSupportsNullLiteralInThenAndElseClauses() {
    assert_valid_dql(
        r"SELECT CASE u.name WHEN 'admin' THEN NULL ELSE NULL END FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
}

#[test]
fn testCoalesceSupportsNullLiteral() {
    assert_valid_dql(r"SELECT COALESCE(NULL, u.name) FROM Doctrine\Tests\Models\CMS\CmsUser u");
}

#[test]
fn testHavingSupportIsNullExpression() {
    assert_valid_dql(
        r"SELECT u.name FROM Doctrine\Tests\Models\CMS\CmsUser u HAVING u.username IS NULL",
    );
}

#[test]
fn testHavingSupportResultVariableInNullComparisonExpression() {
    assert_valid_dql(
        r"SELECT u AS user, SUM(a.id) AS score FROM Doctrine\Tests\Models\CMS\CmsUser u LEFT JOIN Doctrine\Tests\Models\CMS\CmsAddress a ON a.user = u GROUP BY u HAVING score IS NOT NULL AND score >= 5",
    );
}

#[test]
fn testHavingSupportLikeExpression() {
    assert_valid_dql(
        r"SELECT _u.id, count(_articles) as uuuu FROM Doctrine\Tests\Models\CMS\CmsUser _u LEFT JOIN _u.articles _articles GROUP BY _u HAVING uuuu LIKE '3'",
    );
}

#[test]
fn testNewLiteralExpression() {
    assert_valid_dql(
        r"SELECT new Doctrine\Tests\ORM\Query\DummyStruct(u.id, 'foo', 1, true) FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
}

#[test]
fn testNewLiteralWithSubselectExpression() {
    assert_valid_dql(
        r"SELECT new Doctrine\Tests\ORM\Query\DummyStruct(u.id, 'foo', (SELECT 1 FROM Doctrine\Tests\Models\CMS\CmsUser su), true) FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
}

#[test]
fn testNewNullLiteralExpression() {
    assert_valid_dql(
        r"SELECT new Doctrine\Tests\ORM\Query\DummyStruct(u.id, 'foo', null, true) FROM Doctrine\Tests\Models\CMS\CmsUser u",
    );
}

#[test]
fn testStringPrimaryAcceptsAggregateExpression() {
    assert_valid_dql(
        r"SELECT CONCAT(a.topic, MAX(a.version)) last FROM Doctrine\Tests\Models\CMS\CmsArticle a GROUP BY a",
    );
}
