//! Ticket/DDC1043Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1043Test.php

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

use orm::*;

#[tokio::test]
async fn testChangeSetPlusWeirdPHPCastingIntCastingRule() {
    let em = orm::set_up(&CMS_MODELS).await;

    let user = cms_user_with("John Galt", "jgalt", "+44");

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    set(&user, "status", s("44"));
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    assert_eq!(s("44"), get(&user, "status"));
}
