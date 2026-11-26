//! Port of tests/ObjectManagerDecoratorTest.php at doctrine/persistence
//! d8293831c5b99449b2f63eb1e1bbdc202118daa6. `NullObjectManagerDecorator`
//! is an `ObjectManagerDecorator` over a `MockObjectManager`.
//!
//! Upstream: doctrine/persistence tests/ObjectManagerDecoratorTest.php

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

use std::sync::Arc;

use common::{
    MockMappingDriver, MockObjectManager, StubClassMetadata, StubObjectRepository, TEST_OBJECT,
    TestClassMetadataFactory, new_test_object, same,
};
use php_value::{Object, Value};
use php2rust_doctrine_persistence::mapping::{ClassMetadata, ClassMetadataFactory};
use php2rust_doctrine_persistence::{ObjectManager, ObjectManagerDecorator};

type NullObjectManagerDecorator = ObjectManagerDecorator<MockObjectManager>;

fn set_up() -> (Arc<MockObjectManager>, NullObjectManagerDecorator) {
    let wrapped = MockObjectManager::new();
    let decorated = ObjectManagerDecorator::new(Arc::clone(&wrapped));
    (wrapped, decorated)
}

/// `->with($object)`: the one call of `method` got `object`.
#[track_caller]
fn assert_called_once_with(wrapped: &MockObjectManager, method: &str, object: &Object) {
    let calls = wrapped.calls_of(method);
    assert_eq!(calls.len(), 1, "{method} calls");
    assert!(calls[0].arguments[0].identical(&Value::Object(Arc::clone(object))));
}

#[test]
fn testFind() {
    let (wrapped, decorated) = set_up();
    let object = new_test_object();
    *wrapped.find_result.lock().unwrap() = Some(Arc::clone(&object));

    let found = decorated
        .find(TEST_OBJECT, &Value::Int(1))
        .unwrap()
        .unwrap();
    assert!(same(&object, &found));
    let calls = wrapped.calls_of("find");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].class_name.as_deref(), Some(TEST_OBJECT));
    assert!(calls[0].arguments[0].identical(&Value::Int(1)));
}

#[test]
fn testPersist() {
    let (wrapped, decorated) = set_up();
    let object = new_test_object();

    decorated.persist(&object).unwrap();
    assert_called_once_with(&wrapped, "persist", &object);
}

#[test]
fn testRemove() {
    let (wrapped, decorated) = set_up();
    let object = new_test_object();

    decorated.remove(&object).unwrap();
    assert_called_once_with(&wrapped, "remove", &object);
}

#[test]
fn testClear() {
    let (wrapped, decorated) = set_up();

    decorated.clear().unwrap();
    assert_eq!(wrapped.calls_of("clear").len(), 1);
}

#[test]
fn testDetach() {
    let (wrapped, decorated) = set_up();
    let object = new_test_object();

    decorated.detach(&object).unwrap();
    assert_called_once_with(&wrapped, "detach", &object);
}

#[test]
fn testRefresh() {
    let (wrapped, decorated) = set_up();
    let object = new_test_object();

    decorated.refresh(&object).unwrap();
    assert_called_once_with(&wrapped, "refresh", &object);
}

#[test]
fn testFlush() {
    let (wrapped, decorated) = set_up();

    decorated.flush().unwrap();
    assert_eq!(wrapped.calls_of("flush").len(), 1);
}

#[test]
fn testGetRepository() {
    let (wrapped, decorated) = set_up();
    let repository = StubObjectRepository::shared();
    *wrapped.repository.lock().unwrap() = Some(Arc::clone(&repository));

    assert!(same(
        &repository,
        &decorated.get_repository(TEST_OBJECT).unwrap()
    ));
    let calls = wrapped.calls_of("getRepository");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].class_name.as_deref(), Some(TEST_OBJECT));
}

#[test]
fn testGetClassMetadata() {
    let (wrapped, decorated) = set_up();
    let class_metadata: Arc<dyn ClassMetadata> = StubClassMetadata::new();
    *wrapped.class_metadata.lock().unwrap() = Some(Arc::clone(&class_metadata));

    assert!(same(
        &class_metadata,
        &decorated.get_class_metadata(TEST_OBJECT).unwrap()
    ));
    let calls = wrapped.calls_of("getClassMetadata");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].class_name.as_deref(), Some(TEST_OBJECT));
}

#[test]
fn testGetClassMetadataFactory() {
    let (wrapped, decorated) = set_up();
    let class_metadata_factory: Arc<dyn ClassMetadataFactory> = Arc::new(
        TestClassMetadataFactory::new(MockMappingDriver::stub(), StubClassMetadata::new()),
    );
    *wrapped.metadata_factory.lock().unwrap() = Some(Arc::clone(&class_metadata_factory));

    assert!(same(
        &class_metadata_factory,
        &decorated.get_metadata_factory()
    ));
    assert_eq!(wrapped.calls_of("getMetadataFactory").len(), 1);
}

#[test]
fn testInitializeObject() {
    let (wrapped, decorated) = set_up();
    let object = new_test_object();

    decorated.initialize_object(&object).unwrap();
    assert_called_once_with(&wrapped, "initializeObject", &object);
}

#[test]
fn testContains() {
    let (wrapped, decorated) = set_up();
    let object = new_test_object();
    *wrapped.contains_result.lock().unwrap() = true;

    assert!(decorated.contains(&object));
    assert_called_once_with(&wrapped, "contains", &object);
}

#[test]
fn testIsUninitializedObject() {
    let (wrapped, decorated) = set_up();
    let object = new_test_object();
    *wrapped.is_uninitialized_result.lock().unwrap() = false;

    assert!(!decorated.is_uninitialized_object(&Value::Object(Arc::clone(&object))));
    assert_called_once_with(&wrapped, "isUninitializedObject", &object);
}
