//! Upstream: doctrine/orm tests/Tests/ORM/Hydration/ResultSetMappingTest.php

#![allow(non_snake_case)]

use php2rust_doctrine_orm::query::ResultSetMapping;

const CMS_USER: &str = "Doctrine\\Tests\\Models\\CMS\\CmsUser";
const CMS_PHONENUMBER: &str = "Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber";
const LEGACY_USER: &str = "Doctrine\\Tests\\Models\\Legacy\\LegacyUser";
const LEGACY_USER_REFERENCE: &str = "Doctrine\\Tests\\Models\\Legacy\\LegacyUserReference";

/// For SQL: SELECT id, status, username, name FROM `cms_users`
#[test]
fn testBasicResultSetMapping() {
    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None);
    rsm.add_field_result("u", "id", "id", None);
    rsm.add_field_result("u", "status", "status", None);
    rsm.add_field_result("u", "username", "username", None);
    rsm.add_field_result("u", "name", "name", None);

    assert!(!rsm.is_scalar_result("id"));
    assert!(!rsm.is_scalar_result("status"));
    assert!(!rsm.is_scalar_result("username"));
    assert!(!rsm.is_scalar_result("name"));

    assert_eq!(rsm.get_class_name("u"), Some(CMS_USER));
    let class = rsm.get_declaring_class("id");
    assert_eq!(class, Some(CMS_USER));

    assert_eq!(Some("u"), rsm.get_entity_alias("id"));
    assert_eq!(Some("u"), rsm.get_entity_alias("status"));
    assert_eq!(Some("u"), rsm.get_entity_alias("username"));
    assert_eq!(Some("u"), rsm.get_entity_alias("name"));

    assert_eq!(Some("id"), rsm.get_field_name("id"));
    assert_eq!(Some("status"), rsm.get_field_name("status"));
    assert_eq!(Some("username"), rsm.get_field_name("username"));
    assert_eq!(Some("name"), rsm.get_field_name("name"));
}

/// Fluent interface test, not a real result set mapping
#[test]
fn testFluentInterface() {
    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None)
        .add_joined_entity_result(CMS_PHONENUMBER, "p", "u", "phonenumbers", false)
        .add_field_result("u", "id", "id", None)
        .add_field_result("u", "name", "name", None)
        .set_discriminator_column("name", "name")
        .add_index_by_column("id", "id")
        .add_index_by("username", "username")
        .add_index_by_scalar("sclr0")
        .add_scalar_result("sclr0", "numPhones", "string")
        .add_meta_result("a", "user_id", "user_id", false, None);

    assert!(rsm.has_index_by("id"));
    assert!(rsm.is_field_result("id"));
    assert!(rsm.is_field_result("name"));
    assert!(rsm.is_scalar_result("sclr0"));
    assert!(rsm.is_relation("p"));
    assert!(rsm.has_parent_alias("p"));
    assert!(rsm.is_mixed_result());
}

#[test]
fn testIndexByMetadataColumn() {
    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(LEGACY_USER, "u", None);
    rsm.add_joined_entity_result(LEGACY_USER_REFERENCE, "lu", "u", "_references", false);
    rsm.add_meta_result("lu", "_source", "_source", true, Some("integer"));
    rsm.add_meta_result("lu", "_target", "_target", true, Some("integer"));
    rsm.add_index_by("lu", "_source");

    assert!(rsm.has_index_by("lu"));
}
