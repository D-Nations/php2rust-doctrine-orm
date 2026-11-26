//! `MyToManyAssociationMapping` is a class of the test extending
//! `AssociationMapping` with `ToManyAssociationMappingImplementation`
//! ([`AssociationClass::Custom`] with `to_many`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/ToManyAssociationMappingTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::{Value, php_array};
use php2rust_doctrine_orm::mapping::{
    AbstractBase, AssociationClass, AssociationMapping, serialize_object,
};

const SELF: &str = "Doctrine\\Tests\\ORM\\Mapping\\ToManyAssociationMappingTest";

#[test]
fn testItSurvivesSerialization() {
    let class = AssociationClass::Custom {
        name: "Doctrine\\Tests\\ORM\\Mapping\\MyToManyAssociationMapping".into(),
        base: AbstractBase::Association,
        to_many: true,
    };
    let mut mapping = AssociationMapping::new(class.clone(), "foo", SELF, SELF);

    mapping.index_by = Some("foo".into());
    mapping.order_by = php_array!["foo" => "asc"];

    let resurrected_mapping =
        AssociationMapping::unserialize(serialize_object(&mapping).as_bytes(), Some(&class))
            .unwrap();

    assert!(resurrected_mapping.is_to_many());
    assert_eq!("foo", resurrected_mapping.field_name);
    assert!(
        Value::Array(php_array!["foo" => "asc"])
            .identical(&Value::Array(resurrected_mapping.order_by))
    );
}
