//! Upstream: doctrine/orm tests/Tests/ORM/Proxy/ProxyFactoryTest.php
//!
//! The entity manager is a `SQLite` one with the `ecommerce` model set and
//! native lazy objects (`tests/common/orm.rs`), whose statements are
//! logged, in place of `EntityManagerMock` and `UnitOfWorkMock`. The mocked
//! persister's `loadById()` is the real persister: the delegating test
//! checks the one statement it runs with the identifier, and the failing
//! loads are of an identifier the database does not have. `clone $proxy`
//! initializes the proxy first, as PHP 8.4 does for a lazy object (see
//! `ReferenceProxyTest`). `AbstractClass` is not a recorded class: its
//! `ClassMetadata` has no reflection.
//!
//! `testProxyFactoryThrowsIfLazyGhostsAreUnavailable`: the port has no
//! `LazyGhost` proxies, as PHP 8.4 with symfony/var-exporter 8 (no
//! `ProxyHelper::generateLazyGhost()`); the entity manager's own
//! `new ProxyFactory($this)` is the one that throws, as the mock's
//! configuration cannot change after `create()`.

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
#[path = "common/query_log.rs"]
mod query_log;

use std::sync::Arc;

use php_value::{Object, PhpArray, Value, php_array};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::exception::ENTITY_NOT_FOUND_EXCEPTION;
use php2rust_doctrine_orm::mapping::ClassMetadata;
use php2rust_doctrine_orm::proxy::ProxyFactory;

use orm::{ECOMMERCE_FEATURE, ECOMMERCE_MODELS, get, new_object, set};
use query_log::QueryLog;

struct Test {
    em: Arc<EntityManager>,
    log: Arc<QueryLog>,
    proxy_factory: ProxyFactory,
}

async fn set_up() -> Test {
    let (em, log) = query_log::set_up_logged(&ECOMMERCE_MODELS).await;
    // isNativeLazyObjectsEnabled(): [$this->emMock]
    let proxy_factory = ProxyFactory::new(&em);
    Test {
        em,
        log,
        proxy_factory,
    }
}

fn identifier() -> PhpArray {
    php_array!["id" => 42]
}

#[track_caller]
fn assert_uninitialized_lazy_object(em: &EntityManager, proxy: &Object) {
    assert!(em.is_uninitialized_object(&Value::Object(proxy.clone())));
}

#[tokio::test]
async fn testReferenceProxyDelegatesLoadingToThePersister() {
    let t = set_up().await;
    let feature = new_object(ECOMMERCE_FEATURE);
    set(&feature, "description", Value::string("Feature"));
    t.em.persist(&feature).unwrap();
    t.em.flush().await.unwrap();
    let identifier = php_array!["id" => get(&feature, "id")];
    t.em.clear().unwrap();

    let proxy = t
        .proxy_factory
        .get_proxy(ECOMMERCE_FEATURE, &identifier)
        .unwrap();

    t.log.reset_and_enable();
    // $proxy->getDescription()
    t.em.initialize_object(&proxy).await.unwrap();

    let entries = t.log.entries();
    assert_eq!(1, entries.len(), "{entries:#?}");
    assert!(
        entries[0].0.ends_with("WHERE t0.id = ?"),
        "{}",
        entries[0].0
    );
    assert_eq!(Some(&get(&feature, "id")), entries[0].1.values().next());
    assert_eq!(Value::string("Feature"), get(&proxy, "description"));
}

#[tokio::test]
async fn testSkipMappedSuperClassesOnGeneration() {
    let t = set_up().await;
    let cm = ClassMetadata::new("stdClass", None, None);
    cm.write().is_mapped_superclass = true;

    assert_eq!(
        0,
        t.proxy_factory
            .generate_proxy_classes(&[Arc::new(cm)], None),
        "No proxies generated."
    );
}

/// 6625
#[tokio::test]
async fn testSkipEmbeddableClassesOnGeneration() {
    let t = set_up().await;
    let cm = ClassMetadata::new("stdClass", None, None);
    cm.write().is_embedded_class = true;

    assert_eq!(
        0,
        t.proxy_factory
            .generate_proxy_classes(&[Arc::new(cm)], None),
        "No proxies generated."
    );
}

/// DDC-1771
#[tokio::test]
async fn testSkipAbstractClassesOnGeneration() {
    let t = set_up().await;
    let cm = ClassMetadata::new("Doctrine\\Tests\\ORM\\Proxy\\AbstractClass", None, None);

    let num = t
        .proxy_factory
        .generate_proxy_classes(&[Arc::new(cm)], None);

    assert_eq!(0, num, "No proxies generated.");
}

/// DDC-2432
#[tokio::test]
async fn testFailedProxyLoadingDoesNotMarkTheProxyAsInitialized() {
    let t = set_up().await;
    let proxy = t
        .proxy_factory
        .get_proxy(ECOMMERCE_FEATURE, &identifier())
        .unwrap();

    // $proxy->getDescription()
    let error = t.em.initialize_object(&proxy).await.unwrap_err();
    assert_eq!(ENTITY_NOT_FOUND_EXCEPTION, error.class);

    assert_uninitialized_lazy_object(&t.em, &proxy);
}

/// DDC-2432
#[tokio::test]
async fn testFailedProxyCloningDoesNotMarkTheProxyAsInitialized() {
    let t = set_up().await;
    let proxy = t
        .proxy_factory
        .get_proxy(ECOMMERCE_FEATURE, &identifier())
        .unwrap();

    // $cloned = clone $proxy; $cloned->__load();
    let error = t.em.initialize_object(&proxy).await.unwrap_err();
    assert_eq!(ENTITY_NOT_FOUND_EXCEPTION, error.class);

    assert_uninitialized_lazy_object(&t.em, &proxy);
}

#[tokio::test]
async fn testProxyFactoryAcceptsNullProxyArgsWhenNativeLazyObjectsAreEnabled() {
    let t = set_up().await;
    let proxy_factory = ProxyFactory::with_proxy_args(&t.em, None, None, 0).unwrap();
    let proxy = proxy_factory
        .get_proxy(ECOMMERCE_FEATURE, &identifier())
        .unwrap();

    assert_uninitialized_lazy_object(&t.em, &proxy);
}

#[test]
fn testProxyFactoryThrowsIfLazyGhostsAreUnavailable() {
    let e = orm::entity_manager_try_with(orm::connection(), |configuration| {
        let _ = configuration.enable_native_lazy_objects(false);
    })
    .unwrap_err();

    assert_eq!(
        e.class,
        php2rust_doctrine_orm::exception::ORM_INVALID_ARGUMENT_EXCEPTION
    );
    assert_eq!(
        String::from_utf8_lossy(e.message.as_bytes()),
        "Symfony LazyGhost is not available. Please install the \"symfony/var-exporter\" package version 6.4 or 7 to use this feature or enable PHP 8.4 native lazy objects."
    );
}
