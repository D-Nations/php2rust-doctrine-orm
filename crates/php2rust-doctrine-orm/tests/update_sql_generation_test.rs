//! Port of `UpdateSqlGenerationTest`: the SQL of DQL UPDATE statements
//! on the test entity manager (`tests/common/dql.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/UpdateSqlGenerationTest.php

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
    dql::register_negative_to_positive_type();
    let entity_manager = TestEntityManager::new();
    let query = entity_manager.create_query(dql_to_be_tested);
    assert_eq!(sql_to_be_confirmed, query.get_single_sql().unwrap());
}

#[test]
fn testSupportsQueriesWithoutWhere() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.name = ?1",
        "UPDATE cms_users SET name = ?",
    );
}

#[test]
fn testSupportsMultipleFieldsWithoutWhere() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.name = ?1, u.username = ?2",
        "UPDATE cms_users SET name = ?, username = ?",
    );
}

#[test]
fn testSupportsWhereClauses() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.name = ?1 WHERE u.id = ?2",
        "UPDATE cms_users SET name = ? WHERE id = ?",
    );
}

#[test]
fn testSupportsWhereClausesOnTheUpdatedField() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.name = ?1 WHERE u.name = ?2",
        "UPDATE cms_users SET name = ? WHERE name = ?",
    );
}

#[test]
fn testSupportsMultipleWhereClause() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.name = ?1 WHERE u.name = ?2 AND u.status = ?3",
        "UPDATE cms_users SET name = ? WHERE name = ? AND status = ?",
    );
}

#[test]
fn testSupportsInClause() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.name = ?1 WHERE u.id IN (1, 3, 4)",
        "UPDATE cms_users SET name = ? WHERE id IN (1, 3, 4)",
    );
}

#[test]
fn testSupportsParametrizedInClause() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.name = ?1 WHERE u.id IN (?2, ?3, ?4)",
        "UPDATE cms_users SET name = ? WHERE id IN (?, ?, ?)",
    );
}

#[test]
fn testSupportsNotInClause() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.name = ?1 WHERE u.id NOT IN (1, 3, 4)",
        "UPDATE cms_users SET name = ? WHERE id NOT IN (1, 3, 4)",
    );
}

#[test]
fn testSupportsGreaterThanClause() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.status = ?1 WHERE u.id > ?2",
        "UPDATE cms_users SET status = ? WHERE id > ?",
    );
}

#[test]
fn testSupportsGreaterThanOrEqualToClause() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.status = ?1 WHERE u.id >= ?2",
        "UPDATE cms_users SET status = ? WHERE id >= ?",
    );
}

#[test]
fn testSupportsLessThanClause() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.status = ?1 WHERE u.id < ?2",
        "UPDATE cms_users SET status = ? WHERE id < ?",
    );
}

#[test]
fn testSupportsLessThanOrEqualToClause() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.status = ?1 WHERE u.id <= ?2",
        "UPDATE cms_users SET status = ? WHERE id <= ?",
    );
}

#[test]
fn testSupportsBetweenClause() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.status = ?1 WHERE u.id BETWEEN :from AND :to",
        "UPDATE cms_users SET status = ? WHERE id BETWEEN ? AND ?",
    );
}

#[test]
fn testSingleValuedAssociationFieldInWhere() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsPhonenumber p SET p.phonenumber = 1234 WHERE p.user = ?1",
        "UPDATE cms_phonenumbers SET phonenumber = 1234 WHERE user_id = ?",
    );
}

#[test]
fn testSingleValuedAssociationFieldInSetClause() {
    assert_sql_generation(
        r"update Doctrine\Tests\Models\CMS\CmsComment c set c.article = null where c.article=?1",
        "UPDATE cms_comments SET article_id = NULL WHERE article_id = ?",
    );
}

#[test]
fn testSubselectTableAliasReferencing() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CMS\CmsUser u SET u.status = 'inactive' WHERE SIZE(u.groups) = 10",
        "UPDATE cms_users SET status = 'inactive' WHERE (SELECT COUNT(*) FROM cms_users_groups c0_ WHERE c0_.user_id = cms_users.id) = 10",
    );
}

#[test]
fn testCustomTypeValueSqlCompletelyIgnoredInUpdateStatements() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\CustomType\CustomTypeParent p SET p.customInteger = 1 WHERE p.id = 1",
        "UPDATE customtype_parents SET customInteger = 1 WHERE id = 1",
    );
}

#[test]
fn testUpdateWithSubselectAsNewValue() {
    assert_sql_generation(
        r"UPDATE Doctrine\Tests\Models\Company\CompanyFixContract fc SET fc.fixPrice = (SELECT ce2.salary FROM Doctrine\Tests\Models\Company\CompanyEmployee ce2 WHERE ce2.id = 2) WHERE fc.id = 1",
        "UPDATE company_contracts SET fixPrice = (SELECT c0_.salary FROM company_employees c0_ INNER JOIN company_persons c1_ ON c0_.id = c1_.id LEFT JOIN company_managers c2_ ON c0_.id = c2_.id WHERE c1_.id = 2) WHERE (id = 1) AND discr IN ('fix')",
    );
}
