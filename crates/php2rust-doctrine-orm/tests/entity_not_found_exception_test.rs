//! Port of `EntityNotFoundExceptionTest`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/EntityNotFoundExceptionTest.php

#![allow(non_snake_case)]

use php2rust_doctrine_orm::exception::{
    ENTITY_NOT_FOUND_EXCEPTION, entity_not_found_from_class_name_and_identifier,
    no_identifier_found,
};

#[test]
fn testFromClassNameAndIdentifier() {
    let exception = entity_not_found_from_class_name_and_identifier(
        "foo",
        &[("foo".to_owned(), "bar".to_owned())],
    );

    assert_eq!(exception.class, ENTITY_NOT_FOUND_EXCEPTION);
    assert_eq!(
        exception.message.as_bytes(),
        b"Entity of type 'foo' for IDs foo(bar) was not found"
    );

    let exception = entity_not_found_from_class_name_and_identifier("foo", &[]);

    assert_eq!(exception.class, ENTITY_NOT_FOUND_EXCEPTION);
    assert_eq!(
        exception.message.as_bytes(),
        b"Entity of type 'foo' was not found"
    );
}

#[test]
fn testNoIdentifierFound() {
    let exception = no_identifier_found("foo");

    assert_eq!(exception.class, ENTITY_NOT_FOUND_EXCEPTION);
    assert_eq!(
        exception.message.as_bytes(),
        b"Unable to find \"foo\" entity identifier associated with the UnitOfWork"
    );
}
