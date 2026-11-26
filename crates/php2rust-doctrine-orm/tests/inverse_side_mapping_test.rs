//! `MyInverseAssociationMapping` is a class of the test extending the
//! abstract `InverseSideMapping` ([`AssociationClass::Custom`]).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/InverseSideMappingTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php2rust_doctrine_orm::mapping::{
    AbstractBase, AssociationClass, AssociationMapping, serialize_object,
};

const SELF: &str = "Doctrine\\Tests\\ORM\\Mapping\\InverseSideMappingTest";

#[test]
fn testItSurvivesSerialization() {
    let class = AssociationClass::Custom {
        name: "Doctrine\\Tests\\ORM\\Mapping\\MyInverseAssociationMapping".into(),
        base: AbstractBase::InverseSide,
        to_many: false,
    };
    let mut mapping = AssociationMapping::new(class.clone(), "foo", SELF, SELF);

    mapping.mapped_by = Some("bar".into());

    let resurrected_mapping =
        AssociationMapping::unserialize(serialize_object(&mapping).as_bytes(), Some(&class))
            .unwrap();

    assert_eq!(Some("bar"), resurrected_mapping.mapped_by.as_deref());
}
