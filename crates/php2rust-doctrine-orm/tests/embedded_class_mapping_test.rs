//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/EmbeddedClassMappingTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php2rust_doctrine_orm::mapping::{ColumnPrefix, EmbeddedClassMapping, serialize_object};

const SELF: &str = "Doctrine\\Tests\\ORM\\Mapping\\EmbeddedClassMappingTest";

#[test]
fn testItSurvivesSerialization() {
    let mut mapping = EmbeddedClassMapping::new(SELF);
    mapping.column_prefix = ColumnPrefix::Prefix("these".into());
    mapping.declared_field = Some("values".into());
    mapping.original_field = Some("make".into());
    mapping.inherited = Some(SELF.into()); // no
    mapping.declared = Some(SELF.into()); // sense

    let resurrected_mapping =
        EmbeddedClassMapping::unserialize(serialize_object(&mapping).as_bytes()).unwrap();

    assert_eq!(
        ColumnPrefix::Prefix("these".into()),
        resurrected_mapping.column_prefix
    );
    assert_eq!(
        Some("values"),
        resurrected_mapping.declared_field.as_deref()
    );
    assert_eq!(Some("make"), resurrected_mapping.original_field.as_deref());
    assert_eq!(Some(SELF), resurrected_mapping.inherited.as_deref());
    assert_eq!(Some(SELF), resurrected_mapping.declared.as_deref());
}
