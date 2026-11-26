//! Ticket/DDC4024Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (no database).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC4024Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use php2rust_doctrine_orm::abstract_query::{NON_UNIQUE_RESULT_DEFAULT_MESSAGE, non_unique_result};

#[test]
fn testConstructorShouldUseProvidedMessage() {
    let exception = non_unique_result(Some("Testing"));

    assert_eq!("Testing", exception.message.to_string_lossy());
}

#[test]
fn testADefaultMessageShouldBeUsedWhenNothingWasProvided() {
    let exception = non_unique_result(None);

    assert_eq!(
        NON_UNIQUE_RESULT_DEFAULT_MESSAGE,
        exception.message.to_string_lossy()
    );
}
