//! Port of `AttachEntityListenersListenerTest`. The entity manager's
//! event manager is the factory configuration's `loadClassMetadata`
//! hooks: both factories are set up with the same configuration.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/AttachEntityListenersListenerTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;

use std::sync::Arc;

use php2rust_doctrine_orm::events;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::{
    ClassMetadataFactory, ClassMetadataInfo, FactoryConfiguration, mapping_exception,
};
use php2rust_doctrine_orm::tools::AttachEntityListenersListener;
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;

const NS: &str = "Doctrine\\Tests\\ORM\\Tools\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

struct Test {
    listener: Arc<AttachEntityListenersListener>,
    configuration: FactoryConfiguration,
    factory: ClassMetadataFactory,
}

/// `setUp()`.
fn set_up() -> Test {
    models::register();
    let listener = Arc::new(AttachEntityListenersListener::new());
    let driver = Arc::new(AttributeDriver::new(Vec::<String>::new(), true).unwrap());
    let mut configuration = FactoryConfiguration::new(driver);
    let factory = ClassMetadataFactory::new();

    configuration.load_class_metadata.push(listener.hook());
    factory.set_entity_manager(configuration.clone());

    Test {
        listener,
        configuration,
        factory,
    }
}

/// `$metadata->entityListeners[$event][$i]` as `[class, method]`.
fn listener(metadata: &ClassMetadataInfo, event: &str, i: usize) -> (String, String) {
    let listener = &metadata.entity_listeners.get(event).unwrap()[i];
    (listener.class.clone(), listener.method.clone())
}

fn count(metadata: &ClassMetadataInfo, event: &str) -> usize {
    metadata.entity_listeners.get(event).unwrap().len()
}

#[test]
fn testAttachEntityListeners() {
    let test = set_up();
    test.listener.add_entity_listener(
        &class("AttachEntityListenersListenerTestFooEntity"),
        &class("AttachEntityListenersListenerTestListener"),
        Some(events::POST_LOAD),
        Some("postLoadHandler"),
    );

    let metadata = test
        .factory
        .get_metadata_for(&class("AttachEntityListenersListenerTestFooEntity"))
        .unwrap();
    let metadata = metadata.read();

    assert_eq!(1, count(&metadata, "postLoad"));
    assert_eq!(
        (
            class("AttachEntityListenersListenerTestListener"),
            "postLoadHandler".to_owned()
        ),
        listener(&metadata, "postLoad", 0)
    );

    // Can reattach entity listeners even class metadata factory recreated.
    let factory2 = ClassMetadataFactory::new();
    factory2.set_entity_manager(test.configuration.clone());

    let metadata2 = factory2
        .get_metadata_for(&class("AttachEntityListenersListenerTestFooEntity"))
        .unwrap();
    let metadata2 = metadata2.read();

    assert_eq!(1, count(&metadata2, "postLoad"));
    assert_eq!(
        (
            class("AttachEntityListenersListenerTestListener"),
            "postLoadHandler".to_owned()
        ),
        listener(&metadata2, "postLoad", 0)
    );
}

#[test]
fn testAttachToExistingEntityListeners() {
    let test = set_up();
    test.listener.add_entity_listener(
        &class("AttachEntityListenersListenerTestBarEntity"),
        &class("AttachEntityListenersListenerTestListener2"),
        Some(events::PRE_PERSIST),
        None,
    );

    test.listener.add_entity_listener(
        &class("AttachEntityListenersListenerTestBarEntity"),
        &class("AttachEntityListenersListenerTestListener2"),
        Some(events::POST_PERSIST),
        Some("postPersistHandler"),
    );

    let metadata = test
        .factory
        .get_metadata_for(&class("AttachEntityListenersListenerTestBarEntity"))
        .unwrap();
    let metadata = metadata.read();

    assert_eq!(2, count(&metadata, "prePersist"));
    assert_eq!(2, count(&metadata, "postPersist"));

    let listener1 = class("AttachEntityListenersListenerTestListener");
    let listener2 = class("AttachEntityListenersListenerTestListener2");
    assert_eq!(
        (listener1.clone(), "prePersist".to_owned()),
        listener(&metadata, "prePersist", 0)
    );
    assert_eq!(
        (listener2.clone(), "prePersist".to_owned()),
        listener(&metadata, "prePersist", 1)
    );
    assert_eq!(
        (listener1, "postPersist".to_owned()),
        listener(&metadata, "postPersist", 0)
    );
    assert_eq!(
        (listener2, "postPersistHandler".to_owned()),
        listener(&metadata, "postPersist", 1)
    );
}

#[test]
fn testDuplicateEntityListenerException() {
    let test = set_up();
    test.listener.add_entity_listener(
        &class("AttachEntityListenersListenerTestFooEntity"),
        &class("AttachEntityListenersListenerTestListener"),
        Some(events::POST_PERSIST),
        None,
    );

    test.listener.add_entity_listener(
        &class("AttachEntityListenersListenerTestFooEntity"),
        &class("AttachEntityListenersListenerTestListener"),
        Some(events::POST_PERSIST),
        None,
    );

    let Err(e) = test
        .factory
        .get_metadata_for(&class("AttachEntityListenersListenerTestFooEntity"))
    else {
        panic!("expected a MappingException");
    };
    assert!(thrown_is_a(&e, mapping_exception::CLASS));
    assert_eq!(
        "Entity Listener \"Doctrine\\Tests\\ORM\\Tools\\AttachEntityListenersListenerTestListener#postPersist()\" in \"Doctrine\\Tests\\ORM\\Tools\\AttachEntityListenersListenerTestFooEntity\" was already declared, but it must be declared only once.",
        String::from_utf8_lossy(e.message.as_bytes())
    );
}

#[test]
fn testAttachWithoutSpecifyingAnEventName() {
    let test = set_up();
    test.listener.add_entity_listener(
        &class("AttachEntityListenersListenerTestFooEntity"),
        &class("AttachEntityListenersListenerTestListener"),
        None,
        None,
    );

    let metadata = test
        .factory
        .get_metadata_for(&class("AttachEntityListenersListenerTestFooEntity"))
        .unwrap();
    let metadata = metadata.read();

    assert_eq!(2, metadata.entity_listeners.len());

    assert_eq!(1, count(&metadata, "prePersist"));
    assert_eq!(1, count(&metadata, "postPersist"));

    let listener1 = class("AttachEntityListenersListenerTestListener");
    assert_eq!(
        (listener1.clone(), "prePersist".to_owned()),
        listener(&metadata, "prePersist", 0)
    );
    assert_eq!(
        (listener1, "postPersist".to_owned()),
        listener(&metadata, "postPersist", 0)
    );
}
