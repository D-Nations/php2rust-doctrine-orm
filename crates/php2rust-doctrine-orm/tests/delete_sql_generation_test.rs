//! Port of `DeleteSqlGenerationTest`: the SQL of DQL DELETE statements
//! on the test entity manager (`tests/common/dql.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/DeleteSqlGenerationTest.php

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

use dql::TestEntityManager;

fn assert_sql_generation(dql_to_be_tested: &str, sql_to_be_confirmed: &str) {
    let entity_manager = TestEntityManager::new();
    let query = entity_manager.create_query(dql_to_be_tested);
    assert_eq!(sql_to_be_confirmed, query.get_single_sql().unwrap());
}

#[test]
fn testSupportsDeleteWithoutWhereAndAlias() {
    assert_sql_generation(
        r"DELETE FROM Doctrine\Tests\Models\CMS\CmsUser",
        "DELETE FROM cms_users",
    );
}

#[test]
fn testSupportsDeleteWithoutWhereAndFrom() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u",
        "DELETE FROM cms_users",
    );
}

#[test]
fn testSupportsDeleteWithoutWhere() {
    assert_sql_generation(
        r"DELETE FROM Doctrine\Tests\Models\CMS\CmsUser u",
        "DELETE FROM cms_users",
    );
}

#[test]
fn testSupportsWhereClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = ?1",
        "DELETE FROM cms_users WHERE id = ?",
    );
}

#[test]
fn testSupportsWhereOrExpressions() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.username = ?1 OR u.name = ?2",
        "DELETE FROM cms_users WHERE username = ? OR name = ?",
    );
}

#[test]
fn testSupportsWhereNestedConditionalExpressions() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id = ?1 OR ( u.username = ?2 OR u.name = ?3)",
        "DELETE FROM cms_users WHERE id = ? OR (username = ? OR name = ?)",
    );
}

#[test]
fn testIsCaseAgnostic() {
    assert_sql_generation(
        r"delete from Doctrine\Tests\Models\CMS\CmsUser u where u.username = ?1",
        "DELETE FROM cms_users WHERE username = ?",
    );
}

#[test]
fn testSupportsAndCondition() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.username = ?1 AND u.name = ?2",
        "DELETE FROM cms_users WHERE username = ? AND name = ?",
    );
}

#[test]
fn testSupportsWhereNot() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE NOT u.id != ?1",
        "DELETE FROM cms_users WHERE NOT id <> ?",
    );
}

#[test]
fn testSupportsWhereNotWithParentheses() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE NOT ( u.id != ?1 )",
        "DELETE FROM cms_users WHERE NOT (id <> ?)",
    );
}

#[test]
fn testSupportsWhereNotWithAndExpression() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE NOT ( u.id != ?1 AND u.username = ?2 )",
        "DELETE FROM cms_users WHERE NOT (id <> ? AND username = ?)",
    );
}

#[test]
fn testSupportsGreaterThanComparisonClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id > ?1",
        "DELETE FROM cms_users WHERE id > ?",
    );
}

#[test]
fn testSupportsGreaterThanOrEqualToComparisonClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id >= ?1",
        "DELETE FROM cms_users WHERE id >= ?",
    );
}

#[test]
fn testSupportsLessThanComparisonClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id < ?1",
        "DELETE FROM cms_users WHERE id < ?",
    );
}

#[test]
fn testSupportsLessThanOrEqualToComparisonClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id <= ?1",
        "DELETE FROM cms_users WHERE id <= ?",
    );
}

#[test]
fn testSupportsNotEqualToComparisonClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id <> ?1",
        "DELETE FROM cms_users WHERE id <> ?",
    );
}

#[test]
fn testSupportsNotEqualToComparisonClauseExpressedWithExclamationMark() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id != ?1",
        "DELETE FROM cms_users WHERE id <> ?",
    );
}

#[test]
fn testSupportsNotBetweenClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id NOT BETWEEN ?1 AND ?2",
        "DELETE FROM cms_users WHERE id NOT BETWEEN ? AND ?",
    );
}

#[test]
fn testSupportsBetweenClauseUsedWithAndClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id BETWEEN ?1 AND ?2 AND u.username != ?3",
        "DELETE FROM cms_users WHERE id BETWEEN ? AND ? AND username <> ?",
    );
}

#[test]
fn testSupportsNotLikeClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.username NOT LIKE ?1",
        "DELETE FROM cms_users WHERE username NOT LIKE ?",
    );
}

#[test]
fn testSupportsLikeClauseWithEscapeExpression() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.username LIKE ?1 ESCAPE '\'",
        r"DELETE FROM cms_users WHERE username LIKE ? ESCAPE '\'",
    );
}

#[test]
fn testSupportsIsNullClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name IS NULL",
        "DELETE FROM cms_users WHERE name IS NULL",
    );
}

#[test]
fn testSupportsIsNotNullClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.name IS NOT NULL",
        "DELETE FROM cms_users WHERE name IS NOT NULL",
    );
}

#[test]
fn testSupportsAtomExpressionAsClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE 1 = 1",
        "DELETE FROM cms_users WHERE 1 = 1",
    );
}

#[test]
fn testSupportsParameterizedAtomExpression() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE ?1 = 1",
        "DELETE FROM cms_users WHERE ? = 1",
    );
}

#[test]
fn testSupportsInClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id IN ( ?1, ?2, ?3, ?4 )",
        "DELETE FROM cms_users WHERE id IN (?, ?, ?, ?)",
    );
}

#[test]
fn testSupportsNotInClause() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE u.id NOT IN ( ?1, ?2 )",
        "DELETE FROM cms_users WHERE id NOT IN (?, ?)",
    );
}

#[test]
fn testSubselectTableAliasReferencing() {
    assert_sql_generation(
        r"DELETE Doctrine\Tests\Models\CMS\CmsUser u WHERE SIZE(u.groups) = 10",
        "DELETE FROM cms_users WHERE (SELECT COUNT(*) FROM cms_users_groups c0_ WHERE c0_.user_id = cms_users.id) = 10",
    );
}
