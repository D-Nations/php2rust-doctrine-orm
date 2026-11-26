//! Ticket/DDC767Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC767Test.php

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

use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

async fn in_transaction(
    em: &EntityManager,
    p_user: &php_value::Object,
    groups: &[php_value::Value],
) -> Result<(), php_value::Thrown> {
    em.begin_transaction().await?;

    coll::clear(&get(p_user, "groups")).await?;

    em.flush().await?;

    // Add new
    for group_id in groups {
        let group = em.find(CMS_GROUP, group_id.clone()).await?.unwrap();
        add_group(p_user, &group);
    }

    em.flush().await?;
    em.commit().await
}

#[tokio::test]
async fn testCollectionChangesInsideTransaction() {
    let em = orm::set_up(&CMS_MODELS).await;

    let user = cms_user_with("beberlei", "beberlei", "active");

    let group1 = cms_group("foo");
    let group2 = cms_group("bar");
    let group3 = cms_group("baz");

    add_group(&user, &group1);
    add_group(&user, &group2);

    em.persist(&user).unwrap();
    em.persist(&group1).unwrap();
    em.persist(&group2).unwrap();
    em.persist(&group3).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let p_user = em
        .find(CMS_USER, get(&user, "id"))
        .await
        .unwrap()
        .expect("User not retrieved from database.");

    let group_ids = [get(&group2, "id"), get(&group3, "id")];

    if in_transaction(&em, &p_user, &group_ids).await.is_err() {
        em.rollback().await.unwrap();
    }
}
