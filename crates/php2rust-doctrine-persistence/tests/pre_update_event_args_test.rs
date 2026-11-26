//! Port of tests/Event/PreUpdateEventArgsTest.php at doctrine/persistence
//! d8293831c5b99449b2f63eb1e1bbdc202118daa6. The change set passed by
//! reference is a `Handle`.
//!
//! Upstream: doctrine/persistence tests/Event/PreUpdateEventArgsTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(
    clippy::doc_markdown,
    clippy::disallowed_names,
    reason = "upstream's test, class and field names"
)]
#![allow(non_snake_case)]

mod common;

use common::{MockObjectManager, assert_thrown, new_test_object};
use php_value::{Handle, PhpArray, Value, php_array};
use php2rust_doctrine_persistence::event::PreUpdateEventArgs;

fn change_set() -> PhpArray {
    php_array![
        "name" => php_array!["old", "new"],
        "active" => php_array![0, 1],
    ]
}

fn create_test_pre_update_event_args() -> PreUpdateEventArgs {
    let entity = new_test_object();
    let object_manager = MockObjectManager::new();
    let entity_changeset = Handle::new(change_set());

    PreUpdateEventArgs::new(entity, object_manager, entity_changeset)
}

#[test]
fn testPreUpdateEventArgs() {
    let expected_entity_changeset = change_set();

    let event = create_test_pre_update_event_args();

    assert_eq!(expected_entity_changeset, event.get_entity_change_set());
    assert!(event.has_changed_field("name"));
    assert!(event.has_changed_field("active"));
    assert!(!event.has_changed_field("email"));
    assert!(
        event
            .get_old_value("name")
            .unwrap()
            .identical(&Value::from("old"))
    );
    assert!(
        event
            .get_new_value("name")
            .unwrap()
            .identical(&Value::from("new"))
    );

    event
        .set_new_value("name", Value::from("changed new"))
        .unwrap();

    assert!(
        event
            .get_new_value("name")
            .unwrap()
            .identical(&Value::from("changed new"))
    );
}

const MESSAGE: &str = "Field \"does_not_exist\" is not a valid field of the entity \"Doctrine\\Tests\\Persistence\\TestObject";

#[test]
fn testGetOldValueThrowsInvalidArgumentExceptionOnInvalidField() {
    let event = create_test_pre_update_event_args();

    assert_thrown(
        event.get_old_value("does_not_exist"),
        "InvalidArgumentException",
        MESSAGE,
    );
}

#[test]
fn testGetNewValueThrowsInvalidArgumentExceptionOnInvalidField() {
    let event = create_test_pre_update_event_args();

    assert_thrown(
        event.get_new_value("does_not_exist"),
        "InvalidArgumentException",
        MESSAGE,
    );
}

#[test]
fn testSetNewValueThrowsInvalidArgumentExceptionOnInvalidField() {
    let event = create_test_pre_update_event_args();

    assert_thrown(
        event.set_new_value("does_not_exist", Value::from("new value")),
        "InvalidArgumentException",
        MESSAGE,
    );
}
