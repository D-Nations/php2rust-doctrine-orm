//! Port of tests/Mapping/AbstractClassMetadataFactoryTest.php at
//! doctrine/persistence d8293831c5b99449b2f63eb1e1bbdc202118daa6. The
//! mocked driver is a `MockMappingDriver`; the metadata the callbacks
//! create are `StubClassMetadata`s.
//!
//! Upstream: doctrine/persistence tests/Mapping/AbstractClassMetadataFactoryTest.php

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

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use common::{MockMappingDriver, StubClassMetadata, assert_thrown, same};
use php_value::Thrown;
use php2rust_doctrine_persistence::mapping::driver::MappingDriver;
use php2rust_doctrine_persistence::mapping::{
    AbstractClassMetadataFactory, ClassMetadataFactoryState, ReflectionService, mapping_exception,
};
use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};

const SOME_GRAND_PARENT_ENTITY: &str =
    "Doctrine\\Tests\\Persistence\\Mapping\\SomeGrandParentEntity";
const SOME_PARENT_ENTITY: &str = "Doctrine\\Tests\\Persistence\\Mapping\\SomeParentEntity";
const SOME_ENTITY: &str = "Doctrine\\Tests\\Persistence\\Mapping\\SomeEntity";
const SOME_OTHER_ENTITY: &str = "Doctrine\\Tests\\Persistence\\Mapping\\SomeOtherEntity";
/// `(new class {})::class` in testItThrowsWhenAttemptingToGetMetadataForAnonymousClass.
const ANONYMOUS: &str = "class@anonymous\0tests/Mapping/AbstractClassMetadataFactoryTest.php:63$0";
/// `(new class () {})::class` in testAnonymousClassIsNotMistakenForShortAlias.
const ANONYMOUS_2: &str =
    "class@anonymous\0tests/Mapping/AbstractClassMetadataFactoryTest.php:74$1";

fn register() {
    common::register_fixtures();
    register_class(ClassInfo::class(SOME_GRAND_PARENT_ENTITY));
    register_class(ClassInfo::class(SOME_PARENT_ENTITY).extends(SOME_GRAND_PARENT_ENTITY));
    register_class(ClassInfo::class(SOME_ENTITY).extends(SOME_PARENT_ENTITY));
    register_class(ClassInfo::class(SOME_OTHER_ENTITY));
    register_class(ClassInfo::anonymous(ANONYMOUS));
    register_class(ClassInfo::anonymous(ANONYMOUS_2));
}

type Callback = Box<dyn Fn(&str) -> Arc<StubClassMetadata> + Send + Sync>;

/// `TestAbstractClassMetadataFactory`.
struct TestAbstractClassMetadataFactory {
    state: ClassMetadataFactoryState<StubClassMetadata>,
    new_class_metadata_instance_callback: Mutex<Option<Callback>>,
    driver: Option<Arc<dyn MappingDriver>>,
    default_metadata: Option<Arc<StubClassMetadata>>,
}

impl AbstractClassMetadataFactory for TestAbstractClassMetadataFactory {
    type Metadata = StubClassMetadata;

    fn factory_state(&self) -> &ClassMetadataFactoryState<StubClassMetadata> {
        &self.state
    }

    fn initialize(&self) -> Result<(), Thrown> {
        self.state.set_initialized(true);
        Ok(())
    }

    fn get_driver(&self) -> Result<Arc<dyn MappingDriver>, Thrown> {
        self.driver
            .clone()
            .ok_or_else(|| Thrown::new("RuntimeException", "Driver not set", 0))
    }

    fn wakeup_reflection(
        &self,
        _class: &Arc<StubClassMetadata>,
        _refl_service: &dyn ReflectionService,
    ) -> Result<(), Thrown> {
        // No-op for tests
        Ok(())
    }

    fn initialize_reflection(
        &self,
        _class: &Arc<StubClassMetadata>,
        _refl_service: &dyn ReflectionService,
    ) -> Result<(), Thrown> {
        // No-op for tests
        Ok(())
    }

    fn is_entity(&self, _class: &StubClassMetadata) -> bool {
        true
    }

    fn do_load_metadata(
        &self,
        _class: &Arc<StubClassMetadata>,
        _parent: Option<&Arc<StubClassMetadata>>,
        _root_entity_found: bool,
        _non_superclass_parents: &[String],
    ) -> Result<(), Thrown> {
        // No-op for tests - metadata loading is handled by driver
        Ok(())
    }

    fn new_class_metadata_instance(
        &self,
        class_name: &str,
    ) -> Result<Arc<StubClassMetadata>, Thrown> {
        if let Some(callback) = &*self.new_class_metadata_instance_callback.lock().unwrap() {
            return Ok(callback(class_name));
        }

        self.default_metadata
            .clone()
            .ok_or_else(|| Thrown::new("RuntimeException", "Default metadata not set", 0))
    }
}

fn create_test_factory(
    driver: Option<Arc<dyn MappingDriver>>,
    metadata: Option<Arc<StubClassMetadata>>,
) -> TestAbstractClassMetadataFactory {
    register();
    TestAbstractClassMetadataFactory {
        state: ClassMetadataFactoryState::new(),
        new_class_metadata_instance_callback: Mutex::new(None),
        driver,
        default_metadata: metadata,
    }
}

#[test]
fn testItSkipsTransientClasses() {
    let driver = Arc::new(MockMappingDriver {
        is_transient: Some(Box::new(|class_name: &str| {
            class_name == SOME_PARENT_ENTITY
        })),
        ..MockMappingDriver::default()
    });
    let cmf = create_test_factory(Some(driver.clone()), None);

    let metadata_call_count = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&metadata_call_count);
    *cmf.new_class_metadata_instance_callback.lock().unwrap() =
        Some(Box::new(move |class_name: &str| {
            let call = count.fetch_add(1, Ordering::SeqCst) + 1;
            if call == 1 {
                assert_eq!(SOME_GRAND_PARENT_ENTITY, class_name);
            } else if call == 2 {
                assert_eq!(SOME_ENTITY, class_name);
            }

            StubClassMetadata::new()
        }));

    cmf.get_metadata_for(SOME_ENTITY).unwrap();

    // $driver->expects(self::exactly(2))->method('isTransient')
    assert_eq!(
        driver.is_transient_calls(),
        [SOME_GRAND_PARENT_ENTITY, SOME_PARENT_ENTITY]
    );
}

#[test]
fn testItThrowsWhenAttemptingToGetMetadataForAnonymousClass() {
    let cmf = create_test_factory(None, None);
    assert_thrown(
        cmf.get_metadata_for(ANONYMOUS),
        mapping_exception::CLASS,
        "",
    );
}

#[test]
fn testAnonymousClassIsNotMistakenForShortAlias() {
    let driver = Arc::new(MockMappingDriver {
        is_transient: Some(Box::new(|_| false)),
        ..MockMappingDriver::default()
    });

    let cmf = create_test_factory(Some(driver), None);

    assert!(!cmf.is_transient(ANONYMOUS_2).unwrap());
}

#[test]
fn testItThrowsWhenAttemptingToGetMetadataForShortAlias() {
    let cmf = create_test_factory(None, None);
    assert_thrown(
        cmf.get_metadata_for("App:Test"),
        mapping_exception::CLASS,
        "",
    );
}

#[test]
fn testItThrowsWhenAttemptingToCheckTransientForShortAlias() {
    let cmf = create_test_factory(None, None);
    assert_thrown(cmf.is_transient("App:Test"), mapping_exception::CLASS, "");
}

#[test]
fn testItGetsTheSameMetadataForBackslashedClassName() {
    let driver = MockMappingDriver::stub();
    let cmf = create_test_factory(Some(driver), None);

    let metadata = StubClassMetadata::new();
    let returned = Arc::clone(&metadata);
    *cmf.new_class_metadata_instance_callback.lock().unwrap() =
        Some(Box::new(move |class_name: &str| {
            assert_eq!(SOME_OTHER_ENTITY, class_name);
            Arc::clone(&returned)
        }));

    assert!(same(
        &cmf.get_metadata_for(SOME_OTHER_ENTITY).unwrap(),
        &cmf.get_metadata_for(&format!("\\{SOME_OTHER_ENTITY}"))
            .unwrap()
    ));
}
