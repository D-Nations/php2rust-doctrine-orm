//! Port of tests/ManagerRegistryTest.php at doctrine/persistence
//! d8293831c5b99449b2f63eb1e1bbdc202118daa6. The anonymous classes are
//! declared with the names PHP gives them; the mocked managers are
//! `MockObjectManager`s.
//!
//! Upstream: doctrine/persistence tests/ManagerRegistryTest.php

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

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use common::{
    MockMappingDriver, MockObjectManager, OTHER_TEST_OBJECT, StubClassMetadata,
    StubObjectRepository, TEST_OBJECT, TEST_OBJECT_PROXY, TestClassMetadataFactory,
    register_fixtures, same,
};
use php_value::Thrown;
use php2rust_doctrine_persistence::php_reflection::{ClassInfo, MethodInfo, register_class};
use php2rust_doctrine_persistence::{
    AbstractManagerRegistry, ManagerRegistry, ManagerRegistryState, ObjectRepository, Service,
    proxy,
};

/// `new class extends TestObject implements Proxy { ... }`.
const ANONYMOUS_PROXY: &str =
    "Doctrine\\Tests\\Persistence\\TestObject@anonymous\0tests/ManagerRegistryTest.php:47$0";
/// `new class extends TestObject implements Proxy { ... }` in
/// testGetManagerForWithoutProxyInterface.
const ANONYMOUS_PROXY_2: &str =
    "Doctrine\\Tests\\Persistence\\TestObject@anonymous\0tests/ManagerRegistryTest.php:101$0";
/// `new class {}`.
const ANONYMOUS: &str = "class@anonymous\0tests/ManagerRegistryTest.php:77$0";

fn register_anonymous_classes() {
    register_fixtures();
    for name in [ANONYMOUS_PROXY, ANONYMOUS_PROXY_2] {
        register_class(
            ClassInfo::anonymous(name)
                .extends(TEST_OBJECT)
                .implements(proxy::CLASS)
                .method(MethodInfo::new("__isInitialized"))
                .method(MethodInfo::new("__load")),
        );
    }
    register_class(ClassInfo::anonymous(ANONYMOUS));
}

type ManagerFactory = Box<dyn Fn(&str) -> Arc<MockObjectManager> + Send + Sync>;

/// `TestManagerRegistry`.
struct TestManagerRegistry {
    state: ManagerRegistryState,
    services: Mutex<HashMap<String, Arc<MockObjectManager>>>,
    manager_factory: ManagerFactory,
}

impl TestManagerRegistry {
    fn new(
        name: &str,
        connections: &[(&str, &str)],
        managers: &[(&str, &str)],
        default_connection: &str,
        default_manager: &str,
        proxy_interface_name: Option<&str>,
        manager_factory: ManagerFactory,
    ) -> Self {
        let pairs = |list: &[(&str, &str)]| {
            list.iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect()
        };
        Self {
            state: ManagerRegistryState::new(
                name,
                pairs(connections),
                pairs(managers),
                default_connection,
                default_manager,
                proxy_interface_name.map(str::to_owned),
            ),
            services: Mutex::new(HashMap::new()),
            manager_factory,
        }
    }

    /// The mock behind `getService($name)`.
    fn service(&self, name: &str) -> Arc<MockObjectManager> {
        let mut services = self.services.lock().unwrap();
        Arc::clone(
            services
                .entry(name.to_owned())
                .or_insert_with(|| (self.manager_factory)(name)),
        )
    }
}

impl AbstractManagerRegistry for TestManagerRegistry {
    fn registry_state(&self) -> &ManagerRegistryState {
        &self.state
    }

    fn get_service(&self, name: &str) -> Result<Service, Thrown> {
        Ok(Service::Manager(self.service(name)))
    }

    fn reset_service(&self, name: &str) {
        self.services.lock().unwrap().remove(name);
    }
}

fn get_manager_factory() -> ManagerFactory {
    Box::new(|name: &str| {
        let mock = MockObjectManager::new();
        let driver = MockMappingDriver::stub();
        let metadata = StubClassMetadata::named(if name == "other_manager" {
            OTHER_TEST_OBJECT
        } else {
            TEST_OBJECT
        });
        *mock.metadata_factory.lock().unwrap() =
            Some(Arc::new(TestClassMetadataFactory::new(driver, metadata)));
        mock
    })
}

fn set_up() -> TestManagerRegistry {
    register_anonymous_classes();
    TestManagerRegistry::new(
        "ORM",
        &[("default", "default_connection")],
        &[("default", "default_manager")],
        "default",
        "default",
        Some(proxy::CLASS),
        get_manager_factory(),
    )
}

fn two_managers() -> TestManagerRegistry {
    register_anonymous_classes();
    TestManagerRegistry::new(
        "ORM",
        &[("default", "default_connection")],
        &[("default", "default_manager"), ("other", "other_manager")],
        "default",
        "default",
        Some(proxy::CLASS),
        get_manager_factory(),
    )
}

/// `$mock->expects($this->never())->method('clear')`.
fn assert_never_cleared(mr: &TestManagerRegistry) {
    for mock in mr.services.lock().unwrap().values() {
        assert!(mock.calls_of("clear").is_empty());
    }
}

#[test]
fn testGetManagerForClass() {
    let mr = set_up();
    assert!(mr.get_manager_for_class(TEST_OBJECT).unwrap().is_some());
    assert_never_cleared(&mr);
}

#[test]
fn testGetManagerForClassAnonymous() {
    let mr = set_up();
    assert!(mr.get_manager_for_class(ANONYMOUS_PROXY).unwrap().is_none());
}

#[test]
fn testGetManagerForProxiedClass() {
    let mr = set_up();
    assert!(
        mr.get_manager_for_class(TEST_OBJECT_PROXY)
            .unwrap()
            .is_some()
    );
}

#[test]
fn testGetManagerForProxyInterface() {
    let mr = set_up();
    assert!(mr.get_manager_for_class(proxy::CLASS).unwrap().is_none());
}

#[test]
fn testGetManagerForAnonymousClass() {
    let mr = set_up();
    assert!(mr.get_manager_for_class(ANONYMOUS).unwrap().is_none());
}

#[test]
fn testGetManagerForWithoutProxyInterface() {
    register_anonymous_classes();
    let mr = TestManagerRegistry::new(
        "ORM",
        &[("default", "default_connection")],
        &[("default", "default_manager")],
        "default",
        "default",
        None,
        get_manager_factory(),
    );

    assert!(mr.get_manager_for_class(TEST_OBJECT).unwrap().is_some());

    assert!(
        mr.get_manager_for_class(TEST_OBJECT_PROXY)
            .unwrap()
            .is_none()
    );

    assert!(
        mr.get_manager_for_class(ANONYMOUS_PROXY_2)
            .unwrap()
            .is_none()
    );
}

#[test]
fn testResetManager() {
    let mr = set_up();
    let manager = mr.get_manager(None).unwrap();
    let new_manager = mr.reset_manager(None).unwrap();

    assert!(!same(&manager, &new_manager));
}

#[test]
fn testGetRepository() {
    let mr = set_up();
    let repository = StubObjectRepository::shared();

    mr.get_manager(None).unwrap();
    let default_manager = mr.service("default_manager");
    *default_manager.repository.lock().unwrap() = Some(Arc::clone(&repository));

    let found: Arc<dyn ObjectRepository> = mr.get_repository(TEST_OBJECT, None).unwrap();
    assert!(same(&repository, &found));
    let calls = default_manager.calls_of("getRepository");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].class_name.as_deref(), Some(TEST_OBJECT));
}

#[test]
fn testGetRepositoryWithSpecificManagerName() {
    let mr = two_managers();

    let repository = StubObjectRepository::shared();

    mr.get_manager(None).unwrap();
    let default_manager = mr.service("default_manager");

    mr.get_manager(Some("other")).unwrap();
    let other_manager = mr.service("other_manager");
    *other_manager.repository.lock().unwrap() = Some(Arc::clone(&repository));

    let found = mr.get_repository(TEST_OBJECT, Some("other")).unwrap();
    assert!(same(&repository, &found));
    assert!(default_manager.calls_of("getRepository").is_empty());
    let calls = other_manager.calls_of("getRepository");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].class_name.as_deref(), Some(TEST_OBJECT));
}

#[test]
fn testGetRepositoryWithManagerDetection() {
    let mr = two_managers();

    let repository = StubObjectRepository::shared();

    mr.get_manager(None).unwrap();
    let default_manager = mr.service("default_manager");

    mr.get_manager(Some("other")).unwrap();
    let other_manager = mr.service("other_manager");
    *other_manager.repository.lock().unwrap() = Some(Arc::clone(&repository));

    let found = mr.get_repository(OTHER_TEST_OBJECT, None).unwrap();
    assert!(same(&repository, &found));
    assert!(default_manager.calls_of("getRepository").is_empty());
    let calls = other_manager.calls_of("getRepository");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].class_name.as_deref(), Some(OTHER_TEST_OBJECT));
}
