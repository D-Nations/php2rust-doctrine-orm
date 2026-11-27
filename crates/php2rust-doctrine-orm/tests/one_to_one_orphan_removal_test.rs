//! Port of `OneToOneOrphanRemovalTest`, on `SQLite` with the `cms` model
//! set.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OneToOneOrphanRemovalTest.php

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
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;

use orm::{
    CMS_MODELS, CMS_USER, cms_address, cms_email, cms_user, get, list, s, set, set_address,
    set_email,
};

#[tokio::test]
async fn testOrphanRemoval() {
    let em = orm::set_up(&CMS_MODELS).await;
    let user = cms_user();
    set(&user, "status", s("dev"));
    set(&user, "username", s("romanb"));
    set(&user, "name", s("Roman B."));

    let address = cms_address("de", "1234", "Berlin");
    set(&address, "zip", Value::Int(1234));

    set_address(&user, &address);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let user_id = get(&user, "id");

    em.clear().unwrap();

    let user_proxy = em.get_reference(CMS_USER, user_id).await.unwrap().unwrap();

    em.remove(&user_proxy).await.unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query("SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u");
    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(
        0,
        result.len(),
        "CmsUser should be removed by EntityManager"
    );

    let query = em.create_query("SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsAddress a");
    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(
        0,
        result.len(),
        "CmsAddress should be removed by orphanRemoval"
    );
}

#[tokio::test]
async fn testOrphanRemovalWhenUnlink() {
    let em = orm::set_up(&CMS_MODELS).await;
    let user = cms_user();
    set(&user, "status", s("dev"));
    set(&user, "username", s("beberlei"));
    set(&user, "name", s("Benjamin Eberlei"));

    let email = cms_email("beberlei@domain.com");

    set_email(&user, &email);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let user_id = get(&user, "id");

    em.clear().unwrap();

    let user = em.find(CMS_USER, user_id).await.unwrap().unwrap();

    // $user->setEmail(null)
    if !get(&user, "email").is_null() {
        set(&user, "email", Value::Null);
    }

    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query("SELECT e FROM Doctrine\\Tests\\Models\\CMS\\CmsEmail e");
    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(
        0,
        result.len(),
        "CmsEmail should be removed by orphanRemoval"
    );
}
