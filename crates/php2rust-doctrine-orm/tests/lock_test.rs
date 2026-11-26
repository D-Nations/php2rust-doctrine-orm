//! Locking/LockTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models, statements logged by `query_log.rs`).
//!
//! PHP's `null` lock mode of `find()` and `refresh()` is `None` passed to
//! `find_with_lock()` and `refresh_with_lock()`; their default
//! (`LockMode::NONE`) is `find()` and `refresh()`. The pessimistic lock
//! tests that upstream skips on `SQLite` ("Database Driver has no Write
//! Lock support.") are not ported.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Locking/LockTest.php

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

use php_value::{Object, Value};
use php2rust_doctrine_dbal::LockMode;
use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::context::{HINT_LOCK_MODE, Hint};

use orm::{CMS_ARTICLE, CMS_MODELS, CMS_USER, cms_article, get, new_object, s, same, set};

const OPTIMISTIC_LOCK_EXCEPTION: &str = "Doctrine\\ORM\\OptimisticLockException";
const TRANSACTION_REQUIRED_EXCEPTION: &str = "Doctrine\\ORM\\TransactionRequiredException";
/// `ORMInvalidArgumentException extends InvalidArgumentException`.
const ORM_INVALID_ARGUMENT_EXCEPTION: &str = "Doctrine\\ORM\\ORMInvalidArgumentException";
const NULL_LOCK_MODE_DEPRECATION: &str = "https://github.com/doctrine/orm/pull/12548";

async fn set_up() -> (Arc<EntityManager>, Arc<query_log::QueryLog>) {
    query_log::set_up_logged(&CMS_MODELS).await
}

/// `$article = new CmsArticle(); ...; persist; flush`.
async fn persisted_article(em: &EntityManager) -> Object {
    let article = cms_article("Hello", "my article");

    em.persist(&article).unwrap();
    em.flush().await.unwrap();

    article
}

#[tokio::test]
async fn testLockVersionedEntity() {
    let (em, _) = set_up().await;
    let article = persisted_article(&em).await;

    em.lock(
        &article,
        LockMode::Optimistic,
        Some(&get(&article, "version")),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn testLockVersionedEntityMismatchThrowsException() {
    let (em, _) = set_up().await;
    let article = persisted_article(&em).await;

    let Value::Int(version) = get(&article, "version") else {
        panic!("an int version");
    };
    let e = em
        .lock(
            &article,
            LockMode::Optimistic,
            Some(&Value::Int(version + 1)),
        )
        .await
        .unwrap_err();
    assert_eq!(e.class, OPTIMISTIC_LOCK_EXCEPTION, "{e:?}");
}

#[tokio::test]
async fn testLockUnversionedEntityThrowsException() {
    let (em, _) = set_up().await;
    let user = new_object(CMS_USER);
    set(&user, "name", s("foo"));
    set(&user, "status", s("active"));
    set(&user, "username", s("foo"));

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let e = em
        .lock(&user, LockMode::Optimistic, None)
        .await
        .unwrap_err();
    assert_eq!(e.class, OPTIMISTIC_LOCK_EXCEPTION, "{e:?}");
}

#[tokio::test]
async fn testLockUnmanagedEntityThrowsException() {
    let (em, _) = set_up().await;
    let article = new_object(CMS_ARTICLE);

    // `$article->version + 1`: null + 1.
    let e = em
        .lock(&article, LockMode::Optimistic, Some(&Value::Int(1)))
        .await
        .unwrap_err();
    assert_eq!(e.class, ORM_INVALID_ARGUMENT_EXCEPTION, "{e:?}");
    let message = String::from_utf8_lossy(e.message.as_bytes()).into_owned();
    assert!(
        message.contains(&format!("Entity {CMS_ARTICLE}")),
        "{message}"
    );
}

#[tokio::test]
async fn testLockWithLockModeNoneDoesNotRequireTransaction() {
    let (em, log) = set_up().await;
    let article = persisted_article(&em).await;

    log.reset_and_enable();

    em.lock(&article, LockMode::None, None).await.unwrap();

    assert_eq!(log.queries().len(), 0, "{:?}", log.queries());
}

#[tokio::test]
async fn testFindWithLockModeNoneDoesNotRefreshManagedEntity() {
    let (em, log) = set_up().await;
    let article = persisted_article(&em).await;

    set(&article, "topic", s("Changed in memory"));

    log.reset_and_enable();

    let found = em
        .find_with_lock(CMS_ARTICLE, get(&article, "id"), LockMode::None, None)
        .await
        .unwrap()
        .unwrap();

    assert!(same(&article, &found));
    assert_eq!(get(&found, "topic"), s("Changed in memory"));
    assert_eq!(log.queries().len(), 0, "{:?}", log.queries());
}

#[tokio::test]
async fn testFindWithNullLockModeIsDeprecated() {
    let (em, _) = set_up().await;
    let article = persisted_article(&em).await;

    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(NULL_LOCK_MODE_DEPRECATION);

    em.find_with_lock(CMS_ARTICLE, get(&article, "id"), None, None)
        .await
        .unwrap();

    assert!(deprecations::count(NULL_LOCK_MODE_DEPRECATION) > before);
}

#[tokio::test]
async fn testRefreshWithNullLockModeIsDeprecated() {
    let (em, _) = set_up().await;
    let article = persisted_article(&em).await;

    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(NULL_LOCK_MODE_DEPRECATION);

    em.refresh_with_lock(&article, None).await.unwrap();

    assert!(deprecations::count(NULL_LOCK_MODE_DEPRECATION) > before);
}

#[tokio::test]
async fn testFindWithoutLockModeIsNotDeprecated() {
    let (em, _) = set_up().await;
    let article = persisted_article(&em).await;

    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(NULL_LOCK_MODE_DEPRECATION);

    em.find(CMS_ARTICLE, get(&article, "id")).await.unwrap();
    em.refresh(&article).await.unwrap();

    assert_eq!(deprecations::count(NULL_LOCK_MODE_DEPRECATION), before);
}

#[tokio::test]
async fn testLockPessimisticReadNoTransactionThrowsException() {
    let (em, _) = set_up().await;
    let article = persisted_article(&em).await;

    let e = em
        .lock(&article, LockMode::PessimisticRead, None)
        .await
        .unwrap_err();
    assert_eq!(e.class, TRANSACTION_REQUIRED_EXCEPTION, "{e:?}");
}

#[tokio::test]
async fn testLockPessimisticWriteNoTransactionThrowsException() {
    let (em, _) = set_up().await;
    let article = persisted_article(&em).await;

    let e = em
        .lock(&article, LockMode::PessimisticWrite, None)
        .await
        .unwrap_err();
    assert_eq!(e.class, TRANSACTION_REQUIRED_EXCEPTION, "{e:?}");
}

#[tokio::test]
async fn testRefreshWithLockPessimisticWriteNoTransactionThrowsException() {
    let (em, _) = set_up().await;
    let article = persisted_article(&em).await;

    let e = em
        .refresh_with_lock(&article, LockMode::PessimisticWrite)
        .await
        .unwrap_err();
    assert_eq!(e.class, TRANSACTION_REQUIRED_EXCEPTION, "{e:?}");
}

#[tokio::test]
async fn testLockOptimisticNonVersionedThrowsExceptionInDQL() {
    let (em, _) = set_up().await;
    let dql = format!("SELECT u FROM {CMS_USER} u WHERE u.username = 'gblanco'");

    let query = em.create_query(&dql);
    query.set_hint(HINT_LOCK_MODE, Hint::LockMode(LockMode::Optimistic));
    let e = query.get_sql().unwrap_err();

    assert_eq!(e.class, OPTIMISTIC_LOCK_EXCEPTION, "{e:?}");
    let message = String::from_utf8_lossy(e.message.as_bytes()).into_owned();
    assert!(
        message.contains("The optimistic lock on an entity failed."),
        "{message}"
    );
}
