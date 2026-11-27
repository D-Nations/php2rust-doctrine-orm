//! Upstream: doctrine/orm tests/Tests/ORM/Functional/UnitOfWorkLifecycleTest.php
//!
//! On `SQLite` with the `cms` model set (`tests/common/orm.rs`).

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

use std::sync::Arc;

use php_value::{PhpArray, Thrown};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::exception::ORM_INVALID_ARGUMENT_EXCEPTION;

use orm::{CMS_MODELS, cms_user, cms_user_with};

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&CMS_MODELS).await
}

#[track_caller]
fn expect_exception(result: Result<(), Thrown>, message: &str) {
    let error = result.unwrap_err();
    assert_eq!(ORM_INVALID_ARGUMENT_EXCEPTION, error.class);
    let actual = orm::message(&error);
    assert!(actual.contains(message), "{actual}");
}

#[tokio::test]
async fn testScheduleInsertManaged() {
    let em = set_up().await;
    let user = cms_user_with("Benjamin", "beberlei", "active");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    expect_exception(
        em.get_unit_of_work().schedule_for_insert(&user),
        "A managed+dirty entity Doctrine\\Tests\\Models\\CMS\\CmsUser",
    );
}

#[tokio::test]
async fn testScheduleInsertDeleted() {
    let em = set_up().await;
    let user = cms_user_with("Benjamin", "beberlei", "active");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    em.remove(&user).await.unwrap();

    expect_exception(
        em.get_unit_of_work().schedule_for_insert(&user),
        "Removed entity Doctrine\\Tests\\Models\\CMS\\CmsUser",
    );
}

#[tokio::test]
async fn testScheduleInsertTwice() {
    let em = set_up().await;
    let user = cms_user_with("Benjamin", "beberlei", "active");

    em.get_unit_of_work().schedule_for_insert(&user).unwrap();

    expect_exception(
        em.get_unit_of_work().schedule_for_insert(&user),
        "Entity Doctrine\\Tests\\Models\\CMS\\CmsUser",
    );
}

#[tokio::test]
async fn testAddToIdentityMapWithoutIdentity() {
    let em = set_up().await;
    let user = cms_user();

    expect_exception(
        em.get_unit_of_work()
            .register_managed(&user, PhpArray::new(), PhpArray::new()),
        "The given entity of type 'Doctrine\\Tests\\Models\\CMS\\CmsUser' (Doctrine\\Tests\\Models\\CMS\\CmsUser@",
    );
}

#[tokio::test]
async fn testMarkReadOnlyNonManaged() {
    let em = set_up().await;
    let user = cms_user();

    expect_exception(
        em.get_unit_of_work().mark_read_only(&user),
        "Only managed entities can be marked or checked as read only. But Doctrine\\Tests\\Models\\CMS\\CmsUser@",
    );
}
