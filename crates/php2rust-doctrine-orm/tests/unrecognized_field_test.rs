//! Port of `UnrecognizedFieldTest`. The exception is returned, not thrown.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Persisters/Exception/UnrecognizedFieldTest.php

#![allow(non_snake_case)]

use php2rust_doctrine_orm::exception::{UNRECOGNIZED_FIELD, unrecognized_field};

/// `Doctrine\Tests\Models\Taxi\Car::class`.
const CAR: &str = "Doctrine\\Tests\\Models\\Taxi\\Car";

#[test]
fn testByFullyQualifiedName() {
    let exception = unrecognized_field(CAR, "color");

    assert_eq!(exception.class, UNRECOGNIZED_FIELD);
    assert_eq!(
        exception.message.as_bytes(),
        b"Unrecognized field: Doctrine\\Tests\\Models\\Taxi\\Car::$color"
    );
}
