//! Port of tests/Tests/ORM/Internal/HydrationCompleteHandlerTest.php. The
//! mock of `ListenersInvoker` records its `invoke()` calls; the stubs of
//! `ClassMetadata` are metadata of a class each, told apart by identity.
//! A data provider's rows run in one test.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Internal/HydrationCompleteHandlerTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::{Arc, Mutex};

use php_value::{Object, Thrown};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::event::{LifecycleEventArgs, ListenersInvoker};
use php2rust_doctrine_orm::events;
use php2rust_doctrine_orm::internal::{HydrationCompleteHandler, PostLoadListenersInvoker};
use php2rust_doctrine_orm::mapping::ClassMetadata;

/// An `invoke()` call: class, event name, entity, the event's object and
/// object manager, the flag.
type Invocation = (
    Arc<ClassMetadata>,
    String,
    Object,
    Object,
    Arc<EntityManager>,
    i32,
);

/// `createMock(ListenersInvoker::class)`.
struct MockListenersInvoker {
    /// `->method('getSubscribedSystems')->with(...)->willReturn($flag)`:
    /// the metadata it expects and the flag.
    subscribed: (Vec<Arc<ClassMetadata>>, i32),
    invocations: Mutex<Vec<Invocation>>,
}

impl MockListenersInvoker {
    fn new(metadata: &[&Arc<ClassMetadata>], flag: i32) -> Self {
        Self {
            subscribed: (metadata.iter().map(|m| Arc::clone(m)).collect(), flag),
            invocations: Mutex::new(Vec::new()),
        }
    }

    fn invocations(&self) -> Vec<Invocation> {
        self.invocations.lock().unwrap().clone()
    }
}

impl PostLoadListenersInvoker<Arc<ClassMetadata>> for MockListenersInvoker {
    fn get_subscribed_systems(&self, class: &Arc<ClassMetadata>, event_name: &str) -> i32 {
        assert!(
            self.subscribed.0.iter().any(|m| Arc::ptr_eq(m, class)),
            "getSubscribedSystems() with unexpected metadata"
        );
        assert_eq!(events::POST_LOAD, event_name);
        self.subscribed.1
    }

    fn invoke(
        &self,
        class: &Arc<ClassMetadata>,
        event_name: &str,
        entity: &Object,
        event: &LifecycleEventArgs,
        invoke: i32,
    ) -> Result<(), Thrown> {
        self.invocations.lock().unwrap().push((
            class.clone(),
            event_name.to_owned(),
            entity.clone(),
            event.get_object().clone(),
            event.get_object_manager().clone(),
            invoke,
        ));
        Ok(())
    }
}

/// `createStub(ClassMetadata::class)`.
fn metadata() -> Arc<ClassMetadata> {
    Arc::new(ClassMetadata::new("stdClass", None, None))
}

/// `new stdClass()`.
fn std_class() -> Object {
    php_value::object::StdClass::object(php_value::PhpArray::new())
        .as_object()
        .unwrap()
        .clone()
}

/// `createStub(EntityManagerInterface::class)`.
fn entity_manager() -> Arc<EntityManager> {
    orm::entity_manager(orm::connection(), None)
}

/// `invocationFlagProvider()`.
fn invocation_flag_provider() -> [i32; 6] {
    [
        ListenersInvoker::INVOKE_LISTENERS,
        ListenersInvoker::INVOKE_CALLBACKS,
        ListenersInvoker::INVOKE_MANAGER,
        ListenersInvoker::INVOKE_LISTENERS | ListenersInvoker::INVOKE_CALLBACKS,
        ListenersInvoker::INVOKE_LISTENERS | ListenersInvoker::INVOKE_MANAGER,
        ListenersInvoker::INVOKE_LISTENERS
            | ListenersInvoker::INVOKE_CALLBACKS
            | ListenersInvoker::INVOKE_MANAGER,
    ]
}

#[tokio::test]
async fn testDefersPostLoadOfEntity() {
    for listeners_flag in invocation_flag_provider() {
        let entity_manager = entity_manager();
        let mut handler = HydrationCompleteHandler::new();
        let metadata = metadata();
        let entity = std_class();

        let listeners_invoker = MockListenersInvoker::new(&[&metadata], listeners_flag);

        handler.defer_post_load_invoking(&listeners_invoker, metadata.clone(), entity.clone());

        handler
            .hydration_complete(&listeners_invoker, &entity_manager)
            .unwrap();

        let invocations = listeners_invoker.invocations();
        assert_eq!(1, invocations.len(), "{listeners_flag}");
        let (class, event_name, object, args_object, args_em, flag) = &invocations[0];
        assert!(Arc::ptr_eq(&metadata, class));
        assert_eq!(events::POST_LOAD, event_name);
        assert!(Arc::ptr_eq(&entity, object));
        assert!(Arc::ptr_eq(&entity, args_object));
        assert!(Arc::ptr_eq(&entity_manager, args_em));
        assert_eq!(listeners_flag, *flag);
    }
}

#[tokio::test]
async fn testDefersPostLoadOfEntityOnlyOnce() {
    for listeners_flag in invocation_flag_provider() {
        let entity_manager = entity_manager();
        let mut handler = HydrationCompleteHandler::new();
        let metadata = metadata();
        let entity = std_class();

        let listeners_invoker = MockListenersInvoker::new(&[&metadata], listeners_flag);

        handler.defer_post_load_invoking(&listeners_invoker, metadata.clone(), entity);

        handler
            .hydration_complete(&listeners_invoker, &entity_manager)
            .unwrap();
        handler
            .hydration_complete(&listeners_invoker, &entity_manager)
            .unwrap();

        assert_eq!(1, listeners_invoker.invocations().len(), "{listeners_flag}");
    }
}

#[tokio::test]
async fn testDefersMultiplePostLoadOfEntity() {
    for listeners_flag in invocation_flag_provider() {
        let entity_manager = entity_manager();
        let mut handler = HydrationCompleteHandler::new();
        let metadata1 = metadata();
        let metadata2 = metadata();
        let entity1 = std_class();
        let entity2 = std_class();

        let listeners_invoker =
            MockListenersInvoker::new(&[&metadata1, &metadata2], listeners_flag);

        handler.defer_post_load_invoking(&listeners_invoker, metadata1.clone(), entity1.clone());
        handler.defer_post_load_invoking(&listeners_invoker, metadata2.clone(), entity2.clone());

        handler
            .hydration_complete(&listeners_invoker, &entity_manager)
            .unwrap();

        let invocations = listeners_invoker.invocations();
        assert_eq!(2, invocations.len(), "{listeners_flag}");
        for (class, event_name, object, args_object, args_em, flag) in &invocations {
            assert!(Arc::ptr_eq(&metadata1, class) || Arc::ptr_eq(&metadata2, class));
            assert_eq!(events::POST_LOAD, event_name);
            assert!(Arc::ptr_eq(&entity1, object) || Arc::ptr_eq(&entity2, object));
            assert!(Arc::ptr_eq(&entity1, args_object) || Arc::ptr_eq(&entity2, args_object));
            assert!(Arc::ptr_eq(&entity_manager, args_em));
            assert_eq!(listeners_flag, *flag);
        }
    }
}

#[tokio::test]
async fn testSkipsDeferredPostLoadOfMetadataWithNoInvokedListeners() {
    let entity_manager = entity_manager();
    let mut handler = HydrationCompleteHandler::new();
    let metadata = metadata();
    let entity = std_class();

    let listeners_invoker = MockListenersInvoker::new(&[&metadata], ListenersInvoker::INVOKE_NONE);

    handler.defer_post_load_invoking(&listeners_invoker, metadata, entity);

    handler
        .hydration_complete(&listeners_invoker, &entity_manager)
        .unwrap();

    assert!(listeners_invoker.invocations().is_empty());
}
