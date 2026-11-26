//! `MyAssociationMapping` is a class of the test extending the abstract
//! `AssociationMapping` ([`AssociationClass::Custom`]).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/AssociationMappingTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::{Value, php_array};
use php2rust_doctrine_orm::mapping::{
    AbstractBase, AssociationClass, AssociationMapping, serialize_object,
};

const SELF: &str = "Doctrine\\Tests\\ORM\\Mapping\\AssociationMappingTest";
/// `ClassMetadata::FETCH_EAGER`.
const FETCH_EAGER: i64 = 3;
/// `ClassMetadata::CACHE_USAGE_READ_ONLY`.
const CACHE_USAGE_READ_ONLY: i64 = 1;

fn my_association_mapping() -> AssociationClass {
    AssociationClass::Custom {
        name: "Doctrine\\Tests\\ORM\\Mapping\\MyAssociationMapping".into(),
        base: AbstractBase::Association,
        to_many: false,
    }
}

fn new_mapping() -> AssociationMapping {
    AssociationMapping::new(my_association_mapping(), "foo", SELF, SELF)
}

#[test]
fn testItSurvivesSerialization() {
    let mut mapping = new_mapping();

    mapping.cascade = vec!["persist".into()];
    mapping.fetch = Some(FETCH_EAGER);
    mapping.inherited = Some(SELF.into());
    mapping.declared = Some(SELF.into());
    mapping.cache = Some(php_array!["usage" => CACHE_USAGE_READ_ONLY]);
    mapping.id = Some(true);
    mapping.is_on_delete_cascade = Some(true);
    mapping.original_class = Some(SELF.into());
    mapping.original_field = Some("foo".into());
    mapping.orphan_removal = true;
    mapping.unique = Some(true);

    let resurrected_mapping = AssociationMapping::unserialize(
        serialize_object(&mapping).as_bytes(),
        Some(&my_association_mapping()),
    )
    .unwrap();

    assert_eq!(vec!["persist".to_owned()], resurrected_mapping.cascade);
    assert_eq!(Some(FETCH_EAGER), resurrected_mapping.fetch);
    assert_eq!(Some(SELF), resurrected_mapping.inherited.as_deref());
    assert_eq!(Some(SELF), resurrected_mapping.declared.as_deref());
    assert!(
        Value::Array(php_array!["usage" => CACHE_USAGE_READ_ONLY])
            .identical(&Value::Array(resurrected_mapping.cache.clone().unwrap()))
    );
    assert_eq!(Some(true), resurrected_mapping.id);
    assert_eq!(Some(true), resurrected_mapping.is_on_delete_cascade);
    assert_eq!(Some(SELF), resurrected_mapping.original_class.as_deref());
    assert_eq!(Some("foo"), resurrected_mapping.original_field.as_deref());
    assert!(resurrected_mapping.orphan_removal);
    assert_eq!(Some(true), resurrected_mapping.unique);
}

#[test]
fn testItThrowsWhenAccessingUnknownProperty() {
    let mapping = new_mapping();

    assert_eq!(
        "OutOfRangeException",
        mapping.offset_get("foo").unwrap_err().class
    );
}

#[test]
fn testItThrowsWhenSettingUnknownProperty() {
    let mut mapping = new_mapping();

    assert_eq!(
        "OutOfRangeException",
        mapping
            .offset_set("foo", Value::from("bar"))
            .unwrap_err()
            .class
    );
}

#[test]
fn testItThrowsWhenUnsettingUnknownProperty() {
    let mut mapping = new_mapping();

    assert_eq!(
        "OutOfRangeException",
        mapping.offset_unset("foo").unwrap_err().class
    );
}
