//! Port of `DetachedEntityTest`, on `SQLite` with the `cms` model set.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/DetachedEntityTest.php

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

use php_value::Value;
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;

use orm::{CMS_MODELS, CMS_PHONENUMBER, CMS_USER, get, new_object, obj, s, set};

#[tokio::test]
async fn testDetachedEntityThrowsExceptionOnFlush() {
    let em = orm::set_up(&CMS_MODELS).await;
    let ph = new_object(CMS_PHONENUMBER);
    set(&ph, "phonenumber", s("12345"));

    em.persist(&ph).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    em.persist(&ph).unwrap();

    // since it tries to insert the object twice (with the same PK)
    let e = em.flush().await.unwrap_err();
    assert!(
        thrown_is_a(
            &e,
            "Doctrine\\DBAL\\Exception\\UniqueConstraintViolationException"
        ) || e.class == "Doctrine\\DBAL\\Exception\\UniqueConstraintViolationException",
        "{e}"
    );
}

#[tokio::test]
async fn testUseDetachedEntityAsQueryParameter() {
    let em = orm::set_up(&CMS_MODELS).await;
    let user = new_object(CMS_USER);
    set(&user, "name", s("Guilherme"));
    set(&user, "username", s("gblanco"));
    set(&user, "status", s("developer"));

    em.persist(&user).unwrap();

    em.flush().await.unwrap();
    em.detach(&user).unwrap();

    let dql = format!("SELECT u FROM {CMS_USER} u WHERE u.id = ?1");
    let query = em.create_query(&dql);
    query.set_parameter(1, Value::Object(user.clone()), None);

    let new_user = obj(&query.get_single_result(None).await.unwrap());

    assert!(new_user.instance_of(CMS_USER));
    orm::assert_loose_eq(&s("gblanco"), &get(&new_user, "username"));
}

#[tokio::test]
async fn testDetachManagedUnpersistedEntity() {
    let em = orm::set_up(&CMS_MODELS).await;
    let user = new_object(CMS_USER);
    set(&user, "name", s("Guilherme"));
    set(&user, "username", s("gblanco"));
    set(&user, "status", s("developer"));

    em.persist(&user).unwrap();
    em.detach(&user).unwrap();

    em.flush().await.unwrap();

    assert!(!em.contains(&user).unwrap());
    assert!(!em.get_unit_of_work().is_in_identity_map(&user).unwrap());
}
